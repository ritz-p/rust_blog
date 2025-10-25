use rocket::{State, futures::TryFutureExt, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter};

use crate::{
    repository::fixed_content::get_fixed_content_by_slug, utils::markdown::markdown_to_html,
};

#[get("/about")]
pub async fn about(db: &State<DatabaseConnection>) -> Result<Template, Status> {
    let conn = db.inner();
    let maybe = get_fixed_content_by_slug(conn, "about")
        .map_err(|_| Status::InternalServerError)
        .await?
        .ok_or(Status::NotFound);

    let about_page = match maybe {
        Ok(model) => model,
        Err(_) => return Err(Status::NotFound),
    };

    let content = markdown_to_html(&about_page.content);

    Ok(Template::render(
        "about",
        context! {
            title: about_page.title,
            content_html: content,
            created_at: about_page.created_at.to_string(),
            updated_at: about_page.updated_at.to_string(),
        },
    ))
}
