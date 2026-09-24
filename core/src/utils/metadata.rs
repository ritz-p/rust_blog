use crate::{
    entity::article,
    utils::{config::CommonConfig, cut_out_string, markdown::markdown_to_text, url_segment},
};
use serde::Serialize;

#[derive(Serialize)]
pub struct ArticleMetadata {
    pub title: String,
    pub description: String,
    pub url: Option<String>,
    pub image: Option<String>,
}

pub fn article_metadata(
    article: &article::Model,
    config: &CommonConfig,
    is_static: bool,
) -> ArticleMetadata {
    let base = config
        .public_url
        .as_deref()
        .map(str::trim)
        .filter(|url| url.starts_with("https://") || url.starts_with("http://"));
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
        .filter(|s| !s.is_empty())
        .or(config.default_icatch_path.as_deref())
        .filter(|s| !s.is_empty())
        .and_then(|path| {
            if path.starts_with("https://") || path.starts_with("http://") {
                Some(path.to_owned())
            } else {
                base.map(|base| {
                    format!(
                        "{}/{}",
                        base.trim_end_matches('/'),
                        path.trim_start_matches('/')
                    )
                })
            }
        });
    ArticleMetadata {
        title: article.title.clone(),
        description: cut_out_string(&description, 160),
        url: base.map(|base| {
            format!(
                "{}/posts/{}{}",
                base.trim_end_matches('/'),
                url_segment(&article.slug),
                if is_static { "/" } else { "" }
            )
        }),
        image,
    }
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
        };
        let mut config = CommonConfig {
            articles_per_page: 10,
            site_name: None,
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
                include_str!("../../../templates/partial/base.html.tera"),
                &context,
                autoescape,
            )
            .unwrap();
            assert!(html.contains("A &quot;title&quot; &lt;&amp;&gt;"), "{html}");
            assert!(!html.contains("&amp;quot;"));
        }
        assert_eq!(metadata.description, "Summary link");
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
