use rocket::State;
use rocket_dyn_templates::{Template, context};
use sea_orm::DatabaseConnection;
use serde_json::json;

use crate::{
    domain::page::{Page, PageInfo, PagingQuery},
    repository::article::get_all_articles,
    utils::{config::CommonConfig, cut_out_string},
};

#[get("/?<query..>")]
pub async fn index(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    query: Option<PagingQuery>,
) -> Template {
    let query = query.unwrap_or(PagingQuery {
        page: None,
        per: None,
    });
    let page = Page {
        page: query.page.unwrap_or(1),
        per: query.per.unwrap_or(10),
    };
    let (models, page_info) = get_all_articles(db.inner(), page).await.unwrap();

    let base_path = "/";
    let prev_url = PageInfo::get_prev_url(&page_info, base_path);
    let next_url = PageInfo::get_next_url(&page_info, base_path);

    let articles: Vec<_> = models
        .into_iter()
        .map(|m| {
            let excerpt = match m.excerpt.as_ref() {
                Some(value) => value.clone(),
                None => cut_out_string(&m.content, 100),
            };
            json!({
                "title":      m.title,
                "slug":       m.slug,
                "excerpt":    excerpt,
                "created_at": m.created_at.to_string(),
                "updated_at": m.updated_at.to_string(),
            })
        })
        .collect();

    Template::render(
        "index",
        context! {
            site_name: &config.site_name,
            articles:  articles,
            page: page_info.page,
            per: page_info.per,
            total_pages: page_info.total_pages,
            has_prev: page_info.has_prev,
            has_next: page_info.has_next,
            prev_page: page_info.prev_page,
            next_page: page_info.next_page,
            prev_url: prev_url,
            next_url: next_url,
        },
    )
}
