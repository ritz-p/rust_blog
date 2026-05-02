use rust_blog::{
    static_site::{ExportPaths, export_site},
    utils::config::load_config_from_file,
};
use sea_orm::Database;
use std::{
    env,
    path::{Path, PathBuf},
};

fn resolve_file(env_key: &str, candidates: &[PathBuf]) -> PathBuf {
    if let Ok(value) = env::var(env_key) {
        return PathBuf::from(value);
    }
    candidates
        .iter()
        .find(|path| path.is_file())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}

fn resolve_dir(env_key: &str, candidates: &[PathBuf]) -> PathBuf {
    if let Ok(value) = env::var(env_key) {
        return PathBuf::from(value);
    }
    candidates
        .iter()
        .find(|path| path.is_dir())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone())
}

#[rocket::main]
async fn main() -> anyhow::Result<()> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let output_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("dist"));
    let current_dir = env::current_dir()?;
    let executable_dir = env::current_exe()?
        .parent()
        .map(Path::to_path_buf)
        .expect("export binary should have a parent directory");
    let config_path = resolve_file(
        "RUST_BLOG_CONFIG_PATH",
        &[
            current_dir.join("blog_config.toml"),
            executable_dir.join("blog_config.toml"),
        ],
    );
    let templates_dir = resolve_dir(
        "RUST_BLOG_TEMPLATES_DIR",
        &[current_dir.join("templates"), executable_dir.join("templates")],
    );
    let content_dir = resolve_dir(
        "RUST_BLOG_CONTENT_DIR",
        &[current_dir.join("content"), executable_dir.join("content")],
    );

    let db = Database::connect(&database_url).await?;
    let config_map = load_config_from_file(&config_path);
    let paths = ExportPaths {
        templates_dir,
        content_dir,
    };
    export_site(&db, &config_map, output_dir, &paths).await?;
    Ok(())
}
