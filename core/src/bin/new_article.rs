use anyhow::{Context, Result, ensure};
use rust_blog::{
    seed::{
        config::env::load_env,
        markdown::{markdown_files, parse_markdown_slug},
    },
    slug,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};

#[derive(Serialize)]
struct Header<'a> {
    title: &'a str,
    slug: &'a str,
    created_at: String,
    tags: [String; 0],
    categories: [String; 0],
}

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [title, name] = args.as_slice() else {
        anyhow::bail!("Usage: new_article TITLE SLUG");
    };
    ensure!(
        !title.trim().is_empty() && title.encode_utf16().count() <= 50,
        "title must be 1–50 UTF-16 code units"
    );
    slug::validate(name, 100)?;
    let directory = load_env().article_path;
    fs::create_dir_all(&directory)?;
    let key = slug::collision_key(name);
    for path in markdown_files(&directory) {
        let path = path?;
        let existing = parse_markdown_slug(&path).with_context(|| path.display().to_string())?;
        ensure!(
            slug::collision_key(&existing) != key,
            "duplicate slug in {}",
            path.display()
        );
    }
    let header = Header {
        title,
        slug: name,
        created_at: chrono::Local::now().to_rfc3339(),
        tags: [],
        categories: [],
    };
    let yaml = serde_yaml::to_string(&header)?;
    let filename = if name.len() + ".md".len() <= 255 {
        format!("{name}.md")
    } else {
        format!("article-{:x}.md", Sha256::digest(key.as_bytes()))
    };
    let path = Path::new(&directory).join(filename);
    let mut file =
        fs::File::create_new(&path).with_context(|| format!("create {}", path.display()))?;
    writeln!(file, "---\n{yaml}---\n")?;
    println!("{}", path.display());
    Ok(())
}
