pub mod seed;
use crate::entity::{article::ActiveModel, article_tag};
use crate::entity::{article_category, category, tag};
use crate::utils;
use sea_orm::ActiveValue::Set;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};
use seed::{prepare, upsert, validate};
use utils::front_matter::FrontMatter;

pub async fn seed_article(
    db: &DatabaseConnection,
    front_matter: &FrontMatter,
    body: &str,
) -> Result<i32, anyhow::Error> {
    let active_model: ActiveModel = prepare(db, front_matter, body).await?;
    validate(front_matter, body)?;
    let article_id = upsert(db, active_model).await?;
    Ok(article_id)
}

pub async fn seed_tag(
    db: &DatabaseConnection,
    front_matter: &FrontMatter,
    article_id: i32,
) -> Result<(), DbErr> {
    for tag_slug in &front_matter.tags {
        let existing = tag::Entity::find()
            .filter(tag::Column::Slug.eq(tag_slug.as_str()))
            .one(db)
            .await?;
        let tag_id = if let Some(m) = existing {
            m.id
        } else {
            tag::ActiveModel {
                name: Set(tag_slug.clone()),
                slug: Set(tag_slug.clone()),
                ..Default::default()
            }
            .insert(db)
            .await?
            .id
        };

        let exists_link = article_tag::Entity::find()
            .filter(article_tag::Column::ArticleId.eq(article_id))
            .filter(article_tag::Column::TagId.eq(tag_id))
            .one(db)
            .await?
            .is_some();

        if !exists_link {
            article_tag::ActiveModel {
                article_id: Set(article_id),
                tag_id: Set(tag_id),
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}

pub async fn seed_category(
    db: &DatabaseConnection,
    front_matter: &FrontMatter,
    article_id: i32,
) -> Result<(), DbErr> {
    for category_slug in &front_matter.categories {
        let existing = category::Entity::find()
            .filter(category::Column::Slug.eq(category_slug.as_str()))
            .one(db)
            .await?;
        let category_id = if let Some(m) = existing {
            m.id
        } else {
            category::ActiveModel {
                name: Set(category_slug.clone()),
                slug: Set(category_slug.clone()),
                ..Default::default()
            }
            .insert(db)
            .await?
            .id
        };

        let exists_link = article_category::Entity::find()
            .filter(article_category::Column::ArticleId.eq(article_id))
            .filter(article_category::Column::CategoryId.eq(category_id))
            .one(db)
            .await?
            .is_some();

        if !exists_link {
            article_category::ActiveModel {
                article_id: Set(article_id),
                category_id: Set(category_id),
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::seed_article;
    use crate::entity::article;
    use crate::utils::front_matter::FrontMatter;
    use chrono::{TimeZone, Utc};
    use rocket::tokio;
    use sea_orm::{DbBackend, DbErr, MockDatabase, MockExecResult};

    fn build_front_matter_from_title_and_slug(title: &str, slug: &str) -> FrontMatter {
        FrontMatter::new(
            title.to_string(),
            slug.to_string(),
            Some("excerpt".to_string()),
            None,
            vec![],
            vec![],
        )
    }

    fn build_article(id: i32, title: &str, slug: &str, content: &str) -> article::Model {
        let ts = Utc.with_ymd_and_hms(2025, 1, 1, 0, 0, 0);
        article::Model {
            id,
            title: title.to_string(),
            slug: slug.to_string(),
            excerpt: Some("excerpt".to_string()),
            content: content.to_string(),
            created_at: ts.unwrap(),
            updated_at: ts.unwrap(),
            icatch_path: None,
        }
    }

    #[tokio::test]
    async fn test_seed_inserts_new_article() {
        let front_matter = build_front_matter_from_title_and_slug("New Title", "new-slug");
        let returned = build_article(1, "New Title", "new-slug", "body");

        let db = MockDatabase::new(DbBackend::Sqlite)
            .append_exec_results([MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .append_query_results([Vec::<article::Model>::new(), vec![returned.clone()]])
            .into_connection();

        let article_id = seed_article(&db, &front_matter, "body")
            .await
            .expect("seed should insert");
        assert_eq!(article_id, 1);
    }

    #[tokio::test]
    async fn test_seed_updates_existing_article() -> Result<(), DbErr> {
        let front_matter = build_front_matter_from_title_and_slug("Updated Title", "existing-slug");
        let existing = build_article(7, "Old Title", "existing-slug", "old body");
        let returned = build_article(7, "Updated Title", "existing-slug", "new body");

        let db = MockDatabase::new(DbBackend::Sqlite)
            .append_exec_results([MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .append_query_results([vec![existing], vec![returned]])
            .into_connection();

        let article_id = seed_article(&db, &front_matter, "new body")
            .await
            .expect("seed should insert");

        assert_eq!(article_id, 7);
        Ok(())
    }

    #[tokio::test]
    async fn test_seed_returns_error_on_invalid_front_matter() {
        let front_matter = build_front_matter_from_title_and_slug("", "bad-slug");

        let db = MockDatabase::new(DbBackend::Sqlite)
            .append_query_results([Vec::<article::Model>::new()])
            .into_connection();

        let result = seed_article(&db, &front_matter, "body").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_seed_inserts_huge_body() {
        let front_matter = build_front_matter_from_title_and_slug("New Title", "new-slug");
        let mut body = "".to_string();
        let alphabet = "abcdefghijklmnopqrstuvwxyz";
        for _ in 0..70000 {
            body += alphabet;
        }
        println!("{}", body);
        let returned = build_article(1, "New Title", "new-slug", &body);

        let db = MockDatabase::new(DbBackend::Sqlite)
            .append_exec_results([MockExecResult {
                last_insert_id: 1,
                rows_affected: 1,
            }])
            .append_query_results([Vec::<article::Model>::new(), vec![returned.clone()]])
            .into_connection();

        let article_id = seed_article(&db, &front_matter, &body)
            .await
            .expect("seed should insert");
        assert_eq!(article_id, 1);
    }
}
