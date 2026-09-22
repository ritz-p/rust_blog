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
    let config = load_env();
    validate_input_slugs(&config)?;
    taxonomy::normalize(&db).await?;
    println!("{:?}", config);
    run_fixed_content_seed(&db, &config.fixed_content_path).await?;
    println!("✅ 固定ページ Markdown → DB のシード完了");
    run_article_seed(&db, &config.article_path).await?;
    println!("✅ Article Markdown → DB のシード完了");

    Ok(())
}

fn validate_input_slugs(config: &config::PathConfig) -> anyhow::Result<()> {
    let mut errors = Vec::new();
    for (directory, fixed) in [
        (&config.fixed_content_path, true),
        (&config.article_path, false),
    ] {
        let mut registry = crate::slug::SlugRegistry::default();
        for path in markdown_files(directory) {
            let slug = if fixed {
                parse_markdown_to_fixed_content_matter(&path).map(|(matter, _)| matter.slug)
            } else {
                parse_markdown_to_front_matter(&path).map(|(matter, _)| matter.slug)
            };
            if let Ok(slug) = slug {
                let result = if fixed {
                    crate::slug::validate_fixed(&slug)
                } else {
                    crate::slug::validate(&slug, 100)
                };
                if let Err(error) = result.and_then(|()| registry.insert(&slug, path.clone())) {
                    errors.push(format!("{}: {error:#}", path.display()));
                }
            }
        }
    }
    anyhow::ensure!(errors.is_empty(), "{}", errors.join("\n"));
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

#[cfg(test)]
mod slug_tests {
    use super::*;

    #[test]
    fn preflight_rejects_duplicate_documents_before_seed() {
        let root = std::env::temp_dir().join(format!(
            "slug-input-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let articles = root.join("articles");
        let fixed = root.join("fixed");
        std::fs::create_dir_all(&articles).unwrap();
        std::fs::create_dir_all(&fixed).unwrap();
        let document = "---\ntitle: Test\nslug: same\ntags: []\ncategories: []\n---\nbody";
        std::fs::write(articles.join("a.md"), document).unwrap();
        std::fs::write(articles.join("b.md"), document).unwrap();
        std::fs::write(fixed.join("a.md"), document).unwrap();
        let config = config::PathConfig {
            article_path: articles.to_str().unwrap().into(),
            fixed_content_path: fixed.to_str().unwrap().into(),
        };
        let error = validate_input_slugs(&config).unwrap_err().to_string();
        assert!(
            error.contains("duplicate slug") && error.contains("a.md") && error.contains("b.md")
        );
        std::fs::remove_file(articles.join("b.md")).unwrap();
        validate_input_slugs(&config).unwrap();
        std::fs::write(
            fixed.join("a.md"),
            document.replace("slug: same", "slug: tags"),
        )
        .unwrap();
        assert!(
            validate_input_slugs(&config)
                .unwrap_err()
                .to_string()
                .contains("reserved route")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
