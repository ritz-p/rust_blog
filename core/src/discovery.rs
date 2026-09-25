use anyhow::{Result, ensure};
use sea_orm::DatabaseConnection;

pub fn site_origin(value: Option<&str>) -> Result<Option<String>> {
    let Some(value) = value.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    let url = url::Url::parse(value)?;
    ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.path() == "/",
        "public_url must be an HTTP(S) site origin without credentials, path, query or fragment"
    );
    Ok(Some(url.as_str().trim_end_matches('/').to_string()))
}

pub fn robots(origin: Option<&str>) -> String {
    let mut text = String::from("User-agent: *\nAllow: /\n");
    if let Some(origin) = origin {
        text.push_str(&format!("Sitemap: {origin}/sitemap.xml\n"));
    }
    text
}

pub async fn sitemap(db: &DatabaseConnection, origin: &str, is_static: bool) -> Result<String> {
    use crate::repository::{
        article::get_all_published_articles, category::get_all_categories,
        fixed_content::get_all_fixed_contents, tag::get_all_tags,
    };
    let mut entries = std::collections::BTreeMap::new();
    entries.insert("/".to_string(), None);
    let suffix = if is_static { "/" } else { "" };
    for name in ["tags", "categories"] {
        entries.insert(format!("/{name}{suffix}"), None);
    }
    for article in get_all_published_articles(db).await? {
        entries.insert(
            format!(
                "/posts/{}{suffix}",
                crate::utils::url_segment(&article.slug)
            ),
            Some(article.updated_at),
        );
    }
    for page in get_all_fixed_contents(db).await? {
        entries.insert(
            format!("/{}{suffix}", crate::utils::url_segment(&page.slug)),
            Some(page.updated_at),
        );
    }
    for tag in get_all_tags(db).await? {
        entries.insert(
            format!("/tag/{}{suffix}", crate::utils::url_segment(&tag.slug)),
            None,
        );
    }
    for category in get_all_categories(db).await? {
        entries.insert(
            format!(
                "/category/{}{suffix}",
                crate::utils::url_segment(&category.slug)
            ),
            None,
        );
    }
    ensure!(
        entries.len() <= 50_000,
        "sitemap exceeds 50,000 URLs; split into multiple sitemaps"
    );
    let mut xml = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for (path, modified) in entries {
        let absolute = format!("{origin}{path}");
        xml.push_str(&format!(
            "  <url><loc>{}</loc>",
            html_escape::encode_text(&absolute)
        ));
        if let Some(modified) = modified {
            xml.push_str(&format!(
                "<lastmod>{}</lastmod>",
                modified.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            ));
        }
        xml.push_str("</url>\n");
    }
    xml.push_str("</urlset>\n");
    ensure!(xml.len() <= 50 * 1024 * 1024, "sitemap exceeds 50 MB");
    Ok(xml)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rocket::async_test]
    async fn sitemap_uses_published_content_encoded_urls_and_stored_lastmod() {
        use crate::entity::{article, article_category, article_tag, category, fixed_content, tag};
        use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, Schema, Set};
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        for table in [
            schema.create_table_from_entity(article::Entity),
            schema.create_table_from_entity(article_tag::Entity),
            schema.create_table_from_entity(article_category::Entity),
            schema.create_table_from_entity(tag::Entity),
            schema.create_table_from_entity(category::Entity),
            schema.create_table_from_entity(fixed_content::Entity),
        ] {
            db.execute(backend.build(&table)).await.unwrap();
        }
        let modified = chrono::DateTime::parse_from_rfc3339("2020-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc);
        for (slug, date) in [
            ("日本語 & #1", modified),
            (
                "future-secret",
                chrono::Utc::now() + chrono::Duration::days(1),
            ),
        ] {
            article::ActiveModel {
                title: Set(slug.into()),
                slug: Set(slug.into()),
                content: Set("Body".into()),
                created_at: Set(date),
                updated_at: Set(modified),
                table_of_contents: Set(false),
                ..Default::default()
            }
            .insert(&db)
            .await
            .unwrap();
        }
        fixed_content::ActiveModel {
            title: Set("About".into()),
            slug: Set("about".into()),
            content: Set("About".into()),
            created_at: Set(modified),
            updated_at: Set(modified),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        for is_static in [false, true] {
            let xml = sitemap(&db, "https://example.com", is_static)
                .await
                .unwrap();
            let suffix = if is_static { "/" } else { "" };
            assert!(xml.contains(&format!("<loc>https://example.com/posts/%E6%97%A5%E6%9C%AC%E8%AA%9E%20%26%20%231{suffix}</loc>")));
            assert!(xml.contains(&format!("<loc>https://example.com/about{suffix}</loc>")));
            assert!(xml.contains("<lastmod>2020-01-02T03:04:05Z</lastmod>"));
            assert!(!xml.contains("future-secret"));
            assert_eq!(xml.matches("<loc>https://example.com/</loc>").count(), 1);
            assert_eq!(
                xml,
                sitemap(&db, "https://example.com", is_static)
                    .await
                    .unwrap()
            );
        }
        assert!(crate::slug::validate_fixed("sitemap.xml").is_err());
        assert!(crate::slug::validate_fixed("robots.txt").is_err());
    }

    #[test]
    fn public_origin_validation_and_robots() {
        assert_eq!(
            site_origin(Some("https://example.com/"))
                .unwrap()
                .as_deref(),
            Some("https://example.com")
        );
        for invalid in [
            "/relative",
            "ftp://example.com",
            "https://user:password@example.com",
            "https://example.com/blog",
            "https://example.com/?x=1",
            "https://example.com/#x",
        ] {
            assert!(site_origin(Some(invalid)).is_err(), "{invalid}");
        }
        assert!(site_origin(None).unwrap().is_none());
        assert_eq!(robots(None), "User-agent: *\nAllow: /\n");
        assert!(
            robots(Some("https://example.com"))
                .contains("Sitemap: https://example.com/sitemap.xml\n")
        );
    }
}
