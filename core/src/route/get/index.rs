use rocket::State;
use rocket_dyn_templates::{Template, context};
use sea_orm::DatabaseConnection;
use serde_json::json;

use crate::repository::article::get_all_articles;

#[get("/")]
pub async fn index(db: &State<DatabaseConnection>) -> Template {
    let models = get_all_articles(db.inner()).await.unwrap();

    let articles: Vec<_> = models
        .into_iter()
        .map(|m| {
            json!({
                "title":      m.title,
                "slug":       m.slug,
                "created_at": m.created_at.to_string(),
            })
        })
        .collect();

    Template::render(
        "index",
        context! {
            site_name: "My Rust Blog",
            articles:  articles,
        },
    )
}
