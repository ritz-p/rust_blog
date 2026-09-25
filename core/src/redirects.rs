use anyhow::{Context, Result, ensure};
use sea_orm::{DatabaseConnection, EntityTrait};
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::Path,
};

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RedirectMap {
    pub articles: BTreeMap<String, String>,
    pub pages: BTreeMap<String, String>,
}

impl RedirectMap {
    pub fn load(config: &HashMap<String, String>) -> Result<Self> {
        let path = config
            .get("redirect_map_path")
            .map(String::as_str)
            .unwrap_or("redirects.toml");
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(&text).with_context(|| format!("invalid redirect map {path}")),
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && !config.contains_key("redirect_map_path") =>
            {
                Ok(Self::default())
            }
            Err(error) => Err(error).with_context(|| format!("read redirect map {path}")),
        }
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut map: Self = toml::from_str(text)?;
        for (entries, fixed) in [(&mut map.articles, false), (&mut map.pages, true)] {
            let mut names = HashSet::new();
            for (source, target) in entries.iter() {
                for slug in [source, target] {
                    if fixed {
                        crate::slug::validate_fixed(slug)?;
                    } else {
                        crate::slug::validate(slug, 100)?;
                    }
                }
                ensure!(
                    names.insert(crate::slug::collision_key(source)),
                    "duplicate redirect source: {source}"
                );
            }
            let original = entries.clone();
            for (source, target) in entries.iter_mut() {
                let mut seen = HashSet::from([crate::slug::collision_key(source)]);
                loop {
                    ensure!(
                        seen.insert(crate::slug::collision_key(target)),
                        "redirect cycle at {source}"
                    );
                    match original.get(target) {
                        Some(next) => *target = next.clone(),
                        None => break,
                    }
                }
            }
        }
        Ok(map)
    }

    pub async fn validate_database(&self, db: &DatabaseConnection) -> Result<()> {
        for (map, slugs) in [
            (
                &self.articles,
                crate::entity::article::Entity::find()
                    .all(db)
                    .await?
                    .into_iter()
                    .map(|m| m.slug)
                    .collect::<Vec<_>>(),
            ),
            (
                &self.pages,
                crate::entity::fixed_content::Entity::find()
                    .all(db)
                    .await?
                    .into_iter()
                    .map(|m| m.slug)
                    .collect::<Vec<_>>(),
            ),
        ] {
            let keys: HashSet<_> = slugs
                .iter()
                .map(|s| crate::slug::collision_key(s))
                .collect();
            for (source, target) in map {
                ensure!(
                    !keys.contains(&crate::slug::collision_key(source)),
                    "redirect source still exists: {source}"
                );
                ensure!(
                    slugs.contains(target),
                    "redirect target does not exist: {target}"
                );
            }
        }
        Ok(())
    }

    pub async fn write_static(&self, db: &DatabaseConnection, out: &Path) -> Result<()> {
        let published: HashSet<_> = crate::repository::article::get_all_published_articles(db)
            .await?
            .into_iter()
            .map(|m| m.slug)
            .collect();
        let mut rules = String::new();
        for (map, prefix) in [(&self.articles, "posts/"), (&self.pages, "")] {
            for (source, target) in map {
                if prefix == "posts/" && !published.contains(target) {
                    continue;
                }
                let source_url = format!("/{prefix}{}", crate::utils::url_segment(source));
                let target_url = format!("/{prefix}{}/", crate::utils::url_segment(target));
                rules.push_str(&format!(
                    "{source_url} {target_url} 308\n{source_url}/ {target_url} 308\n"
                ));
                let directory = out.join(prefix).join(source);
                fs::create_dir_all(&directory)?;
                let target_html = html_escape::encode_double_quoted_attribute(&target_url);
                fs::write(
                    directory.join("index.html"),
                    format!(
                        "<!doctype html>\n<html lang=\"ja\"><head><meta charset=\"utf-8\"><meta name=\"robots\" content=\"noindex\"><meta http-equiv=\"refresh\" content=\"0;url={target_html}\"><link rel=\"canonical\" href=\"{target_html}\"><title>Moved</title></head><body><a href=\"{target_html}\">Moved</a></body></html>\n"
                    ),
                )?;
            }
        }
        // Explicit aliases precede generic trailing-slash rules.
        rules.push_str(&fs::read_to_string(out.join("_redirects"))?);
        fs::write(out.join("_redirects"), rules)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rocket::async_test]
    async fn renamed_markdown_exports_redirects_and_hides_future_targets() {
        use crate::entity::{article, article_category, article_tag, category, fixed_content, tag};
        use sea_orm::{ConnectionTrait, Database, Schema, Set};
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        for table in [
            schema.create_table_from_entity(article::Entity),
            schema.create_table_from_entity(article_tag::Entity),
            schema.create_table_from_entity(article_category::Entity),
            schema.create_table_from_entity(tag::Entity),
            schema.create_table_from_entity(category::Entity),
            schema.create_table_from_entity(fixed_content::Entity),
        ] {
            db.execute(backend.build(&table)).await.unwrap();
        }
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/redirects");
        let (mut matter, body) =
            crate::seed::markdown::parse_markdown_to_front_matter(&fixture.join("renamed.md"))
                .unwrap();
        let id = crate::seed::article::seed_article(&db, &matter, &body)
            .await
            .unwrap();
        let map = RedirectMap::parse(&fs::read_to_string(fixture.join("redirects.toml")).unwrap())
            .unwrap();
        map.validate_database(&db).await.unwrap();
        assert!(
            RedirectMap::parse("[articles]\nold = 'missing'")
                .unwrap()
                .validate_database(&db)
                .await
                .is_err()
        );
        let out = std::env::temp_dir().join(format!(
            "redirect-export-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let paths = crate::static_site::ExportPaths {
            templates_dir: Path::new(env!("CARGO_MANIFEST_DIR")).join("../templates"),
            content_dir: fixture.join("content"),
        };
        let config = HashMap::from([(
            "redirect_map_path".into(),
            fixture
                .join("redirects.toml")
                .to_string_lossy()
                .into_owned(),
        )]);
        crate::static_site::export_site(&db, &config, &out, &paths)
            .await
            .unwrap();
        let rules = fs::read_to_string(out.join("_redirects")).unwrap();
        assert!(rules.starts_with("/posts/old-article /posts/new-%E8%A8%98%E4%BA%8B/ 308\n"));
        assert!(rules.contains("/posts/older-article/ /posts/new-%E8%A8%98%E4%BA%8B/ 308"));
        assert!(
            fs::read_to_string(out.join("posts/old-article/index.html"))
                .unwrap()
                .contains("0;url=/posts/new-%E8%A8%98%E4%BA%8B/")
        );
        article::Entity::update(article::ActiveModel {
            id: Set(id),
            created_at: Set(chrono::Utc::now() + chrono::Duration::days(1)),
            ..Default::default()
        })
        .exec(&db)
        .await
        .unwrap();
        crate::static_site::export_site(&db, &config, &out, &paths)
            .await
            .unwrap();
        assert!(!out.join("posts/old-article/index.html").exists());
        assert!(
            !fs::read_to_string(out.join("_redirects"))
                .unwrap()
                .contains("old-article")
        );
        matter.slug = "old-article".into();
        crate::seed::article::seed_article(&db, &matter, &body)
            .await
            .unwrap();
        fs::write(out.join("keep.txt"), "previous output").unwrap();
        assert!(
            crate::static_site::export_site(&db, &config, &out, &paths)
                .await
                .is_err()
        );
        assert!(out.join("keep.txt").exists());
        fs::remove_dir_all(out).unwrap();
    }

    #[test]
    fn rejects_cycles_unsafe_paths_and_ambiguous_sources() {
        for text in [
            "[articles]\na = 'a'",
            "[articles]\na = 'b'\nb = 'a'",
            "[articles]\n'../old' = 'new'",
            "[articles]\na = 'https://example.com'",
            "[articles]\na = 'new'\nA = 'new'",
            "[pages]\nposts = 'about'",
        ] {
            assert!(RedirectMap::parse(text).is_err(), "{text}");
        }
        let map = RedirectMap::parse("[articles]\na = 'b'\nb = 'new'").unwrap();
        assert_eq!(map.articles["a"], "new");
    }
}
