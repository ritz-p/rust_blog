use chrono::Utc;
use sea_orm::{ActiveModelTrait, Database, DatabaseConnection, DbErr, Set};
use serde_yaml;
use std::default::Default;
use std::fs;
use walkdir::WalkDir;

use rust_blog::entity::article::ActiveModel;
use rust_blog::utils::front_matter::FrontMatter;
#[rocket::main]
async fn main() -> Result<(), DbErr> {
    let db: DatabaseConnection = Database::connect(std::env::var("DATABASE_URL").unwrap()).await?;
    for entry in WalkDir::new("content/articles")
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|e| e.to_str()) == Some("md")
        {
            let text = fs::read_to_string(entry.path()).unwrap();

            let parts: Vec<&str> = text.splitn(3, "---").collect();
            if parts.len() != 3 {
                eprintln!("front-matter が見つかりません: {:?}", entry.path());
                continue;
            }
            let fm_yaml = parts[1];
            let body_md = parts[2].trim_start();

            let fm: FrontMatter = serde_yaml::from_str(fm_yaml).unwrap();

            let am = ActiveModel {
                title: Set(fm.title.clone()),
                slug: Set(fm.slug.clone()),
                content: Set(body_md.to_string()),
                created_at: Set(fm.created_at),
                updated_at: Set(fm.updated_at),
                ..Default::default()
            };
            let _saved: _ = am.save(&db).await?;
        }
    }

    println!("✅ Markdown → DB のシード完了");
    Ok(())
}
