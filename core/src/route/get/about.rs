use rocket::{State, futures::TryFutureExt, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter};

use crate::{
    repository::fixed_content::get_fixed_content_by_slug,
    utils::{cut_out_string, markdown::markdown_to_html},
};

#[get("/<slug>")]
pub async fn fixed_content_detail(
    db: &State<DatabaseConnection>,
    slug: &str,
) -> Result<Template, Status> {
    let conn = db.inner();
    let maybe = get_fixed_content_by_slug(conn, slug)
        .map_err(|_| Status::InternalServerError)
        .await?
        .ok_or(Status::NotFound);

    let about_page = match maybe {
        Ok(model) => model,
        Err(_) => return Err(Status::NotFound),
    };

    let content = markdown_to_html(&about_page.content);
    let excerpt = match about_page.excerpt.as_ref() {
        Some(value) => value.clone(),
        None => cut_out_string(&about_page.content, 100),
    };

    Ok(Template::render(
        "about",
        context! {
            title: about_page.title,
            excerpt: excerpt,
            content_html: content,
            created_at: about_page.created_at.to_string(),
            updated_at: about_page.updated_at.to_string(),
        },
    ))
}
