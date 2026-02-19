use rocket::{State, futures::TryFutureExt, http::Status};
use rocket_dyn_templates::{Template, context};
use rust_blog::utils::utc_to_jst;
use sea_orm::DatabaseConnection;
use serde_json::json;

use crate::{
    repository::{article::get_latest_articles, fixed_content::get_fixed_content_by_slug},
    utils::{
        config::CommonConfig,
        cut_out_string,
        markdown::{markdown_to_html, markdown_to_text},
    },
};

#[get("/<slug>")]
pub async fn fixed_content_detail(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    slug: &str,
) -> Result<Template, Status> {
    let conn = db.inner();
    let maybe = get_fixed_content_by_slug(conn, slug)
        .map_err(|_| Status::InternalServerError)
        .await?
        .ok_or(Status::NotFound);

    let fixed_content_page = match maybe {
        Ok(model) => model,
        Err(_) => return Err(Status::NotFound),
    };

    let content = markdown_to_html(&fixed_content_page.content);
    let excerpt = match fixed_content_page.excerpt.as_ref() {
        Some(value) => value.clone(),
        None => cut_out_string(&markdown_to_text(&fixed_content_page.content), 100),
    };

    let latest_articles: Vec<_> = get_latest_articles(db, 5)
        .await
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|model| {
            json!({
                "title":      model.title,
                "slug":       model.slug,
            })
        })
        .collect();

    Ok(Template::render(
        "about",
        context! {
            site_name: &config.site_name,
            favicon_path: &config.favicon_path,
            title: fixed_content_page.title,
            excerpt: excerpt,
            content_html: content,
            created_at: utc_to_jst(fixed_content_page.created_at),
            updated_at: fixed_content_page.updated_at.to_string(),
            latest_articles: latest_articles,
        },
    ))
}
