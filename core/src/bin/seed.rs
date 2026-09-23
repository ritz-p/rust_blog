use anyhow::Context;
use rust_blog::seed::run_all;
use sea_orm::Database;
use std::process::ExitCode;

#[rocket::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let db = Database::connect(&url)
        .await
        .context("connect to database")?;
    run_all(db).await
}
