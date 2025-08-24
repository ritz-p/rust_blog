#[macro_use]
extern crate rocket;

mod entity;
mod repository;
mod utils;
mod view;
use entity::{
    article::{self, Model as Article},
    article_tag,
    category::{self, Entity as CategoryEntity, Model as Category},
    prelude::*,
    tag::{self, Entity as TagEntity, Model as Tag},
};
use repository::{
    article::{get_all_articles, get_articles_by_tag_slug},
    tag::get_all_tags,
};
use rocket::{State, futures::TryFutureExt, http::Status};
use rocket_dyn_templates::{Template, context};
use utils::markdown::markdown_to_html;
use view::{category::CategoryView, tag::TagView};

use sea_orm::{
    ColumnTrait, Database, DatabaseConnection, EntityTrait, JoinType, ModelTrait, QueryFilter,
    QuerySelect, RelationTrait,
};
use serde_json::json;

#[rocket::main]
async fn main() -> Result<(), anyhow::Error> {
    let db: DatabaseConnection = Database::connect(std::env::var("DATABASE_URL")?).await?;

    let _rocket = rocket::build()
        .manage(db)
        .attach(Template::fairing())
        .mount("/", routes![index, post_detail, tag_list, tag_detail])
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

#[get("/tag/<slug>")]
pub async fn tag_detail(db: &State<DatabaseConnection>, slug: &str) -> Result<Template, Status> {
    let articles = get_articles_by_tag_slug(db.inner(), slug)
        .await
        .map_err(|_| Status::InternalServerError)?;

    if articles.is_empty() {
        return Err(Status::NotFound);
    }

    Ok(Template::render(
        "tag",
        context! {
            tag_slug: slug,
            articles: articles.iter().map(|article| {
                json!({
                    "title": article.title.clone(),
                    "slug": slug.to_string(),
                    "create_at": article.created_at.to_string(),
                })
            }).collect::<Vec<_>>()
        },
    ))
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

    Ok(Template::render(
        "detail",
        context! {
            title: article.title,
            content_html: content,
            created_at: article.created_at.to_string(),
            tags: &tags,
            categories: &categories
        },
    ))
}
