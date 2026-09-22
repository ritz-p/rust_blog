use rocket::{State, http::Status};
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

#[derive(Clone, Copy)]
enum IndexUrlMode {
    Query,
    Archive,
}

fn build_index_url(
    page: u64,
    per: u64,
    period: Option<ArticlePeriod>,
    mode: IndexUrlMode,
) -> String {
    match (mode, period) {
        (IndexUrlMode::Archive, Some(period)) if page <= 1 => {
            format!("/archive/{}/{:02}", period.year, period.month)
        }
        (IndexUrlMode::Archive, Some(period)) => {
            format!("/archive/{}/{:02}/page/{page}", period.year, period.month)
        }
        (_, Some(period)) => format!(
            "/?page={page}&per={per}&year={}&month={}",
            period.year, period.month
        ),
        _ => format!("/?page={page}&per={per}"),
    }
}

#[get("/?<query..>")]
pub async fn index(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    query: Option<IndexQuery>,
) -> Result<Template, Status> {
    render_index(config, db, query, IndexUrlMode::Query).await
}

#[get("/archive/<year>/<month>")]
pub async fn index_archive(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    year: i32,
    month: u32,
) -> Result<Template, Status> {
    let query = IndexQuery {
        page: None,
        per: None,
        year: Some(year),
        month: Some(month),
    };
    render_index(config, db, Some(query), IndexUrlMode::Archive).await
}

#[get("/archive/<year>/<month>/page/<page>")]
pub async fn index_archive_page(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    year: i32,
    month: u32,
    page: u64,
) -> Result<Template, Status> {
    let query = IndexQuery {
        page: Some(page),
        per: None,
        year: Some(year),
        month: Some(month),
    };
    render_index(config, db, Some(query), IndexUrlMode::Archive).await
}

async fn render_index(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    query: Option<IndexQuery>,
    mode: IndexUrlMode,
) -> Result<Template, Status> {
    let query = query.unwrap_or(IndexQuery::new());
    let page = Page::new_from_query(&query, config.articles_per_page);
    let has_period_query = query.year.is_some() || query.month.is_some();
    let selected_period = match (query.year, query.month) {
        (Some(year), Some(month)) => ArticlePeriod::new(year, month),
        _ => None,
    };
    if matches!(mode, IndexUrlMode::Archive) && has_period_query && selected_period.is_none() {
        return Err(Status::NotFound);
    }
    let (models, page_info) = if has_period_query && selected_period.is_none() {
        (Vec::new(), PageInfo::new(page.normalize(u64::MAX), 0))
    } else {
        get_all_articles(db.inner(), page, selected_period)
            .await
            .map_err(|_| Status::InternalServerError)?
    };
    let prev_url = if page_info.has_prev {
        build_index_url(page_info.prev_page, page_info.per, selected_period, mode)
    } else {
        String::new()
    };
    let next_url = if page_info.has_next {
        build_index_url(page_info.next_page, page_info.per, selected_period, mode)
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
                "href": build_index_url(1, page.per, Some(*period), IndexUrlMode::Archive),
                "is_selected": selected_period == Some(*period),
            })
        })
        .collect();
    let articles: Vec<_> = models
        .into_iter()
        .map(|m| {
            let excerpt = match m.excerpt.as_ref() {
                Some(value) => crate::utils::markdown::markdown_to_text(value),
                None => cut_out_string(&markdown_to_text(&m.content), 100),
            };
            let slug = m.slug;
            let icatch_path = m
                .icatch_path
                .clone()
                .unwrap_or_else(|| default_icatch_path.clone());
            json!({
                "title":      m.title,
                "slug":       slug.clone(),
                "url":        format!("/posts/{}", crate::utils::url_segment(&slug)),
                "excerpt":    excerpt,
                "icatch_path": icatch_path,
                "created_at": utc_to_jst(m.created_at),
                "updated_at": utc_to_jst(m.updated_at),
            })
        })
        .collect();

    Ok(Template::render(
        "index",
        context! {
            site_name: &config.site_name,
            favicon_path: &config.favicon_path,
            tags_url: "/tags",
            categories_url: "/categories",
            about_url: "/about",
            articles:  articles,
            page: page_info.current_page,
            per: page_info.per,
            total_pages: page_info.total_pages,
            has_prev: page_info.has_prev,
            has_next: page_info.has_next,
            prev_page: page_info.prev_page,
            next_page: page_info.next_page,
            prev_url: prev_url,
            next_url: next_url,
            pagination: page_info.navigation(|number| build_index_url(number, page_info.per, selected_period, mode)),
            selected_period: selected_period.map(|period| format!("{}/{:02}", period.year, period.month)),
            period_links: period_links,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::{index, index_archive, index_archive_page};
    use crate::utils::config::CommonConfig;
    use rocket::http::Status;
    use rocket::local::asynchronous::Client;
    use rocket_dyn_templates::Template;
    use sea_orm::{ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement};

    async fn client_with_db(db: sea_orm::DatabaseConnection) -> Client {
        client_with_page_size(db, 10).await
    }

    async fn client_with_page_size(db: sea_orm::DatabaseConnection, per: u64) -> Client {
        let rocket =
            rocket::custom(rocket::Config::figment().merge(("template_dir", "../templates")))
                .manage(db)
                .manage(CommonConfig {
                    articles_per_page: per,
                    site_name: Some("Test Blog".to_string()),
                    default_icatch_path: Some("/default.png".to_string()),
                    favicon_path: Some("/favicon.ico".to_string()),
                })
                .attach(Template::fairing())
                .mount("/", routes![index, index_archive, index_archive_page]);
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
            "CREATE TABLE article (id INTEGER PRIMARY KEY, title TEXT NOT NULL, slug TEXT NOT NULL, excerpt TEXT NULL, content TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, icatch_path TEXT NULL, table_of_contents BOOLEAN NOT NULL DEFAULT false);",
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
            (6, 'Jan Legacy Text Date', 'jan-legacy-text-date', NULL, 'body', '2025-12-31 16:46:43', '2025-12-31 16:46:43', NULL),
            (4, 'Future', 'future', NULL, 'body', '2099-01-10T00:00:00Z', '2099-01-10T00:00:00Z', NULL);",
        ))
        .await
        .expect("failed to insert articles");
        db
    }

    #[rocket::async_test]
    async fn numbered_navigation_keeps_query_and_archive_urls() {
        let client = client_with_page_size(prepare_index_db().await, 1).await;
        for (url, latest, oldest) in [
            (
                "/?page=2&per=1&year=2025&month=12",
                "/?page=1&per=1&year=2025&month=12",
                "/?page=2&per=1&year=2025&month=12",
            ),
            (
                "/archive/2025/12/page/2",
                "/archive/2025/12",
                "/archive/2025/12/page/2",
            ),
        ] {
            let response = client.get(url).dispatch().await;
            assert_eq!(response.status(), Status::Ok);
            let html = html_escape::decode_html_entities(&response.into_string().await.unwrap())
                .into_owned();
            assert!(
                html.contains(&format!("href=\"{latest}\" aria-label=\"Latest page\"")),
                "{html}"
            );
            assert!(html.contains("aria-current=\"page\" aria-label=\"Page 2\""));
            assert!(html.contains("aria-disabled=\"true\">Oldest"));
            assert!(!html.contains(&format!("href=\"{oldest}\" aria-label=\"Oldest page\"")));
        }
    }

    #[rocket::async_test]
    async fn index_uses_configured_page_size_and_keeps_query_override() {
        let client = client_with_page_size(prepare_index_db().await, 2).await;
        for (url, count) in [("/", 2), ("/?page=3", 1), ("/?per=3", 3)] {
            let response = client.get(url).dispatch().await;
            assert_eq!(response.status(), Status::Ok);
            let html = response.into_string().await.unwrap();
            assert_eq!(
                html.matches("<article class=\"article-card\">").count(),
                count
            );
        }
    }

    #[rocket::async_test]
    async fn index_paginates_at_ten_and_eleven_articles() {
        for total in [10, 11] {
            let db = prepare_index_db().await;
            db.execute(Statement::from_string(
                DbBackend::Sqlite,
                "DELETE FROM article",
            ))
            .await
            .unwrap();
            for id in 1..=total {
                db.execute(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    "INSERT INTO article (id, title, slug, content, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
                    vec![id.into(), format!("Boundary article {id:02}").into(), format!("boundary-{id:02}").into(), "body".into(), format!("2025-12-{id:02}T00:00:00Z").into(), format!("2025-12-{id:02}T00:00:00Z").into()],
                )).await.unwrap();
            }
            let client = client_with_db(db).await;
            let response = client.get("/").dispatch().await;
            assert_eq!(response.status(), Status::Ok);
            let html = response.into_string().await.unwrap();
            assert_eq!(html.matches("<article class=\"article-card\">").count(), 10);
            assert!(html.contains("class=\"pagination-previous\" disabled"));
            if total == 10 {
                assert!(html.contains("Page 1 / 1"));
                assert!(html.contains("class=\"pagination-next\" disabled"));
                assert!(html.contains("Boundary article 01"));
            } else {
                assert!(html.contains("Page 1 / 2"));
                assert!(html.contains("class=\"pagination-next\" href="));
                assert!(!html.contains("Boundary article 01"));
                let response = client.get("/?page=2&per=10").dispatch().await;
                assert_eq!(response.status(), Status::Ok);
                let html = response.into_string().await.unwrap();
                assert_eq!(html.matches("<article class=\"article-card\">").count(), 1);
                assert!(html.contains("Boundary article 01"));
                assert!(html.contains("Page 2 / 2"));
                assert!(html.contains("class=\"pagination-next\" disabled"));
                assert!(html.contains("class=\"pagination-previous\" href="));
            }
        }
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
    async fn index_uses_server_urls_instead_of_static_archive_urls() {
        let db = prepare_index_db().await;
        let client = client_with_db(db).await;

        let response = client.get("/").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response
            .into_string()
            .await
            .expect("response body should exist");

        assert!(body.contains("&#x2F;archive&#x2F;"));
        assert!(!body.contains("/?year="));
    }

    #[rocket::async_test]
    async fn archive_route_filters_articles_by_year_and_month() {
        let db = prepare_index_db().await;
        let client = client_with_db(db).await;

        let response = client.get("/archive/2025/12").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        let body = response
            .into_string()
            .await
            .expect("response body should exist");

        assert!(body.contains("Dec 1"));
        assert!(body.contains("Dec 2"));
        assert!(!body.contains("Nov 1"));
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
        assert!(jan_body.contains("Jan Legacy Text Date"));
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
