use crate::{
    entity::article,
    utils::{config::CommonConfig, cut_out_string, markdown::markdown_to_text, url_segment},
};
use serde::Serialize;

#[derive(Serialize)]
pub struct PageMetadata {
    pub kind: &'static str,
    pub site_name: String,
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub image: Option<String>,
}

pub fn article_metadata(
    article: &article::Model,
    config: &CommonConfig,
    is_static: bool,
) -> PageMetadata {
    let origin = site_origin(config);
    let description = markdown_to_text(
        article
            .excerpt
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&article.content),
    );
    let description = description.split_whitespace().collect::<Vec<_>>().join(" ");
    let image = article
        .icatch_path
        .as_deref()
        .and_then(|path| image_url(path, origin.as_ref()))
        .or_else(|| default_image(config, origin.as_ref()));
    PageMetadata {
        kind: "article",
        site_name: site_name(config).to_owned(),
        title: article.title.clone(),
        description: Some(cut_out_string(&description, 160)),
        url: origin.map(|base| {
            format!(
                "{}/posts/{}{}",
                base.as_str().trim_end_matches('/'),
                url_segment(&article.slug),
                if is_static { "/" } else { "" }
            )
        }),
        image,
    }
}

pub fn site_metadata(config: &CommonConfig) -> PageMetadata {
    let origin = site_origin(config);
    PageMetadata {
        kind: "website",
        title: site_name(config).to_owned(),
        site_name: site_name(config).to_owned(),
        description: config
            .description
            .as_deref()
            .map(str::trim)
            .filter(|description| !description.is_empty())
            .map(str::to_owned),
        image: default_image(config, origin.as_ref()),
        url: origin.map(|url| url.to_string()),
    }
}

fn site_name(config: &CommonConfig) -> &str {
    config
        .site_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or("My Blog")
}

fn site_origin(config: &CommonConfig) -> Option<url::Url> {
    crate::discovery::site_origin(config.public_url.as_deref())
        .ok()
        .flatten()
        .and_then(|origin| url::Url::parse(&origin).ok())
}

fn default_image(config: &CommonConfig, origin: Option<&url::Url>) -> Option<String> {
    image_url(config.default_icatch_path.as_deref()?, origin)
}

fn image_url(path: &str, origin: Option<&url::Url>) -> Option<String> {
    let path = path.trim();
    if path.is_empty() {
        return None;
    }
    let url = match origin {
        Some(base) => base.join(path),
        None => url::Url::parse(path),
    }
    .ok()?;
    (matches!(url.scheme(), "http" | "https")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none())
    .then(|| url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_uses_plain_text_encoded_urls_and_image_fallback() {
        let mut article = article::Model {
            id: 1,
            title: "A \"title\" <&>".into(),
            slug: "日本語 #1".into(),
            excerpt: Some("**Summary**\n\n[link](https://example.org)".into()),
            content: "本文".repeat(200),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            icatch_path: None,
            table_of_contents: false,
            deprecated: false,
            adsense_sidebar: false,
            adsense_auto_ads: false,
            section: "blog".into(),
        };
        let mut config = CommonConfig {
            articles_per_page: 10,
            site_name: None,
            description: None,
            timezone: chrono_tz::UTC,
            default_icatch_path: Some("/image/default.png".into()),
            favicon_path: None,
            public_url: Some("https://example.com/".into()),
        };
        let metadata = article_metadata(&article, &config, false);
        for autoescape in [false, true] {
            let context = tera::Context::from_serialize(serde_json::json!({
                "metadata": &metadata, "site_name": "Test", "favicon_path": ""
            }))
            .unwrap();
            let html = tera::Tera::one_off(
                include_str!("../../../templates/partial/share_metadata.html.tera"),
                &context,
                autoescape,
            )
            .unwrap();
            assert!(html.contains("A &quot;title&quot; &lt;&amp;&gt;"), "{html}");
            assert!(!html.contains("&amp;quot;"));
        }
        assert_eq!(metadata.description.as_deref(), Some("Summary link"));
        assert_eq!(
            metadata.url.as_deref(),
            Some("https://example.com/posts/%E6%97%A5%E6%9C%AC%E8%AA%9E%20%231")
        );
        assert_eq!(
            metadata.image.as_deref(),
            Some("https://example.com/image/default.png")
        );
        assert!(
            article_metadata(&article, &config, true)
                .url
                .unwrap()
                .ends_with('/')
        );
        article.excerpt = None;
        assert_eq!(
            article_metadata(&article, &config, false)
                .description
                .unwrap()
                .chars()
                .count(),
            160
        );
        config.public_url = None;
        let metadata = article_metadata(&article, &config, false);
        assert!(metadata.url.is_none());
        assert!(metadata.image.is_none());
        article.icatch_path = Some("https://cdn.example.com/image.png".into());
        assert_eq!(
            article_metadata(&article, &config, false).image,
            article.icatch_path
        );
    }
}
