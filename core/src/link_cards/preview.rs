use encoding_rs::{Encoding, UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252, X_USER_DEFINED};
use mime::Mime;
use reqwest::{Client, header, redirect::Policy};
use rocket::tokio::{net::lookup_host, time::timeout};
use scraper::{Html, Selector};
use std::{
    borrow::Cow,
    net::{IpAddr, SocketAddr},
    sync::LazyLock,
    time::Duration,
};
use url::{Host, Url};

#[derive(Clone, serde::Deserialize, serde::Serialize)]
pub(super) struct Preview {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<Url>,
}

static CHARSET_META: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("meta[charset], meta[http-equiv][content]").unwrap());

fn mime_encoding(content_type: &Mime) -> Option<&'static Encoding> {
    Encoding::for_label_no_replacement(content_type.get_param(mime::CHARSET)?.as_str().as_bytes())
}

fn html_meta_encoding(body: &[u8]) -> Option<&'static Encoding> {
    let prefix = &body[..body.len().min(1024)];
    let document = Html::parse_document(&String::from_utf8_lossy(prefix));
    let encoding = document.select(&CHARSET_META).find_map(|meta| {
        if let Some(label) = meta.attr("charset") {
            return Encoding::for_label_no_replacement(label.as_bytes());
        }
        if !meta
            .attr("http-equiv")?
            .eq_ignore_ascii_case("content-type")
        {
            return None;
        }
        mime_encoding(&meta.attr("content")?.parse().ok()?)
    })?;
    if encoding == UTF_16LE || encoding == UTF_16BE {
        Some(UTF_8)
    } else if encoding == X_USER_DEFINED {
        Some(WINDOWS_1252)
    } else {
        Some(encoding)
    }
}

fn decode_html<'a>(body: &'a [u8], content_type: &Mime) -> Cow<'a, str> {
    let (encoding, skip) = Encoding::for_bom(body).unwrap_or_else(|| {
        let encoding = mime_encoding(content_type)
            .or_else(|| html_meta_encoding(body))
            .unwrap_or(UTF_8);
        (encoding, 0)
    });
    encoding.decode_without_bom_handling(&body[skip..]).0
}

static HEAD: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("head title, head meta[content]").unwrap());

pub(super) fn metadata(html: &str, base: &Url) -> Option<Preview> {
    let document = Html::parse_document(html);
    let mut title = None;
    let mut og_title = None;
    let mut description = None;
    let mut og_description = None;
    let mut image = None;
    for node in document.select(&HEAD) {
        let value = node.value();
        if value.name() == "title" {
            let text = node.text().collect::<String>();
            if !text.trim().is_empty() {
                title.get_or_insert_with(|| text.trim().chars().take(256).collect());
            }
            continue;
        }
        let content = value.attr("content")?.trim();
        if content.is_empty() {
            continue;
        }
        match value
            .attr("property")
            .or_else(|| value.attr("name"))
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str()
        {
            "og:title" => {
                og_title.get_or_insert_with(|| content.chars().take(256).collect());
            }
            "og:description" => {
                og_description.get_or_insert_with(|| content.chars().take(512).collect());
            }
            "description" => {
                description.get_or_insert_with(|| content.chars().take(512).collect());
            }
            "og:image" | "og:image:secure_url" if image.is_none() && content.len() <= 8192 => {
                image = base
                    .join(content)
                    .ok()
                    .filter(|url| url.scheme() == "https" && web_url(url));
            }
            _ => {}
        }
    }
    let title = og_title.or(title);
    let description = og_description.or(description);
    (title.is_some() || description.is_some() || image.is_some()).then_some(Preview {
        title,
        description,
        image,
    })
}

fn web_url(url: &Url) -> bool {
    matches!(url.scheme(), "http" | "https")
        && url.username().is_empty()
        && url.password().is_none()
}

fn public_address(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(v4) => !matches!(
            v4.octets(),
            [0 | 10 | 127 | 224..=255, ..]
                | [100, 64..=127, ..]
                | [169, 254, ..]
                | [172, 16..=31, ..]
                | [192, 0, 0 | 2, _]
                | [192, 88, 99, _]
                | [192, 168, ..]
                | [198, 18 | 19, ..]
                | [198, 51, 100, _]
                | [203, 0, 113, _]
        ),
        IpAddr::V6(v6) => {
            matches!(v6.segments(), [0x2000..=0x3fff, ..])
                && !matches!(
                    v6.segments(),
                    [0x2001, 0..=0x1ff | 0xdb8, ..] | [0x2002, ..] | [0x3fff, 0..=0xfff, ..]
                )
        }
    }
}

pub(super) async fn fetch(url: Url) -> Option<Preview> {
    timeout(Duration::from_secs(4), fetch_page(url))
        .await
        .ok()
        .flatten()
}

async fn fetch_page(mut url: Url) -> Option<Preview> {
    const HEAD_LIMIT: usize = 256 * 1024;
    for _ in 0..4 {
        if !web_url(&url) {
            return None;
        }
        let port = url.port_or_known_default()?;
        let addrs: Vec<SocketAddr> = match url.host()? {
            Host::Domain(host) => lookup_host((host, port)).await.ok()?.collect(),
            Host::Ipv4(ip) => vec![SocketAddr::new(ip.into(), port)],
            Host::Ipv6(ip) => vec![SocketAddr::new(ip.into(), port)],
        };
        if addrs.is_empty() || !addrs.iter().all(|addr| public_address(addr.ip())) {
            return None;
        }
        // Pin the checked addresses for this hop, including after every redirect.
        let client = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .resolve_to_addrs(url.host_str()?, &addrs)
            .build()
            .ok()?;
        let mut response = client
            .get(url.clone())
            .header(header::ACCEPT, "text/html, application/xhtml+xml")
            .header(header::USER_AGENT, "rust_blog/link-preview")
            .send()
            .await
            .ok()?;
        if response.status().is_redirection() {
            url = url
                .join(response.headers().get(header::LOCATION)?.to_str().ok()?)
                .ok()?;
            continue;
        }
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)?
            .to_str()
            .ok()?;
        let mime: Mime = content_type.trim().parse().ok().or_else(|| {
            let (essence, _) = content_type.split_once(';')?;
            essence.trim().parse().ok()
        })?;
        if !response.status().is_success()
            || !matches!(mime.essence_str(), "text/html" | "application/xhtml+xml")
        {
            return None;
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.ok()? {
            let scan_from = body.len().saturating_sub(b"</head>".len() - 1);
            let len = chunk.len().min(HEAD_LIMIT - body.len());
            body.extend_from_slice(&chunk[..len]);
            if body.len() == HEAD_LIMIT
                || body[scan_from..]
                    .windows(b"</head>".len())
                    .any(|s| s.eq_ignore_ascii_case(b"</head>"))
            {
                break;
            }
        }
        return metadata(&decode_html(&body, &mime), &url);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fetches_only_public_addresses() {
        for ip in [
            "127.0.0.1",
            "::ffff:127.0.0.1",
            "10.0.0.1",
            "::1",
            "fd00::1",
            "169.254.169.254",
            "64:ff9b::7f00:1",
            "224.0.0.1",
        ] {
            assert!(!public_address(ip.parse().unwrap()), "{ip}");
        }
        for ip in ["1.1.1.1", "2606:4700:4700::1111"] {
            assert!(public_address(ip.parse().unwrap()), "{ip}");
        }
    }

    #[test]
    fn decodes_titles_and_descriptions_using_the_response_or_html_charset() {
        use encoding_rs::{SHIFT_JIS, WINDOWS_1252};
        let url = Url::parse("https://example.org/").unwrap();
        for (http_type, declaration, encoding, label) in [
            (
                "text/html; charset=Shift_JIS",
                "<meta charset='utf-8'>",
                SHIFT_JIS,
                "日本語",
            ),
            (
                "text/html",
                "<meta charset='Shift_JIS'>",
                SHIFT_JIS,
                "日本語",
            ),
            (
                "text/html; charset=unknown",
                "<meta http-equiv='Content-Type' content='text/html; charset=windows-1252'>",
                WINDOWS_1252,
                "Café €",
            ),
        ] {
            let html = format!(
                r#"{declaration}<title>{label}</title><meta property="og:description" content="{label}">"#
            );
            let (encoded, _, _) = encoding.encode(&html);
            let decoded = decode_html(&encoded, &http_type.parse().unwrap());
            let preview = metadata(&decoded, &url).unwrap();
            assert_eq!(preview.title.as_deref(), Some(label));
            assert_eq!(preview.description.as_deref(), Some(label));
        }
    }

    #[test]
    fn bom_overrides_the_header_and_undeclared_utf8_is_preserved() {
        let bytes: Vec<_> = std::iter::once(0xfeff)
            .chain("日本語".encode_utf16())
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(
            decode_html(&bytes, &"text/html; charset=windows-1252".parse().unwrap()),
            "日本語"
        );
        assert_eq!(
            decode_html("日本語".as_bytes(), &"text/html".parse().unwrap()),
            "日本語"
        );
    }

    #[test]
    fn keeps_http_pages_linkable_but_omits_http_thumbnails() {
        let url = Url::parse("http://example.org/").unwrap();
        assert!(web_url(&url));
        for image in ["/thumb.jpg", "http://cdn.example.org/thumb.jpg"] {
            let html =
                format!(r#"<title>Page</title><meta property="og:image" content="{image}">"#);
            let preview = metadata(&html, &url).unwrap();
            assert_eq!(preview.title.as_deref(), Some("Page"));
            assert_eq!(preview.image, None);
        }
        let html = r#"<meta property="og:image" content="/thumb.jpg"><meta property="og:image:secure_url" content="https://cdn.example.org/thumb.jpg">"#;
        assert_eq!(
            metadata(html, &url).unwrap().image.unwrap().as_str(),
            "https://cdn.example.org/thumb.jpg"
        );
    }

    #[test]
    fn ignores_body_metadata_and_non_web_images() {
        let url = Url::parse("https://example.org/").unwrap();
        let preview = metadata(r#"<title>Page</title><meta property="og:image" content="data:text/plain,foo"><body><meta property="og:title" content="decoy">"#, &url).unwrap();
        assert_eq!(preview.title.as_deref(), Some("Page"));
        assert_eq!(preview.image, None);
        assert!(metadata("<p>No preview</p>", &url).is_none());
    }
}
