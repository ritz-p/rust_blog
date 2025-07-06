#[macro_use]
extern crate rocket;

use rocket::response::content::RawHtml;
use sea_orm::{Database, DatabaseConnection, EntityTrait};

mod entity;
use crate::entity::{article, prelude::*};

#[rocket::main]
async fn main() -> Result<(), anyhow::Error> {
    let db: DatabaseConnection = Database::connect(std::env::var("DATABASE_URL")?).await?;
    let articles: Vec<article::Model> = Article::find().all(&db).await?;
    println!("記事数: {}", articles.len());

    let _rocket = rocket::build()
        .mount("/", routes![index, post_detail])
        .launch()
        .await?;

    Ok(())
}

#[get("/")]
fn index() -> RawHtml<&'static str> {
    RawHtml("<h1>ようこそ Rust ブログへ!</h1>")
}

#[get("/posts/<slug>")]
fn post_detail(slug: &str) -> RawHtml<String> {
    RawHtml(format!(
        "<h1>記事: {}</h1><p>本文はまだありません</p>",
        slug
    ))
}
