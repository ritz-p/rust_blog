#[macro_use]
extern crate rocket;

mod entity;
mod repository;
mod route;
mod utils;
mod view;
use std::collections::HashMap;

use anyhow::Context;
use sea_orm::{Database, DatabaseConnection};

use crate::{route::launch, utils::config::load_config};

#[rocket::main]
async fn main() -> Result<(), anyhow::Error> {
    let db: DatabaseConnection = Database::connect(std::env::var("DATABASE_URL")?).await?;
    let config_map = load_config();
    launch(db, config_map).await?;
    Ok(())
}
