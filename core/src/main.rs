#[macro_use]
extern crate rocket;

mod domain;
mod entity;
mod entity_extension;
mod repository;
mod route;
mod utils;
mod view;
use sea_orm::{Database, DatabaseConnection};

use crate::{route::launch, utils::config::load_config};

#[rocket::main]
async fn main() -> Result<(), anyhow::Error> {
    let db: DatabaseConnection = Database::connect(std::env::var("DATABASE_URL")?).await?;
    let config_map = load_config();
    launch(db, config_map).await?;
    Ok(())
}
