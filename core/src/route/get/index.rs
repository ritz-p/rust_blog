use rocket::State;
use rocket_dyn_templates::{Template, context};
use sea_orm::DatabaseConnection;
use serde_json::json;

use crate::{
    domain::{
        page::{Page, PageInfo},
        query::{PagingQuery, index::IndexQuery},
    },
    repository::article::{ArticlePeriod, get_all_articles, get_article_periods},
    utils::{config::CommonConfig, cut_out_string, markdown::markdown_to_text, utc_to_jst},
};

fn build_index_url(page: u64, per: u64, period: Option<ArticlePeriod>) -> String {
    match period {
        Some(period) => format!(
            "/?page={page}&per={per}&year={}&month={}",
            period.year, period.month
        ),
        None => format!("/?page={page}&per={per}"),
    }
}

#[get("/?<query..>")]
pub async fn index(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    query: Option<IndexQuery>,
) -> Template {
    let query = query.unwrap_or(IndexQuery::new());
    let page = Page::new_from_query(&query);
    let has_period_query = query.year.is_some() || query.month.is_some();
    let selected_period = match (query.year, query.month) {
        (Some(year), Some(month)) => ArticlePeriod::new(year, month),
        _ => None,
    };
    let (models, page_info) = if has_period_query && selected_period.is_none() {
        (Vec::new(), PageInfo::new(page.normalize(50), 0))
    } else {
        get_all_articles(db.inner(), page, selected_period)
            .await
            .unwrap()
    };
    let prev_url = if page_info.has_prev {
        build_index_url(page_info.prev_page, page_info.per, selected_period)
    } else {
        String::new()
    };
    let next_url = if page_info.has_next {
        build_index_url(page_info.next_page, page_info.per, selected_period)
    } else {
        String::new()
    };
    let default_icatch_path = config.default_icatch_path.clone().unwrap_or_default();
    let periods = get_article_periods(db.inner(), None)
        .await
        .unwrap_or_default();
    let period_links: Vec<_> = periods
        .iter()
        .map(|period| {
            json!({
                "label": format!("{}/{:02}", period.year, period.month),
                "href": format!("/?year={}&month={}", period.year, period.month),
                "is_selected": selected_period == Some(*period),
            })
        })
        .collect();
    let articles: Vec<_> = models
        .into_iter()
        .map(|m| {
            let excerpt = match m.excerpt.as_ref() {
                Some(value) => value.clone(),
                None => cut_out_string(&markdown_to_text(&m.content), 100),
            };
            let icatch_path = m
                .icatch_path
                .clone()
                .unwrap_or_else(|| default_icatch_path.clone());
            json!({
                "title":      m.title,
                "slug":       m.slug,
                "excerpt":    excerpt,
                "icatch_path": icatch_path,
                "created_at": utc_to_jst(m.created_at),
                "updated_at": utc_to_jst(m.updated_at),
            })
        })
        .collect();

    Template::render(
        "index",
        context! {
            site_name: &config.site_name,
            favicon_path: &config.favicon_path,
            articles:  articles,
            page: page_info.count,
            per: page_info.per,
            total_pages: page_info.total_pages,
            has_prev: page_info.has_prev,
            has_next: page_info.has_next,
            prev_page: page_info.prev_page,
            next_page: page_info.next_page,
            prev_url: prev_url,
            next_url: next_url,
            selected_period: selected_period.map(|period| format!("{}/{:02}", period.year, period.month)),
            period_links: period_links,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::index;
    use crate::utils::config::CommonConfig;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use rocket_dyn_templates::Template;
    use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

    async fn client_with_db(db: sea_orm::DatabaseConnection) -> Client {
        let rocket =
            rocket::custom(rocket::Config::figment().merge(("template_dir", "../templates")))
                .manage(db)
                .manage(CommonConfig {
                    site_name: Some("Test Blog".to_string()),
                    default_icatch_path: Some("/default.png".to_string()),
                    favicon_path: Some("/favicon.ico".to_string()),
                })
                .attach(Template::fairing())
                .mount("/", routes![index]);
        Client::tracked(rocket)
            .await
            .expect("failed to build client")
    }

    async fn prepare_index_db() -> DatabaseConnection {
        let db = Database::connect("sqlite::memory:")
            .await
            .expect("failed to connect sqlite memory");
        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "CREATE TABLE article (id INTEGER PRIMARY KEY, title TEXT NOT NULL, slug TEXT NOT NULL, excerpt TEXT NULL, content TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, icatch_path TEXT NULL);",
        ))
        .await
        .expect("failed to create article table");

        db.execute(Statement::from_string(
            DbBackend::Sqlite,
            "INSERT INTO article (id, title, slug, excerpt, content, created_at, updated_at, icatch_path) VALUES
            (1, 'Dec 1', 'dec-1', NULL, 'body', '2025-12-01T00:00:00Z', '2025-12-01T00:00:00Z', NULL),
            (2, 'Dec 2', 'dec-2', NULL, 'body', '2025-12-15T00:00:00Z', '2025-12-15T00:00:00Z', NULL),
            (3, 'Nov 1', 'nov-1', NULL, 'body', '2025-11-10T00:00:00Z', '2025-11-10T00:00:00Z', NULL),
            (5, 'Feb JST Boundary', 'feb-jst-boundary', NULL, 'body', '2026-01-31T15:00:00Z', '2026-01-31T15:00:00Z', NULL),
            (4, 'Future', 'future', NULL, 'body', '2099-01-10T00:00:00Z', '2099-01-10T00:00:00Z', NULL);",
        ))
        .await
        .expect("failed to insert articles");
        db
    }

    #[rocket::async_test]
    async fn index_filters_articles_by_year_and_month() {
        let db = prepare_index_db().await;
        let client = client_with_db(db).await;

        let response = client.get("/?year=2025&month=12").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response
            .into_string()
            .await
            .expect("response body should exist");

        assert!(body.contains("Dec 1"));
        assert!(body.contains("Dec 2"));
        assert!(!body.contains("Nov 1"));
        assert!(!body.contains("Future"));
    }

    #[rocket::async_test]
    async fn index_keeps_period_query_in_pagination_url() {
        let db = prepare_index_db().await;
        let client = client_with_db(db).await;

        let response = client.get("/?year=2025&month=12&per=1").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response
            .into_string()
            .await
            .expect("response body should exist");

        assert!(body.contains("pagination-next"));
        assert!(body.contains("year=2025"));
        assert!(body.contains("month=12"));
    }

    #[rocket::async_test]
    async fn index_filters_period_with_jst_month_boundary() {
        let db = prepare_index_db().await;
        let client = client_with_db(db).await;

        let feb_response = client.get("/?year=2026&month=2").dispatch().await;
        assert_eq!(feb_response.status(), Status::Ok);
        let feb_body = feb_response
            .into_string()
            .await
            .expect("response body should exist");
        assert!(feb_body.contains("Feb JST Boundary"));

        let jan_response = client.get("/?year=2026&month=1").dispatch().await;
        assert_eq!(jan_response.status(), Status::Ok);
        let jan_body = jan_response
            .into_string()
            .await
            .expect("response body should exist");
        assert!(!jan_body.contains("Feb JST Boundary"));
    }

    #[rocket::async_test]
    async fn index_returns_empty_for_invalid_period_query() {
        let db = prepare_index_db().await;
        let client = client_with_db(db).await;

        let response = client.get("/?year=300000&month=1").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response
            .into_string()
            .await
            .expect("response body should exist");

        assert!(!body.contains("Dec 1"));
        assert!(!body.contains("Dec 2"));
        assert!(!body.contains("Nov 1"));
        assert!(!body.contains("Future"));
        assert!(body.contains("まだ記事がありません。"));
    }
}
