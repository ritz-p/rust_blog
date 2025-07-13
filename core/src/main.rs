#[macro_use]
extern crate rocket;

mod entity;
mod repository;
mod utils;

use crate::{
    entity::{
        article::{self, Model},
        prelude::*,
    },
    repository::article::get_all_articles,
    utils::markdown::markdown_to_html,
};
use rocket::{State, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::{ColumnTrait, Database, DatabaseConnection, EntityTrait, QueryFilter};
use serde_json::json;

#[rocket::main]
async fn main() -> Result<(), anyhow::Error> {
    let db: DatabaseConnection = Database::connect(std::env::var("DATABASE_URL")?).await?;
    let articles: Vec<article::Model> = Article::find().all(&db).await?;

    let _rocket = rocket::build()
        .manage(db)
        .attach(Template::fairing())
        .mount("/", routes![index, post_detail])
        .register("/", catchers![not_found])
        .launch()
        .await?;

    Ok(())
}

#[get("/")]
async fn index(db: &State<DatabaseConnection>) -> Template {
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

#[catch(404)]
fn not_found() -> Template {
    Template::render(
        "404",
        context! {
            site_name: "404 - My Rust Blog"
        },
    )
}

#[get("/posts/<slug>")]
async fn post_detail(db: &State<DatabaseConnection>, slug: &str) -> Result<Template, Status> {
    let maybe = article::Entity::find()
        .filter(article::Column::Slug.eq(slug.to_string()))
        .one(db.inner())
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound);

    let article = match maybe {
        Ok(model) => model,
        Err(_) => return Err(Status::NotFound),
    };

    let content = markdown_to_html(&article.content);

    Ok(Template::render(
        "detail",
        context! {
            title: article.title,
            content_html: content,
            created_at: article.created_at.to_string()
        },
    ))
}
