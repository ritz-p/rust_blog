pub mod article;
pub mod config;
pub mod fixed_content;
pub mod markdown;
mod taxonomy;
use crate::seed::{
    fixed_content::seed_fixed_content,
    markdown::{
        markdown_files, parse_markdown_to_fixed_content_matter, parse_markdown_to_front_matter,
    },
};
use article::{delete_article_by_slug, seed_article, seed_category, seed_tag};
use config::env::load_env;
use sea_orm::DatabaseConnection;

pub async fn run_all(db: DatabaseConnection) -> anyhow::Result<()> {
    taxonomy::normalize(&db).await?;
    let config = load_env();
    println!("{:?}", config);
    run_fixed_content_seed(&db, &config.fixed_content_path).await?;
    println!("✅ 固定ページ Markdown → DB のシード完了");
    run_article_seed(&db, &config.article_path).await?;
    println!("✅ Article Markdown → DB のシード完了");

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

        if front_matter.deleted {
            delete_article_by_slug(db, &front_matter.slug).await?;
            continue;
        }

        if std::env::var("RUST_BLOG_REQUIRE_CREATED_AT").as_deref() == Ok("1") {
            anyhow::ensure!(
                front_matter.created_at.is_some(),
                "{}: static export requires created_at (or date) in article front matter",
                path.display()
            );
        }

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
