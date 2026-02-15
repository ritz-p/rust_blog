use rocket::{State, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::{DatabaseConnection, DbErr};
use serde_json::json;

use crate::{
    domain::{
        page::{Page, PageInfo},
        query::{PagingQuery, category::CategoryQuery},
    },
    repository::{article::get_article_by_category_slug, category::get_all_categories},
    utils::{config::CommonConfig, cut_out_string, markdown::markdown_to_text},
};

#[get("/categories")]
pub async fn category_list(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
) -> Result<Template, Status> {
    let models = get_all_categories(db)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let categories = models
        .iter()
        .map(|category| {
            json!({
                "name": category.name.clone(),"slug": category.slug.clone()
            })
        })
        .collect::<Vec<_>>();
    Ok(Template::render(
        "categories",
        context! {
            site_name: &config.site_name,
            favicon_path: &config.favicon_path,
            categories
        },
    ))
}

#[get("/category/<slug>?<query..>")]
pub async fn category_detail(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    query: Option<CategoryQuery>,
    slug: &str,
) -> Result<Template, Status> {
    let query = query.unwrap_or(CategoryQuery::new());
    let page = Page::new_from_query(&query);
    let sort_key = query.sort_key.unwrap_or_else(|| "created_at".to_string());
    match get_article_by_category_slug(db.inner(), page, slug, &sort_key).await {
        Ok((articles, page_info)) => {
            let base_path = "/category/".to_owned() + slug;
            let prev_url = PageInfo::get_prev_url(&page_info, &base_path, Some(&sort_key));
            let next_url = PageInfo::get_next_url(&page_info, &base_path, Some(&sort_key));
            let default_icatch_path = config.default_icatch_path.clone().unwrap_or_default();

            Ok(Template::render(
                "category",
                context! {
                    site_name: &config.site_name,
                    favicon_path: &config.favicon_path,
                    category_slug: slug,
                    sort_key: sort_key,
                    articles: articles.iter().map(|article| {
                        let icatch_path = article
                            .icatch_path
                            .clone()
                            .unwrap_or_else(|| default_icatch_path.clone());
                        let excerpt = match article.excerpt.as_ref() {
                            Some(value) => value.clone(),
                            None => cut_out_string(&markdown_to_text(&article.content), 100),
                        };
                        json!({
                            "title": article.title.clone(),
                            "slug": article.slug,
                            "icatch_path": icatch_path,
                            "excerpt": excerpt,
                            "created_at": article.created_at.to_string(),
                        })
                    }).collect::<Vec<_>>(),
                    page: page_info.count,
                    per: page_info.per,
                    total_pages: page_info.total_pages,
                    has_prev: page_info.has_prev,
                    has_next: page_info.has_next,
                    prev_page: page_info.prev_page,
                    next_page: page_info.next_page,
                    prev_url: prev_url,
                    next_url: next_url,
                },
            ))
        }
        Err(DbErr::RecordNotFound(_)) => Err(Status::NotFound),
        Err(e) => {
            error!("category_detail error for {}: {}", slug, e);
            Err(Status::InternalServerError)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::category_detail;
    use crate::entity::category;
    use crate::utils::config::CommonConfig;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use rocket_dyn_templates::Template;
    use sea_orm::{
        ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, DbBackend, MockDatabase,
        Statement,
    };

    async fn client_with_db(db: sea_orm::DatabaseConnection) -> Client {
        let rocket = rocket::custom(rocket::Config::figment().merge(("template_dir", "../templates")))
            .manage(db)
            .manage(CommonConfig {
                site_name: Some("Test Blog".to_string()),
                default_icatch_path: Some("/default.png".to_string()),
                favicon_path: Some("/favicon.ico".to_string()),
            })
            .attach(Template::fairing())
            .mount("/", routes![category_detail]);
        Client::tracked(rocket).await.expect("failed to build client")
    }

    async fn prepare_category_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("failed to connect sqlite memory");
        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TABLE category (id INTEGER PRIMARY KEY, name TEXT NOT NULL, slug TEXT NOT NULL);",
        ))
        .await
        .expect("failed to create category table");
        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TABLE article (id INTEGER PRIMARY KEY, title TEXT NOT NULL, slug TEXT NOT NULL, excerpt TEXT NULL, content TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, icatch_path TEXT NULL);",
        ))
        .await
        .expect("failed to create article table");
        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TABLE article_category (id INTEGER PRIMARY KEY, article_id INTEGER NOT NULL, category_id INTEGER NOT NULL);",
        ))
        .await
        .expect("failed to create article_category table");

        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO category (id, name, slug) VALUES (1, 'Dev', 'dev');",
        ))
        .await
        .expect("failed to insert category");

        for i in 1..=11 {
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                format!(
                    "INSERT INTO article (id, title, slug, excerpt, content, created_at, updated_at, icatch_path) VALUES ({id}, 'Title {id}', 'slug-{id}', NULL, 'This is **markdown** body', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP, NULL);",
                    id = i
                ),
            ))
            .await
            .expect("failed to insert article");
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                format!(
                    "INSERT INTO article_category (id, article_id, category_id) VALUES ({id}, {id}, 1);",
                    id = i
                ),
            ))
            .await
            .expect("failed to insert article_category");
        }

        db
    }

    #[rocket::async_test]
    async fn category_detail_uses_default_sort_key_and_generated_excerpt() {
        let db = prepare_category_db().await;
        let client = client_with_db(db).await;

        let response = client.get("/category/dev").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response
            .into_string()
            .await
            .expect("response body should exist");
        assert!(body.contains("sort_key=created_at"));
        assert!(body.contains("This is markdown body"));
    }

    #[rocket::async_test]
    async fn category_detail_returns_404_when_category_does_not_exist() {
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results([Vec::<category::Model>::new()])
            .into_connection();
        let client = client_with_db(db).await;

        let response = client.get("/category/missing").dispatch().await;
        assert_eq!(response.status(), Status::NotFound);
    }

    #[rocket::async_test]
    async fn category_detail_returns_500_on_unexpected_db_error() {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("failed to connect sqlite memory");
        let client = client_with_db(db).await;

        let response = client.get("/category/dev").dispatch().await;
        assert_eq!(response.status(), Status::InternalServerError);
    }
}
