pub mod article;
pub mod config;
pub mod fixed_content;
pub mod markdown;
mod taxonomy;

use anyhow::{Context, Result, ensure};
use article::{delete_article_by_slug, seed_article, seed_category, seed_tag};
use config::{PathConfig, env::load_env};
use fixed_content::seed_fixed_content;
use markdown::{
    markdown_files, parse_markdown_to_fixed_content_matter, parse_markdown_to_front_matter,
};
use sea_orm::DatabaseConnection;
use std::path::Path;

pub async fn run_all(db: DatabaseConnection) -> Result<()> {
    run_with_config(
        &db,
        &load_env(),
        std::env::var("RUST_BLOG_REQUIRE_CREATED_AT").as_deref() == Ok("1"),
    )
    .await
}

async fn run_with_config(
    db: &DatabaseConnection,
    config: &PathConfig,
    require_created_at: bool,
) -> Result<()> {
    let mut errors = Vec::new();
    let mut succeeded = 0;
    let slug_errors = validate_input_slugs(config);
    if let Err(error) = taxonomy::normalize(db).await {
        errors.push(format!("taxonomy normalization: {error:#}"));
    }
    for (dir, article) in [
        (&config.fixed_content_path, false),
        (&config.article_path, true),
    ] {
        for entry in markdown_files(dir) {
            let path = match entry {
                Ok(path) => path,
                Err(error) => {
                    errors.push(format!(
                        "{}: traversal: {error}",
                        error.path().unwrap_or(Path::new(dir)).display()
                    ));
                    continue;
                }
            };
            if let Some(error) = slug_errors.get(&path) {
                errors.push(format!("{}: {error}", path.display()));
                continue;
            }
            let result = if article {
                seed_article_file(db, &path, require_created_at).await
            } else {
                seed_fixed_file(db, &path).await
            };
            match result {
                Ok(()) => succeeded += 1,
                Err(error) => errors.push(format!("{}: {error:#}", path.display())),
            }
        }
    }
    println!(
        "Seed completed: {succeeded} file(s) succeeded, {} error(s)",
        errors.len()
    );
    ensure!(errors.is_empty(), "Seed errors:\n{}", errors.join("\n"));
    Ok(())
}

fn validate_input_slugs(
    config: &PathConfig,
) -> std::collections::HashMap<std::path::PathBuf, String> {
    use std::{collections::HashMap, path::PathBuf};
    let mut errors = HashMap::new();
    for (directory, fixed) in [
        (&config.fixed_content_path, true),
        (&config.article_path, false),
    ] {
        let mut groups: HashMap<String, Vec<PathBuf>> = HashMap::new();
        for path in markdown_files(directory).flatten() {
            let slug = markdown::parse_markdown_slug(&path);
            if let Ok(slug) = slug {
                let validation = if fixed {
                    crate::slug::validate_fixed(&slug)
                } else {
                    crate::slug::validate(&slug, 100)
                };
                if let Err(error) = validation {
                    errors.insert(path, error.to_string());
                } else {
                    groups
                        .entry(crate::slug::collision_key(&slug))
                        .or_default()
                        .push(path);
                }
            }
        }
        for (slug, paths) in groups {
            if paths.len() > 1 {
                let names = paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                for path in paths {
                    errors.insert(path, format!("duplicate slug {slug:?}: {names}"));
                }
            }
        }
    }
    errors
}

async fn seed_article_file(
    db: &DatabaseConnection,
    path: &Path,
    require_created_at: bool,
) -> Result<()> {
    let (matter, body) =
        parse_markdown_to_front_matter(path).map_err(|error| anyhow::anyhow!("parse: {error}"))?;
    if matter.deleted {
        delete_article_by_slug(db, &matter.slug)
            .await
            .context("delete article")?;
        return Ok(());
    }
    ensure!(
        !require_created_at || matter.created_at.is_some(),
        "static export requires created_at (or date) in article front matter"
    );
    let id = seed_article(db, &matter, &body)
        .await
        .context("save article")?;
    let mut errors = Vec::new();
    if let Err(error) = seed_tag(db, &matter, id).await {
        errors.push(format!("save tags: {error:#}"));
    }
    if let Err(error) = seed_category(db, &matter, id).await {
        errors.push(format!("save categories: {error:#}"));
    }
    ensure!(errors.is_empty(), "{}", errors.join("; "));
    Ok(())
}

async fn seed_fixed_file(db: &DatabaseConnection, path: &Path) -> Result<()> {
    let (matter, body) = parse_markdown_to_fixed_content_matter(path)
        .map_err(|error| anyhow::anyhow!("parse: {error}"))?;
    seed_fixed_content(db, &matter, &body)
        .await
        .context("save fixed page")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{article, article_category, article_tag, category, tag};
    use sea_orm::{ConnectionTrait, Database, EntityTrait, Schema, Statement};

    #[rocket::async_test]
    async fn collects_errors_and_seeds_later_files() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        for table in [
            schema.create_table_from_entity(article::Entity),
            schema.create_table_from_entity(tag::Entity),
            schema.create_table_from_entity(category::Entity),
            schema.create_table_from_entity(article_tag::Entity),
            schema.create_table_from_entity(article_category::Entity),
        ] {
            db.execute(backend.build(&table)).await.unwrap();
        }
        db.execute(Statement::from_string(backend, "CREATE TRIGGER reject_article BEFORE INSERT ON article WHEN NEW.slug = 'reject' BEGIN SELECT RAISE(FAIL, 'rejected article'); END;")).await.unwrap();
        let root = std::env::temp_dir().join(format!(
            "seed-errors-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let articles = root.join("articles");
        std::fs::create_dir_all(&articles).unwrap();
        std::fs::write(articles.join("01-broken.md"), "broken").unwrap();
        for (file, slug) in [
            ("00-duplicate-a.md", "Σ"),
            ("00-duplicate-b.md", "ς"),
            ("00-invalid-slug.md", "COM¹"),
        ] {
            std::fs::write(articles.join(file), format!("---\ntitle: Test\nslug: {slug}\ndate: 2026-01-01\ntags: []\ncategories: []\n---\nbody")).unwrap();
        }
        std::fs::write(
            articles.join("00-duplicate-b.md"),
            "---\nslug: ς\ntags: false\ncategories: []\n---\nbody",
        )
        .unwrap();
        std::fs::write(
            articles.join("01-invalid.md"),
            "---\ntitle: ''\nslug: invalid\ndate: 2026-01-01\ntags: []\ncategories: []\n---\nbody",
        )
        .unwrap();
        for (file, slug, date) in [
            ("02-date.md", "no-date", ""),
            ("03-db.md", "reject", "date: 2026-01-01\n"),
            ("04-good.md", "good", "date: 2026-01-01\n"),
        ] {
            std::fs::write(
                articles.join(file),
                format!(
                    "---\ntitle: Test\nslug: {slug}\n{date}tags: []\ncategories: []\n---\nbody"
                ),
            )
            .unwrap();
        }
        let config = PathConfig {
            fixed_content_path: root.join("missing-fixed").to_str().unwrap().into(),
            article_path: articles.to_str().unwrap().into(),
        };
        let error = run_with_config(&db, &config, true)
            .await
            .unwrap_err()
            .to_string();
        for expected in [
            "missing-fixed",
            "00-duplicate-a.md",
            "00-duplicate-b.md",
            "duplicate slug",
            "00-invalid-slug.md",
            "01-broken.md",
            "01-invalid.md",
            "02-date.md",
            "03-db.md",
            "rejected article",
        ] {
            assert!(error.contains(expected), "{error}");
        }
        let saved = article::Entity::find().all(&db).await.unwrap();
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].slug, "good");
        std::fs::remove_dir_all(root).unwrap();
    }
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
        let error = validate_input_slugs(&config)
            .values()
            .cloned()
            .collect::<Vec<_>>()
            .join("; ");
        assert!(
            error.contains("duplicate slug") && error.contains("a.md") && error.contains("b.md")
        );
        std::fs::remove_file(articles.join("b.md")).unwrap();
        assert!(validate_input_slugs(&config).is_empty());
        for directory in [&articles, &fixed] {
            for invalid in [
                document.replace("title: Test\n", ""),
                document.replace("title: Test", "title: []"),
            ] {
                std::fs::write(directory.join("b.md"), invalid).unwrap();
                let errors = validate_input_slugs(&config);
                for name in ["a.md", "b.md"] {
                    assert!(
                        errors
                            .get(&directory.join(name))
                            .unwrap()
                            .contains("duplicate slug")
                    );
                }
                std::fs::remove_file(directory.join("b.md")).unwrap();
            }
        }
        std::fs::write(
            articles.join("b.md"),
            document.replace("tags: []", "tags: false"),
        )
        .unwrap();
        let errors = validate_input_slugs(&config);
        assert!(errors.contains_key(&articles.join("a.md")));
        assert!(errors.contains_key(&articles.join("b.md")));
        std::fs::remove_file(articles.join("b.md")).unwrap();
        std::fs::write(
            fixed.join("a.md"),
            document.replace("slug: same", "slug: tags"),
        )
        .unwrap();
        assert!(
            validate_input_slugs(&config)
                .values()
                .cloned()
                .collect::<Vec<_>>()
                .join("; ")
                .contains("reserved route")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
