use rocket::State;
use rocket_dyn_templates::{Template, context};
use sea_orm::DatabaseConnection;
use serde_json::json;

use crate::{
    domain::{
        page::{Page, PageInfo},
        query::{PagingQuery, index::IndexQuery},
    },
    repository::article::get_all_articles,
    utils::{config::CommonConfig, cut_out_string, markdown::markdown_to_text},
};

#[get("/?<query..>")]
pub async fn index(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    query: Option<IndexQuery>,
) -> Template {
    let query = query.unwrap_or(IndexQuery::new());
    let page = Page::new_from_query(&query);
    let (models, page_info) = get_all_articles(db.inner(), page).await.unwrap();
    let base_path = "/";
    let prev_url = PageInfo::get_prev_url(&page_info, base_path, None);
    let next_url = PageInfo::get_next_url(&page_info, base_path, None);
    let default_icatch_path = config.default_icatch_path.clone().unwrap_or_default();
    let articles: Vec<_> = models
        .into_iter()
        .map(|m| {
            let excerpt = match m.excerpt.as_ref() {
                Some(value) => value.clone(),
                None => markdown_to_text(&cut_out_string(&m.content, 100)),
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
                "created_at": m.created_at.to_string(),
                "updated_at": m.updated_at.to_string(),
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
        },
    )
}
