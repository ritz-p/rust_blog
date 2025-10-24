#[macro_use]
extern crate rocket;

mod entity;
mod repository;
mod route;
mod utils;
mod view;
use sea_orm::{Database, DatabaseConnection};

use crate::route::launch;

#[rocket::main]
async fn main() -> Result<(), anyhow::Error> {
    let db: DatabaseConnection = Database::connect(std::env::var("DATABASE_URL")?).await?;
    launch(db).await?;
    Ok(())
}
