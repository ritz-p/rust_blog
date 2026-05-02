use crate::{
    repository::{
        article::{get_article_by_slug, get_latest_articles},
        category::get_categories_by_article,
        tag::get_tags_by_article,
    },
    utils::{config::CommonConfig, markdown::markdown_to_html, utc_to_jst},
};
use rocket::{State, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::DatabaseConnection;
use serde_json::json;

#[get("/posts/<slug>")]
pub async fn article_detail(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    slug: &str,
) -> Result<Template, Status> {
    let conn = db.inner();
    let maybe = get_article_by_slug(conn, slug)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound);

    let article = match maybe {
        Ok(model) => model,
        Err(_) => return Err(Status::NotFound),
    };

    let content = markdown_to_html(&article.content);

    let tags: Vec<_> = get_tags_by_article(conn, &article)
        .await
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|tag| {
            let slug = tag.slug;
            json!({
                "name": tag.name,
                "slug": slug.clone(),
                "url": format!("/tag/{slug}"),
            })
        })
        .collect();

    let categories: Vec<_> = get_categories_by_article(conn, &article)
        .await
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|category| {
            let slug = category.slug;
            json!({
                "name": category.name,
                "slug": slug.clone(),
                "url": format!("/category/{slug}"),
            })
        })
        .collect();
    let created_at = utc_to_jst(article.created_at);
    let updated_at = utc_to_jst(article.updated_at);

    let latest_articles: Vec<_> = get_latest_articles(db, 5)
        .await
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|model| {
            let slug = model.slug;
            json!({
                "title":      model.title,
                "slug":       slug.clone(),
                "url":        format!("/posts/{slug}"),
            })
        })
        .collect();

    Ok(Template::render(
        "article_detail",
        context! {
            site_name: &config.site_name,
            favicon_path: &config.favicon_path,
            tags_url: "/tags",
            categories_url: "/categories",
            about_url: "/about",
            title: article.title,
            content_html: content,
            created_at: created_at,
            updated_at: updated_at,
            tags: &tags,
            categories: &categories,
            latest_articles: latest_articles
        },
    ))
}
