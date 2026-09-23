use anyhow::{Context, Result, bail, ensure};
use rust_blog::entity::{article, category, fixed_content, tag};
use sea_orm::{ConnectionTrait, Database, EntityTrait, ModelTrait, QueryOrder, TransactionTrait};
use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Serialize)]
struct ArticleMatter<'a> {
    title: &'a str,
    slug: &'a str,
    table_of_contents: bool,
    created_at: String,
    excerpt: &'a Option<String>,
    icatch_path: &'a Option<String>,
    tags: Vec<String>,
    categories: Vec<String>,
}

#[derive(Serialize)]
struct FixedMatter<'a> {
    title: &'a str,
    slug: &'a str,
    excerpt: &'a Option<String>,
}

fn markdown(matter: &impl Serialize, body: &str) -> Result<String> {
    Ok(format!(
        "---\n{}---\n{body}",
        serde_yaml::to_string(matter)?
    ))
}

async fn documents(db: &impl ConnectionTrait) -> Result<Vec<(PathBuf, String)>> {
    let mut output = Vec::new();
    for model in article::Entity::find()
        .order_by_asc(article::Column::Id)
        .all(db)
        .await?
    {
        let tags = model
            .find_related(tag::Entity)
            .order_by_asc(tag::Column::Id)
            .all(db)
            .await?
            .into_iter()
            .map(|tag| tag.name)
            .collect();
        let categories = model
            .find_related(category::Entity)
            .order_by_asc(category::Column::Id)
            .all(db)
            .await?
            .into_iter()
            .map(|category| category.name)
            .collect();
        let matter = ArticleMatter {
            title: &model.title,
            slug: &model.slug,
            table_of_contents: model.table_of_contents,
            created_at: model.created_at.to_rfc3339(),
            excerpt: &model.excerpt,
            icatch_path: &model.icatch_path,
            tags,
            categories,
        };
        output.push((
            PathBuf::from(format!("articles/{}.md", model.id)),
            markdown(&matter, &model.content)?,
        ));
    }
    for model in fixed_content::Entity::find()
        .order_by_asc(fixed_content::Column::Id)
        .all(db)
        .await?
    {
        let matter = FixedMatter {
            title: &model.title,
            slug: &model.slug,
            excerpt: &model.excerpt,
        };
        output.push((
            PathBuf::from(format!("fixed_contents/{}.md", model.id)),
            markdown(&matter, &model.content)?,
        ));
    }
    Ok(output)
}

fn stale_documents(root: &Path, documents: &[(PathBuf, String)]) -> Result<Vec<PathBuf>> {
    let mut stale = Vec::new();
    for directory in ["articles", "fixed_contents"] {
        let path = root.join(directory);
        let entries = match fs::read_dir(&path) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error).with_context(|| path.display().to_string()),
        };
        ensure!(
            !fs::symlink_metadata(&path)?.file_type().is_symlink(),
            "{} must not be a symlink",
            path.display()
        );
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            let managed = path.extension().is_some_and(|extension| extension == "md")
                && path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .is_some_and(|stem| stem.parse::<i32>().is_ok_and(|id| id.to_string() == stem));
            let relative = PathBuf::from(directory).join(entry.file_name());
            if managed
                && entry.file_type()?.is_file()
                && !documents.iter().any(|(current, _)| current == &relative)
            {
                stale.push(path);
            }
        }
    }
    Ok(stale)
}

fn write_documents(root: &Path, documents: &[(PathBuf, String)], force: bool) -> Result<()> {
    let stale = stale_documents(root, documents)?;
    ensure!(
        force || stale.is_empty(),
        "stale exported Markdown files exist; use --force to replace the export"
    );
    for (relative, _) in documents {
        let path = root.join(relative);
        ensure!(
            force || !path.try_exists()?,
            "{} already exists; use --force to overwrite",
            path.display()
        );
    }
    for (relative, text) in documents {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().context("missing parent directory")?)?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(force)
            .truncate(force)
            .create_new(!force)
            .open(&path)
            .with_context(|| path.display().to_string())?;
        file.write_all(text.as_bytes())?;
    }
    for path in stale {
        fs::remove_file(&path).with_context(|| path.display().to_string())?;
    }
    Ok(())
}

#[rocket::main]
async fn main() -> Result<()> {
    let mut root = None;
    let mut force = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--force" => force = true,
            "--help" | "-h" => {
                println!(
                    "Usage: export_markdown [--force] [OUTPUT_DIRECTORY]\nDefault: markdown_output. Reads DATABASE_URL.\n--force overwrites current files and removes stale articles/<id>.md and fixed_contents/<id>.md files."
                );
                return Ok(());
            }
            _ if arg.starts_with('-') => bail!("unknown option: {arg}"),
            _ => {
                ensure!(root.is_none(), "only one output directory is allowed");
                root = Some(PathBuf::from(arg));
            }
        }
    }
    let root = root.unwrap_or_else(|| "markdown_output".into());
    let url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let db = Database::connect(url).await?;
    let transaction = db.begin().await?;
    let documents = documents(&transaction).await?;
    transaction.commit().await?;
    write_documents(&root, &documents, force)?;
    println!(
        "Exported {} Markdown files to {}",
        documents.len(),
        root.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_blog::{
        seed::article::{seed_article, seed_category, seed_tag},
        utils::front_matter::FrontMatter,
    };
    use sea_orm::{ActiveModelTrait, Schema, Set};

    #[rocket::async_test]
    async fn exports_seedable_articles_with_relations_and_fixed_pages() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        for statement in [
            schema.create_table_from_entity(article::Entity),
            schema.create_table_from_entity(tag::Entity),
            schema.create_table_from_entity(category::Entity),
            schema.create_table_from_entity(rust_blog::entity::article_tag::Entity),
            schema.create_table_from_entity(rust_blog::entity::article_category::Entity),
            schema.create_table_from_entity(fixed_content::Entity),
        ] {
            db.execute(backend.build(&statement)).await.unwrap();
        }
        assert!(documents(&db).await.unwrap().is_empty());
        let matter: FrontMatter = serde_yaml::from_str("title: 'Test: 日本語 --- test'\nslug: test-export\ntable_of_contents: true\ncreated_at: 2026-01-01T00:00:00Z\ntags: [Rust]\ncategories: [Dev]\n").unwrap();
        let body = "\n# Body\n---\n```rust\nfn main() {}\n```\n";
        let id = seed_article(&db, &matter, body).await.unwrap();
        seed_tag(&db, &matter, id).await.unwrap();
        seed_category(&db, &matter, id).await.unwrap();
        fixed_content::ActiveModel {
            title: Set("About".into()),
            slug: Set("about".into()),
            content: Set("About body".into()),
            created_at: Set(chrono::Utc::now()),
            updated_at: Set(chrono::Utc::now()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let output = documents(&db).await.unwrap();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0].0, PathBuf::from(format!("articles/{id}.md")));
        let (_, rest) = output[0].1.split_once("---\n").unwrap();
        let (yaml, exported_body) = rest.split_once("\n---\n").unwrap();
        let parsed: FrontMatter = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(parsed.title, matter.title);
        assert_eq!(parsed.slug, matter.slug);
        assert_eq!(parsed.tags, matter.tags);
        assert_eq!(parsed.categories, matter.categories);
        assert_eq!(
            parsed.created_at.as_deref(),
            Some("2026-01-01T00:00:00+00:00")
        );
        assert!(parsed.table_of_contents);
        assert_eq!(exported_body, body);
        assert!(output[1].1.ends_with("---\nAbout body"));
        let root = std::env::temp_dir().join(format!(
            "export-markdown-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        write_documents(&root, &output, false).unwrap();
        let (reparsed, reparsed_body) =
            rust_blog::seed::markdown::parse_markdown_to_front_matter(&root.join(&output[0].0))
                .unwrap();
        assert_eq!(reparsed.title, matter.title);
        assert_eq!(reparsed_body, body.trim_start());
        assert_eq!(
            seed_article(&db, &reparsed, &reparsed_body).await.unwrap(),
            id
        );
        let (fixed, fixed_body) =
            rust_blog::seed::markdown::parse_markdown_to_fixed_content_matter(
                &root.join(&output[1].0),
            )
            .unwrap();
        assert_eq!(fixed.title, "About");
        assert_eq!(fixed_body, "About body");
        fs::write(root.join(&output[1].0), "keep").unwrap();
        assert!(write_documents(&root, &output, false).is_err());
        assert_eq!(fs::read_to_string(root.join(&output[1].0)).unwrap(), "keep");
        write_documents(&root, &output, true).unwrap();
        assert_eq!(
            fs::read_to_string(root.join(&output[1].0)).unwrap(),
            output[1].1
        );
        fs::write(root.join("articles/notes.md"), "unmanaged").unwrap();
        fs::write(root.join("articles/10.png"), "asset").unwrap();
        fs::write(root.join("README.md"), "instructions").unwrap();
        rust_blog::entity::article_tag::Entity::delete_many()
            .exec(&db)
            .await
            .unwrap();
        rust_blog::entity::article_category::Entity::delete_many()
            .exec(&db)
            .await
            .unwrap();
        article::Entity::delete_many().exec(&db).await.unwrap();
        let remaining = documents(&db).await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert!(write_documents(&root, &remaining, false).is_err());
        assert!(root.join(&output[0].0).exists());
        write_documents(&root, &remaining, true).unwrap();
        assert!(!root.join(&output[0].0).exists());
        assert_eq!(
            fs::read_to_string(root.join(&output[1].0)).unwrap(),
            output[1].1
        );
        fixed_content::Entity::delete_many()
            .exec(&db)
            .await
            .unwrap();
        let empty = documents(&db).await.unwrap();
        assert!(empty.is_empty());
        assert!(write_documents(&root, &empty, false).is_err());
        assert!(root.join(&output[1].0).exists());
        write_documents(&root, &empty, true).unwrap();
        assert!(!root.join(&output[1].0).exists());
        assert_eq!(
            fs::read_to_string(root.join("articles/notes.md")).unwrap(),
            "unmanaged"
        );
        assert_eq!(
            fs::read_to_string(root.join("articles/10.png")).unwrap(),
            "asset"
        );
        assert_eq!(
            fs::read_to_string(root.join("README.md")).unwrap(),
            "instructions"
        );
        write_documents(&root, &empty, true).unwrap();
        fs::remove_dir_all(root).unwrap();
    }
}
