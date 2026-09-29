use anyhow::Result;
use sea_orm::DatabaseConnection;

fn xml(value: &str) -> String {
    let value: String = value.chars().filter(|c| matches!(*c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')).collect();
    html_escape::encode_double_quoted_attribute(&value).into_owned()
}

pub async fn atom(
    db: &DatabaseConnection,
    origin: &str,
    title: &str,
    is_static: bool,
) -> Result<String> {
    let mut articles = crate::repository::article::get_all_published_articles(db).await?;
    articles.sort_by(|a, b| {
        b.created_at
            .cmp(&a.created_at)
            .then_with(|| a.slug.cmp(&b.slug))
    });
    articles.truncate(20);
    let updated = articles
        .iter()
        .map(|article| article.updated_at.max(article.created_at))
        .max()
        .unwrap_or(chrono::DateTime::UNIX_EPOCH);
    let timestamp = |date: chrono::DateTime<chrono::Utc>| {
        date.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    };
    let feed_url = xml(&format!("{origin}/feed.xml"));
    let title = xml(title);
    let mut output = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<feed xmlns=\"http://www.w3.org/2005/Atom\"><id>{feed_url}</id><title>{title}</title><updated>{}</updated><author><name>{title}</name></author><link rel=\"self\" href=\"{feed_url}\" type=\"application/atom+xml\"/><link rel=\"alternate\" href=\"{}/\"/>",
        timestamp(updated),
        xml(origin)
    );
    for article in articles {
        let id = format!(
            "{origin}/posts/{}",
            crate::utils::url_segment(&article.slug)
        );
        let url = format!("{id}{}", if is_static { "/" } else { "" });
        let summary = crate::utils::markdown::markdown_to_text(
            article.excerpt.as_deref().unwrap_or(&article.content),
        );
        let summary = crate::utils::cut_out_string(&summary, 300);
        output.push_str(&format!("<entry><id>{}</id><title>{}</title><link rel=\"alternate\" href=\"{}\"/><published>{}</published><updated>{}</updated><summary type=\"text\">{}</summary></entry>", xml(&id), xml(&article.title), xml(&url), timestamp(article.created_at), timestamp(article.updated_at.max(article.created_at)), xml(&summary)));
    }
    output.push_str("</feed>\n");
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::article;
    use sea_orm::{ActiveModelTrait, ConnectionTrait, Database, EntityTrait, Schema, Set};

    #[rocket::async_test]
    async fn feed_filters_publication_limits_entries_and_keeps_ids_between_modes() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        db.execute(backend.build(&Schema::new(backend).create_table_from_entity(article::Entity)))
            .await
            .unwrap();
        let empty = atom(&db, "https://example.com", "A <&> \"blog\"", false)
            .await
            .unwrap();
        assert!(empty.contains("1970-01-01T00:00:00Z") && !empty.contains("<entry>"));
        for index in 0..25 {
            let date = chrono::DateTime::from_timestamp(1_600_000_000 + index * 86400, 0).unwrap();
            article::ActiveModel {
                title: Set(format!("Title <&> {index}")),
                slug: Set(format!("記事-{index}")),
                content: Set("**plain** <unsafe> & text".into()),
                created_at: Set(if index == 24 {
                    chrono::Utc::now() + chrono::Duration::days(365)
                } else {
                    date
                }),
                updated_at: Set(date),
                table_of_contents: Set(false),
                ..Default::default()
            }
            .insert(&db)
            .await
            .unwrap();
        }
        article::Entity::delete_by_id(24).exec(&db).await.unwrap();
        for is_static in [false, true] {
            let output = atom(&db, "https://example.com", "Blog", is_static)
                .await
                .unwrap();
            assert_eq!(output.matches("<entry>").count(), 20);
            assert!(
                !output.contains("Title &lt;&amp;&gt; 24")
                    && !output.contains("Title &lt;&amp;&gt; 23")
            );
            assert!(output.contains("<id>https://example.com/posts/%E8%A8%98%E4%BA%8B-22</id>"));
            let suffix = if is_static { "/" } else { "" };
            assert!(output.contains(&format!(
                "href=\"https://example.com/posts/%E8%A8%98%E4%BA%8B-22{suffix}\""
            )));
            assert!(!output.contains("<unsafe>"));
        }
    }
}
