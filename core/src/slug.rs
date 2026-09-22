use anyhow::{Result, ensure};
use std::{collections::HashMap, path::PathBuf};

pub fn validate(slug: &str, max: usize) -> Result<()> {
    ensure!(
        (1..=max).contains(&slug.encode_utf16().count()),
        "slug must contain 1 to {max} UTF-16 code units"
    );
    ensure!(
        !slug.trim().is_empty() && slug.trim() == slug,
        "slug must not be blank or have surrounding whitespace"
    );
    ensure!(
        slug != "." && slug != ".." && !slug.ends_with('.'),
        "slug must not be a dot path or end with a dot"
    );
    ensure!(
        !slug
            .chars()
            .any(|c| c.is_control() || "/\\:*?\"<>|".contains(c)),
        "slug contains a path separator, control character, or unsupported filename character"
    );
    let stem = slug.split('.').next().unwrap_or(slug).to_ascii_uppercase();
    let device = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix)
                .is_some_and(|n| matches!(n, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"))
        });
    ensure!(!device, "slug is a reserved filename");
    Ok(())
}

pub fn validate_fixed(slug: &str) -> Result<()> {
    validate(slug, 100)?;
    ensure!(
        ![
            "posts",
            "post",
            "page",
            "archive",
            "tags",
            "tag",
            "categories",
            "category",
            "css",
            "js",
            "image",
            "icon",
            "index.html",
            "404.html",
            "search-index.json",
            "_headers",
            "_redirects"
        ]
        .contains(&slug.to_lowercase().as_str()),
        "fixed page slug conflicts with a reserved route or output file: {slug}"
    );
    Ok(())
}

pub(crate) async fn validate_database_collision(
    db: &sea_orm::DatabaseConnection,
    table: &str,
    slug: &str,
) -> Result<()> {
    use sea_orm::{ConnectionTrait, Statement};
    ensure!(
        matches!(table, "article" | "fixed_content"),
        "unsupported slug namespace"
    );
    let rows = db
        .query_all(Statement::from_string(
            db.get_database_backend(),
            format!("SELECT slug FROM {table}"),
        ))
        .await?;
    let key = slug.to_lowercase();
    for row in rows {
        let existing: String = row.try_get("", "slug")?;
        ensure!(
            existing == slug || existing.to_lowercase() != key,
            "slug {slug:?} conflicts with existing {table} slug {existing:?}"
        );
    }
    Ok(())
}

#[derive(Default)]
pub struct SlugRegistry {
    paths: HashMap<String, PathBuf>,
}

impl SlugRegistry {
    pub fn insert(&mut self, slug: &str, path: impl Into<PathBuf>) -> Result<()> {
        let path = path.into();
        let key = slug.to_lowercase();
        if let Some(previous) = self.paths.get(&key) {
            ensure!(
                previous == &path,
                "duplicate slug {slug:?}: {} and {}",
                previous.display(),
                path.display()
            );
        } else {
            self.paths.insert(key, path);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rocket::async_test]
    async fn existing_slug_can_be_updated_but_case_collision_is_rejected() {
        use sea_orm::{ConnectionTrait, Database, Statement};
        let db = Database::connect("sqlite::memory:").await.unwrap();
        for sql in [
            "CREATE TABLE article (slug TEXT)",
            "INSERT INTO article VALUES ('Rust')",
        ] {
            db.execute(Statement::from_string(db.get_database_backend(), sql))
                .await
                .unwrap();
        }
        validate_database_collision(&db, "article", "Rust")
            .await
            .unwrap();
        validate_database_collision(&db, "article", "new")
            .await
            .unwrap();
        assert!(
            validate_database_collision(&db, "article", "rust")
                .await
                .is_err()
        );
    }

    #[test]
    fn validates_portable_single_path_segments() {
        for slug in [
            "", " ", " foo", "foo ", ".", "..", "../foo", "a/b", "a\\b", "a\nb", "a:b", "CON",
            "nul.txt", "LPT1", "last.",
        ] {
            assert!(validate(slug, 100).is_err(), "{slug:?}");
        }
        for slug in ["rust-blog", "C# 100%", "日本語", "v1.0", "COM10"] {
            assert!(validate(slug, 100).is_ok(), "{slug}");
        }
        assert!(validate(&"a".repeat(100), 100).is_ok());
        assert!(validate(&"a".repeat(101), 100).is_err());
        assert!(validate(&"😀".repeat(50), 100).is_ok());
        assert!(validate(&"😀".repeat(51), 100).is_err());
        assert!(validate_fixed("POSTS").is_err());
        assert!(validate_fixed("about").is_ok());
    }

    #[test]
    fn detects_duplicates_without_rejecting_the_same_file() {
        let mut registry = SlugRegistry::default();
        registry.insert("rust", "a.md").unwrap();
        registry.insert("rust", "a.md").unwrap();
        let error = registry.insert("Rust", "b.md").unwrap_err().to_string();
        assert!(error.contains("a.md") && error.contains("b.md"));
    }
}
