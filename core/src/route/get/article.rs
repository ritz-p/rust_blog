use crate::{
    entity::{article, category, tag},
    utils::{markdown::markdown_to_html, utc_to_jst},
    view::{category::CategoryView, tag::TagView},
};
use rocket::{State, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait, QueryFilter};

#[get("/posts/<slug>")]
pub async fn post_detail(db: &State<DatabaseConnection>, slug: &str) -> Result<Template, Status> {
    let conn = db.inner();
    let maybe = article::Entity::find()
        .filter(article::Column::Slug.eq(slug.to_string()))
        .one(conn)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound);

    let article = match maybe {
        Ok(model) => model,
        Err(_) => return Err(Status::NotFound),
    };

    let content = markdown_to_html(&article.content);

    let tags: Vec<_> = article
        .find_related(tag::Entity)
        .all(conn)
        .await
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|tag| TagView {
            name: tag.name,
            slug: tag.slug,
        })
        .collect();

    let categories: Vec<_> = article
        .find_related(category::Entity)
        .all(conn)
        .await
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|category| CategoryView {
            name: category.name,
            slug: category.slug,
        })
        .collect();
    let created_at = utc_to_jst(article.created_at);
    let updated_at = utc_to_jst(article.updated_at);

    Ok(Template::render(
        "article_detail",
        context! {
            title: article.title,
            content_html: content,
            created_at: created_at,
            updated_at: updated_at,
            tags: &tags,
            categories: &categories
        },
    ))
}
