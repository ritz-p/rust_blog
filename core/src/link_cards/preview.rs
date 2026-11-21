use reqwest::{Client, header, redirect::Policy};
use rocket::tokio::{net::lookup_host, time::timeout};
use scraper::{Html, Selector};
use std::{
    net::{IpAddr, SocketAddr},
    sync::LazyLock,
    time::Duration,
};
use url::{Host, Url};

#[derive(Clone)]
pub(super) struct Preview {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<Url>,
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
            "og:image" if image.is_none() && content.len() <= 8192 => {
                image = base.join(content).ok().filter(web_url);
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
        let mime = response
            .headers()
            .get(header::CONTENT_TYPE)?
            .to_str()
            .ok()?
            .split(';')
            .next()?
            .trim();
        if !response.status().is_success()
            || !(mime.eq_ignore_ascii_case("text/html")
                || mime.eq_ignore_ascii_case("application/xhtml+xml"))
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
        return metadata(&String::from_utf8_lossy(&body), &url);
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
    fn ignores_body_metadata_and_non_web_images() {
        let url = Url::parse("https://example.org/").unwrap();
        let preview = metadata(r#"<title>Page</title><meta property="og:image" content="data:text/plain,foo"><body><meta property="og:title" content="decoy">"#, &url).unwrap();
        assert_eq!(preview.title.as_deref(), Some("Page"));
        assert_eq!(preview.image, None);
        assert!(metadata("<p>No preview</p>", &url).is_none());
    }
}
