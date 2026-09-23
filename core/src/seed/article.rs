pub mod seed;
use crate::entity::{article, article::ActiveModel, article_tag};
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
    crate::slug::validate(&front_matter.slug, 100)?;
    validate(front_matter, body)?;
    crate::slug::validate_database_collision(db, "article", &front_matter.slug).await?;
    let active_model: ActiveModel = prepare(db, front_matter, body).await?;
    let article_id = upsert(db, active_model).await?;
    Ok(article_id)
}

pub async fn delete_article_by_slug(db: &DatabaseConnection, slug: &str) -> Result<(), DbErr> {
    crate::slug::validate(slug, 100).map_err(|error| DbErr::Custom(error.to_string()))?;
    if slug.trim().is_empty() {
        return Err(DbErr::Custom("slug is empty".into()));
    }

    article::Entity::delete_many()
        .filter(article::Column::Slug.eq(slug))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn seed_tag(
    db: &DatabaseConnection,
    front_matter: &FrontMatter,
    article_id: i32,
) -> Result<(), DbErr> {
    for tag_name in &front_matter.tags {
        let tag_slug = tag_name.to_lowercase();
        let existing = tag::Entity::find()
            .filter(tag::Column::Slug.eq(tag_slug.as_str()))
            .one(db)
            .await?;
        let tag_id = if let Some(m) = existing {
            m.id
        } else {
            tag::ActiveModel {
                name: Set(tag_name.clone()),
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
    for category_name in &front_matter.categories {
        let category_slug = category_name.to_lowercase();
        let existing = category::Entity::find()
            .filter(category::Column::Slug.eq(category_slug.as_str()))
            .one(db)
            .await?;
        let category_id = if let Some(m) = existing {
            m.id
        } else {
            category::ActiveModel {
                name: Set(category_name.clone()),
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
    #[rocket::async_test]
    async fn reseed_updates_table_of_contents_in_both_directions() {
        use sea_orm::{ConnectionTrait, Database, EntityTrait, Schema};
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        db.execute(backend.build(&schema.create_table_from_entity(article::Entity)))
            .await
            .unwrap();
        let mut matter = build_front_matter_from_title_and_slug("TOC", "31");
        for enabled in [false, true, false] {
            matter.table_of_contents = enabled;
            let id = seed_article(&db, &matter, "## Heading").await.unwrap();
            let saved = article::Entity::find_by_id(id)
                .one(&db)
                .await
                .unwrap()
                .unwrap();
            assert_eq!(saved.table_of_contents, enabled);
            assert_eq!(saved.content, "## Heading");
        }
    }

    use super::{delete_article_by_slug, seed_article};
    use crate::entity::article;
    use crate::utils::front_matter::FrontMatter;
    use chrono::{TimeZone, Utc};
    use rocket::tokio;
    use sea_orm::{DbBackend, DbErr, MockDatabase, MockExecResult};

    #[rocket::async_test]
    async fn taxonomy_seed_preserves_first_display_name_and_reuses_normalized_slug() {
        use sea_orm::{ConnectionTrait, Database, Statement};
        let db = Database::connect("sqlite::memory:").await.unwrap();
        for (table, relation, column) in [
            ("tag", "article_tag", "tag_id"),
            ("category", "article_category", "category_id"),
        ] {
            for sql in [
                format!(
                    "CREATE TABLE {table} (id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE, slug TEXT NOT NULL UNIQUE)"
                ),
                format!(
                    "CREATE TABLE {relation} (article_id INTEGER, {column} INTEGER REFERENCES {table}(id), PRIMARY KEY(article_id, {column}))"
                ),
            ] {
                db.execute(Statement::from_string(DbBackend::Sqlite, sql))
                    .await
                    .unwrap();
            }
        }
        let mut matter = build_front_matter_from_title_and_slug("Test", "test");
        matter.tags = vec!["C#".into(), "c#".into()];
        matter.categories = vec!["WebDev".into(), "webdev".into()];
        super::seed_tag(&db, &matter, 1).await.unwrap();
        super::seed_category(&db, &matter, 1).await.unwrap();
        matter.tags.reverse();
        matter.categories.reverse();
        super::seed_tag(&db, &matter, 1).await.unwrap();
        super::seed_category(&db, &matter, 1).await.unwrap();
        for (table, relation, name, slug) in [
            ("tag", "article_tag", "C#", "c#"),
            ("category", "article_category", "WebDev", "webdev"),
        ] {
            let rows = db
                .query_all(Statement::from_string(
                    DbBackend::Sqlite,
                    format!("SELECT name, slug FROM {table}"),
                ))
                .await
                .unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].try_get::<String>("", "name").unwrap(), name);
            assert_eq!(rows[0].try_get::<String>("", "slug").unwrap(), slug);
            let links = db
                .query_all(Statement::from_string(
                    DbBackend::Sqlite,
                    format!("SELECT * FROM {relation}"),
                ))
                .await
                .unwrap();
            assert_eq!(links.len(), 1);
        }
    }

    fn build_front_matter_from_title_and_slug(title: &str, slug: &str) -> FrontMatter {
        FrontMatter::new(
            title.to_string(),
            slug.to_string(),
            false,
            None,
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
            table_of_contents: false,
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
            .append_query_results([
                Vec::<article::Model>::new(),
                Vec::<article::Model>::new(),
                vec![returned.clone()],
            ])
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
            .append_query_results([vec![existing.clone()], vec![existing], vec![returned]])
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
            .append_query_results([
                Vec::<article::Model>::new(),
                Vec::<article::Model>::new(),
                vec![returned.clone()],
            ])
            .into_connection();

        let article_id = seed_article(&db, &front_matter, &body)
            .await
            .expect("seed should insert");
        assert_eq!(article_id, 1);
    }

    #[tokio::test]
    async fn test_delete_article_by_slug_executes_delete() {
        let db = MockDatabase::new(DbBackend::Sqlite)
            .append_exec_results([MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .into_connection();

        let result = delete_article_by_slug(&db, "to-delete").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_article_by_slug_returns_error_when_slug_is_empty() {
        let db = MockDatabase::new(DbBackend::Sqlite).into_connection();
        let result = delete_article_by_slug(&db, "   ").await;
        assert!(result.is_err());
    }
}
