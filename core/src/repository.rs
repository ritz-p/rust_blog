pub mod article;
pub mod category;
pub mod fixed_content;
pub mod tag;

#[cfg(test)]
mod repository_tests {
    #[rocket::async_test]
    async fn taxonomy_lists_only_include_published_article_assignments() {
        use sea_orm::{ConnectionTrait, Database, Statement};
        let db = Database::connect("sqlite::memory:").await.unwrap();
        for sql in [
            "CREATE TABLE article (id INTEGER PRIMARY KEY, created_at TEXT)",
            "INSERT INTO article VALUES (1, '2020-01-01'), (2, '2999-01-01')",
            "CREATE TABLE tag (id INTEGER PRIMARY KEY, name TEXT, slug TEXT)",
            "CREATE TABLE category (id INTEGER PRIMARY KEY, name TEXT, slug TEXT)",
            "CREATE TABLE article_tag (article_id INTEGER, tag_id INTEGER)",
            "CREATE TABLE article_category (article_id INTEGER, category_id INTEGER)",
            "INSERT INTO tag VALUES (1, 'unused', 'unused'), (2, 'future', 'future'), (3, 'published', 'published')",
            "INSERT INTO category SELECT * FROM tag",
            "INSERT INTO article_tag VALUES (2, 2)",
            "INSERT INTO article_category VALUES (2, 2)",
        ] {
            db.execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
                .await
                .unwrap();
        }
        assert!(super::tag::get_all_tags(&db).await.unwrap().is_empty());
        assert!(
            super::category::get_all_categories(&db)
                .await
                .unwrap()
                .is_empty()
        );
        for sql in [
            "INSERT INTO article_tag VALUES (1, 3)",
            "INSERT INTO article_category VALUES (1, 3)",
        ] {
            db.execute(Statement::from_string(DatabaseBackend::Sqlite, sql))
                .await
                .unwrap();
        }
        assert_eq!(
            super::tag::get_all_tags(&db).await.unwrap()[0].slug,
            "published"
        );
        assert_eq!(
            super::category::get_all_categories(&db).await.unwrap()[0].slug,
            "published"
        );
    }
    use crate::domain::page::Page;
    use crate::entity::article;
    use crate::repository::article::{get_all_articles, get_article_by_slug};
    use chrono::Utc;
    use rocket::tokio;
    use sea_orm::MockExecResult;
    use sea_orm::{DatabaseBackend, MockDatabase};

    #[tokio::test]
    async fn test_get_all_articles_returns_list() {
        let dummy_article = article::Model {
            id: 1,
            title: "Test Title".to_owned(),
            slug: "test-slug".to_owned(),
            excerpt: Some("Excerpt".to_owned()),
            content: "Content".to_owned(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            icatch_path: None,
            table_of_contents: false,
        };
        let page = Page { number: 1, per: 10 };
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_exec_results([MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .append_query_results([Vec::<article::Model>::new(), vec![dummy_article.clone()]])
            .into_connection();
        let (models, _) = get_all_articles(&db, page, None)
            .await
            .expect("Query should succeed");
        assert_eq!(models, vec![dummy_article]);
    }

    #[tokio::test]
    async fn test_get_article_by_slug_not_found() {
        let db = MockDatabase::new(DatabaseBackend::Postgres)
            .append_query_results(vec![vec![] as Vec<article::Model>])
            .into_connection();

        let result = get_article_by_slug(&db, "nonexistent-slug")
            .await
            .expect("Query should succeed");
        assert!(
            result.is_none(),
            "No article should be found for nonexistent slug"
        );
    }
}
