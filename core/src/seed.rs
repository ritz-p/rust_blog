pub mod article;
pub mod config;
pub mod fixed_content;
pub mod markdown;
use crate::{
    entity::category::Entity as CategoryEntity,
    entity::tag::Entity as TagEntity,
    seed::{
        fixed_content::seed_fixed_content,
        markdown::{
            markdown_files, parse_markdown_to_fixed_content_matter, parse_markdown_to_front_matter,
        },
    },
};
use article::{seed_article, seed_category, seed_tag};
use config::{env::load_env, seeder::seed_from_toml};
use sea_orm::DatabaseConnection;

pub async fn run_all(db: DatabaseConnection) -> anyhow::Result<()> {
    let config = load_env();
    println!("{:?}", config);
    run_fixed_content_seed(&db, &config.fixed_content_path).await?;
    println!("✅ 固定ページ Markdown → DB のシード完了");
    run_article_seed(&db, &config.article_path).await?;
    println!("✅ Article Markdown → DB のシード完了");
    seed_from_toml::<TagEntity>(&db, &config.config_toml_path, "tags").await?;
    println!("✅ Tag Toml → DB のシード完了");
    seed_from_toml::<CategoryEntity>(&db, &config.config_toml_path, "categories").await?;
    println!("✅ Category Toml → DB のシード完了");

    Ok(())
}

async fn run_article_seed(db: &DatabaseConnection, dir: &str) -> Result<(), anyhow::Error> {
    for path in markdown_files(dir) {
        println!("{:?}", path);
        let (front_matter, body) = match parse_markdown_to_front_matter(&path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("FrontMatter parse error {:?}", e);
                continue;
            }
        };

        let article_id = seed_article(db, &front_matter, &body).await?;
        seed_tag(db, &front_matter, article_id).await?;
        seed_category(db, &front_matter, article_id).await?;
    }
    Ok(())
}

async fn run_fixed_content_seed(db: &DatabaseConnection, dir: &str) -> Result<(), anyhow::Error> {
    for path in markdown_files(dir) {
        println!("{:?}", path);
        let (front_matter, body) = match parse_markdown_to_fixed_content_matter(&path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("FrontMatter parse error {:?}", e);
                continue;
            }
        };
        seed_fixed_content(db, &front_matter, &body).await?;
    }
    Ok(())
}
