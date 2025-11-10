use rocket::{State, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::{DatabaseConnection, DbErr};
use serde_json::json;

use crate::repository::{article::get_articles_by_tag_slug, tag::get_all_tags};

#[get("/tags")]
pub async fn tag_list(db: &State<DatabaseConnection>) -> Result<Template, Status> {
    let models = get_all_tags(db)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let tags = models
        .iter()
        .map(|tag| {
            json!({
                "name": tag.name.clone(),"slug": tag.slug.clone()
            })
        })
        .collect::<Vec<_>>();
    Ok(Template::render("tags", context! {tags}))
}

#[get("/tag/<slug>?<sort_key>")]
pub async fn tag_detail(
    db: &State<DatabaseConnection>,
    slug: &str,
    sort_key: Option<String>,
) -> Result<Template, Status> {
    let sort_key = sort_key.unwrap_or_else(|| "created_at".to_string());
    match get_articles_by_tag_slug(&db, slug, &sort_key).await {
        Ok(articles) => Ok(Template::render(
            "tag",
            context! {
                tag_slug: slug,
                sort_key: sort_key,
                articles: articles.iter().map(|article| {
                    json!({
                        "title": article.title.clone(),
                        "slug": article.slug,
                        "created_at": article.created_at.to_string(),
                    })
                }).collect::<Vec<_>>()
            },
        )),
        Err(DbErr::RecordNotFound(_)) => Err(Status::NotFound),
        Err(e) => {
            error!("tag_detail error for {}: {}", slug, e);
            Err(Status::InternalServerError)
        }
    }
}
