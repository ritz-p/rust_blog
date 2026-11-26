mod disk_cache;
mod preview;

use crate::utils::markdown::{markdown_parser, render_markdown};
use futures::{StreamExt, stream};
use html_escape::{encode_double_quoted_attribute, encode_text};
use moka::future::Cache;
use preview::Preview;
use pulldown_cmark::{Event, LinkType, Tag};
use std::{ops::Range, path::Path, sync::LazyLock, time::Duration};
use url::Url;

static PREVIEWS: LazyLock<Cache<Url, Option<Preview>>> = LazyLock::new(|| {
    Cache::builder()
        .max_capacity(1024)
        .time_to_live(Duration::from_secs(15 * 60))
        .build()
});

pub async fn article_html(markdown: &str) -> String {
    render_with(markdown, |mut url| async move {
        url.set_fragment(None);
        PREVIEWS.get_with(url.clone(), preview::fetch(url)).await
    })
    .await
}

pub(crate) struct ExportPreviews<'a> {
    directory: &'a Path,
    results: Cache<Url, Option<Preview>>,
}

impl<'a> ExportPreviews<'a> {
    pub(crate) fn new(directory: &'a Path) -> Self {
        Self {
            directory,
            results: Cache::builder().build(),
        }
    }

    async fn lookup<F, Fut>(&self, mut url: Url, fetch: F) -> Option<Preview>
    where
        F: FnOnce(Url) -> Fut,
        Fut: Future<Output = Option<Preview>>,
    {
        url.set_fragment(None);
        self.results
            .get_with(url.clone(), async {
                disk_cache::lookup(self.directory, url.clone(), || fetch(url)).await
            })
            .await
    }

    pub(crate) async fn article_html(&self, markdown: &str) -> String {
        render_with(markdown, |url| self.lookup(url, preview::fetch)).await
    }
}

async fn render_with<F, Fut>(markdown: &str, lookup: F) -> String
where
    F: Fn(Url) -> Fut,
    Fut: Future<Output = Option<Preview>>,
{
    let mut events: Vec<_> = markdown_parser(markdown).collect();
    let candidates = (0..events.len()).filter_map(|i| standalone_url(&events, i));
    let cards: Vec<_> = stream::iter(candidates)
        .map(|(range, text, url)| {
            let preview = lookup(url.clone());
            async move { (range, card_html(&text, &url, preview.await)) }
        })
        .buffered(4)
        .collect()
        .await;

    for (range, html) in cards {
        let first = range.start;
        events[range].fill(Event::Text("".into()));
        events[first] = Event::Html(html.into());
    }
    render_markdown(events.into_iter())
}

fn standalone_url(events: &[Event<'_>], start: usize) -> Option<(Range<usize>, String, Url)> {
    let Event::Start(container @ (Tag::Paragraph | Tag::Item)) = &events[start] else {
        return None;
    };
    let first = start + 1;
    let autolink = matches!(
        events.get(first),
        Some(Event::Start(Tag::Link(LinkType::Autolink, ..)))
    );
    let mut end = first + usize::from(autolink);
    let mut text = String::new();
    while let Some(Event::Text(part)) = events.get(end) {
        text.push_str(part);
        end += 1;
    }
    if autolink {
        if !matches!(
            events.get(end),
            Some(Event::End(Tag::Link(LinkType::Autolink, ..)))
        ) {
            return None;
        }
        end += 1;
    }
    if !matches!(events.get(end), Some(Event::End(tag)) if tag == container) {
        return None;
    }
    let text = text.trim();
    if !text
        .split_once("://")
        .is_some_and(|(s, _)| s.eq_ignore_ascii_case("http") || s.eq_ignore_ascii_case("https"))
        || text.contains(char::is_whitespace)
        || text.contains(char::is_control)
    {
        return None;
    }
    let url = Url::parse(text).ok()?;
    url.host()?;
    Some((first..end, text.to_owned(), url))
}

fn card_html(text: &str, url: &Url, preview: Option<Preview>) -> String {
    let href = encode_double_quoted_attribute(url.as_str());
    let Some(preview) = preview else {
        return format!(r#"<a href="{href}">{}</a>"#, encode_text(text));
    };
    let domain = url.host_str().unwrap_or_default();
    let title = encode_text(preview.title.as_deref().unwrap_or(domain));
    let description = preview
        .description
        .as_deref()
        .map(|v| {
            format!(
                r#"<span class="link-card-description">{}</span>"#,
                encode_text(v)
            )
        })
        .unwrap_or_default();
    let image = preview
        .image
        .as_ref()
        .map(|v| {
            format!(
                r#"<img src="{}" alt="">"#,
                encode_double_quoted_attribute(v.as_str())
            )
        })
        .unwrap_or_default();
    format!(
        r#"<span class="link-card"><a href="{href}">{image}<span class="link-card-body"><strong>{title}</strong>{description}<span class="link-card-domain">{}</span></span></a></span>"#,
        encode_text(domain)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use scraper::{Html, Selector};

    #[rocket::async_test]
    async fn failed_fetch_is_shared_within_one_export_and_retried_on_the_next() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let directory = tempfile::tempdir().unwrap();
        let calls = AtomicUsize::new(0);
        let fetch = |_| async {
            calls.fetch_add(1, Ordering::SeqCst);
            None
        };
        let url = Url::parse("https://example.org/").unwrap();
        let fragment = url.join("#section").unwrap();
        for run in 1..=2 {
            let previews = ExportPreviews::new(directory.path());
            let (first, second) = futures::join!(
                previews.lookup(url.clone(), fetch),
                previews.lookup(fragment.clone(), fetch)
            );
            assert!(first.is_none() && second.is_none());
            assert!(previews.lookup(url.clone(), fetch).await.is_none());
            assert_eq!(calls.load(Ordering::SeqCst), run);
        }
    }

    #[rocket::async_test]
    async fn previews_for_paragraphs_and_list_items_preserve_other_markdown() {
        let input = "https://example.org/with?q=1&v=2#section\n\n- https://example.org/without\n- <https://example.org/missing>\n\n1. https://example.org/loose\n\n2. other\n\nText https://example.org/inline\n\n[named](https://example.org/link)\n\n[![diagram](/image/d.svg)](https://example.org/image)\n\n`https://example.org/code`\n\n```\nhttps://example.org/fenced\n```";
        let output = render_with(input, |url| async move {
            let html = match url.path() {
                "/with" => r#"<title>Fallback</title><meta property="og:title" content="A &amp; &lt;B&gt;"><meta name="description" content="generic"><meta property="og:description" content="Preview"><meta property="og:image" content="/thumb.jpg">"#,
                "/without" | "/loose" => "<title>Text only</title>",
                "/missing" => return None,
                path => panic!("unexpected preview request for {path}"),
            };
            preview::metadata(html, &url)
        }).await;
        let dom = Html::parse_fragment(&output);
        let cards: Vec<_> = dom
            .select(&Selector::parse(".link-card > a").unwrap())
            .collect();
        assert_eq!(cards.len(), 3);
        assert_eq!(
            cards[0].attr("href"),
            Some("https://example.org/with?q=1&v=2#section")
        );
        assert_eq!(
            cards[0].text().collect::<String>(),
            "A & <B>Previewexample.org"
        );
        let imgs = Selector::parse("img").unwrap();
        assert_eq!(
            cards[0].select(&imgs).next().unwrap().attr("src"),
            Some("https://example.org/thumb.jpg")
        );
        assert_eq!(cards[1].select(&imgs).count(), 0);
        assert_eq!(cards[1].text().collect::<String>(), "Text onlyexample.org");
        let fallback = Selector::parse(r#"li > a[href="https://example.org/missing"]"#).unwrap();
        assert_eq!(
            dom.select(&fallback)
                .next()
                .unwrap()
                .text()
                .collect::<String>(),
            "https://example.org/missing"
        );
        assert!(output.contains("Text https://example.org/inline"));
        assert!(output.contains("<code>https://example.org/code</code>"));
        assert!(output.contains(">named</a>"));
        assert!(output.contains(r#"src="/image/d.svg""#));
    }
}
