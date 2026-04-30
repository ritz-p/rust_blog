use rust_blog::{static_site::export_site, utils::config::load_config};
use sea_orm::Database;
use std::{env, path::PathBuf};

#[rocket::main]
async fn main() -> anyhow::Result<()> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let output_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("dist"));

    let db = Database::connect(&database_url).await?;
    let config_map = load_config();
    export_site(&db, &config_map, output_dir).await?;
    Ok(())
}
