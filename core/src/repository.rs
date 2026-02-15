pub mod article;
pub mod category;
pub mod fixed_content;
pub mod tag;

#[cfg(test)]
mod repository_tests {
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
        };
        let page = Page { count: 1, per: 10 };
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_exec_results([MockExecResult {
                last_insert_id: 0,
                rows_affected: 1,
            }])
            .append_query_results([Vec::<article::Model>::new(), vec![dummy_article.clone()]])
            .into_connection();
        let (models, _) = get_all_articles(&db, page)
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
