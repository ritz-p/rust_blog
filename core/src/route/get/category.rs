use rocket::{State, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::DatabaseConnection;
use serde_json::json;

use crate::repository::{article::get_article_by_category_slug, category::get_all_categories};

#[get("/categories")]
pub async fn category_list(db: &State<DatabaseConnection>) -> Result<Template, Status> {
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
    Ok(Template::render("categories", context! {categories}))
}

#[get("/category/<slug>")]
pub async fn category_detail(
    db: &State<DatabaseConnection>,
    slug: &str,
) -> Result<Template, Status> {
    let articles = get_article_by_category_slug(db.inner(), slug)
        .await
        .map_err(|_| Status::InternalServerError)?;

    if articles.is_empty() {
        return Err(Status::NotFound);
    }

    Ok(Template::render(
        "category",
        context! {
            category_slug: slug,
            articles: articles.iter().map(|article| {
                json!({
                    "title": article.title.clone(),
                    "slug": article.slug,
                    "created_at": article.created_at.to_string(),
                })
            }).collect::<Vec<_>>()
        },
    ))
}
