use std::{cell::RefCell, collections::HashSet};
use unicode_normalization::UnicodeNormalization;

use lol_html::{RewriteStrSettings, element, rewrite_str, text};

pub fn toc(input: &str) -> String {
    decorate(input, true)
}

pub(super) fn anchors(input: &str) -> String {
    decorate(input, false)
}

fn decorate(input: &str, with_toc: bool) -> String {
    let headings = RefCell::new(Vec::<(u8, String, String)>::new());
    let selector = "h1, h2, h3, h4, h5, h6";
    let result = rewrite_str(
        input,
        RewriteStrSettings::new()
            .append_element_content_handler(element!(selector, |el| {
                let mut headings = headings.borrow_mut();
                let id = format!("toc-heading-{}", headings.len() + 1);
                let level = el.tag_name().as_bytes()[1] - b'0';
                headings.push((level, id, String::new()));
                Ok(())
            }))
            .append_element_content_handler(text!(selector, |text| {
                if let Some(heading) = headings.borrow_mut().last_mut() {
                    heading.2.push_str(text.as_str());
                }
                Ok(())
            }))
            .append_element_content_handler(element!(
                "h1 br, h2 br, h3 br, h4 br, h5 br, h6 br",
                |_| {
                    if let Some(heading) = headings.borrow_mut().last_mut() {
                        heading.2.push(' ');
                    }
                    Ok(())
                }
            ))
            .append_element_content_handler(element!(
                "h1 img, h2 img, h3 img, h4 img, h5 img, h6 img",
                |el| {
                    if let Some(alt) = el.get_attribute("alt")
                        && let Some(heading) = headings.borrow_mut().last_mut()
                    {
                        heading.2.push_str(&alt);
                    }
                    Ok(())
                }
            )),
    );
    let Ok(_) = result else {
        return input.to_owned();
    };
    let mut headings = headings.into_inner();
    if headings.is_empty() {
        return input.to_owned();
    }
    let mut used = HashSet::new();
    for (_, id, label) in &mut headings {
        *label = html_escape::decode_html_entities(label).into_owned();
        let slug = label
            .nfc()
            .flat_map(char::to_lowercase)
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>();
        let slug = slug
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        let base = format!(
            "heading-{}",
            if slug.is_empty() { "section" } else { &slug }
        );
        *id = base.clone();
        let mut suffix = 2;
        while !used.insert(id.clone()) {
            *id = format!("{base}-{suffix}");
            suffix += 1;
        }
    }
    let mut index = 0;
    let content = rewrite_str(
        input,
        RewriteStrSettings::new().append_element_content_handler(element!(selector, |el| {
            el.set_attribute("id", &headings[index].1)?;
            index += 1;
            if with_toc {
                el.before(
                    &format!("<span id=\"toc-heading-{index}\"></span>"),
                    lol_html::html_content::ContentType::Html,
                );
            }
            Ok(())
        })),
    )
    .unwrap_or_else(|_| input.to_owned());
    if !with_toc {
        return content;
    }
    let mut output = format!(
        "<nav class=\"table-of-contents\" aria-label=\"目次\"><div class=\"toc-header\"><span class=\"toc-title\">目次</span><span class=\"toc-count\">{}項目</span></div>",
        headings.len()
    );
    render_toc(&headings, &mut 0, 0, &mut output);
    output.push_str("</nav>\n");
    output.push_str(&content);
    output
}

fn render_toc(headings: &[(u8, String, String)], index: &mut usize, parent: u8, html: &mut String) {
    html.push_str("<ol>");
    while *index < headings.len() && headings[*index].0 > parent {
        let (level, id, text) = &headings[*index];
        let escaped = html_escape::encode_safe(text);
        html.push_str(&format!("<li><a href=\"#{id}\">{escaped}</a>"));
        *index += 1;
        if *index < headings.len() && headings[*index].0 > *level {
            render_toc(headings, index, *level, html);
        }
        html.push_str("</li>");
    }
    html.push_str("</ol>");
}

#[cfg(test)]
mod tests {
    use super::toc;
    use crate::utils::markdown::markdown_to_html;

    #[test]
    fn toc_preserves_literal_entity_syntax() {
        for (markdown, label) in [
            ("## `&lt;`", "&amp;lt;"),
            ("## `&#60;`", "&amp;#60;"),
            ("## `&amp;lt;`", "&amp;amp;lt;"),
            ("## A & B", "A &amp; B"),
            ("## &lt;", "&lt;"),
        ] {
            let output = toc(&markdown_to_html(markdown));
            let nav = output.split_once("</nav>").unwrap().0;
            assert!(nav.contains(&format!(">{label}</a>")), "{markdown}: {nav}");
        }
    }

    #[test]
    fn toc_preserves_rendered_body_except_heading_ids() {
        let rendered = markdown_to_html(
            "## A & B\n\n```rust\nfn main() {}\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n<div><h3 title=\"a > b\">HTML &amp; heading</h3></div>",
        );
        let output = toc(&rendered);
        let (nav, body) = output.split_once("</nav>\n").unwrap();
        assert!(nav.contains(">A &amp; B</a>"));
        assert!(nav.contains(">HTML &amp; heading</a>"));
        assert!(!nav.contains("&amp;amp;"));
        assert_eq!(
            body.replace("<span id=\"toc-heading-1\"></span>", "")
                .replace("<span id=\"toc-heading-2\"></span>", ""),
            rendered
        );
    }

    #[test]
    fn heading_links_survive_unrelated_insertions_and_disambiguate_duplicates() {
        let original = markdown_to_html("## **日本語** & text\n\n## Same\n\n## Same\n\n## Same-2");
        let edited = markdown_to_html(
            "## New heading\n\n## **日本語** & text\n\n## Same\n\n## Same\n\n## Same-2",
        );
        for id in [
            "heading-日本語-text",
            "heading-same",
            "heading-same-2",
            "heading-same-2-2",
        ] {
            assert_eq!(original.matches(&format!("id=\"{id}\"")).count(), 1);
            assert_eq!(edited.matches(&format!("id=\"{id}\"")).count(), 1);
        }
    }
}
