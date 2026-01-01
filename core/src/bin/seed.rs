use rust_blog::seed::run_all;
use sea_orm::{Database, DatabaseConnection, DbErr};

#[rocket::main]
async fn main() -> anyhow::Result<()> {
    let db = connect_db().await?;
    run_all(db).await?;
    Ok(())
}

async fn connect_db() -> Result<DatabaseConnection, DbErr> {
    let url = std::env::var("DATABASE_URL").expect("DATABASE URL must be set");
    Database::connect(&url).await
}
