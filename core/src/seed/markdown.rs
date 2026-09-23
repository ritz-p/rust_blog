use crate::utils::{self, fixed_content_matter::FixedContentMatter};
use std::fs;
use utils::front_matter::FrontMatter;
use walkdir::WalkDir;

pub fn markdown_files(
    dir: &str,
) -> impl Iterator<Item = Result<std::path::PathBuf, walkdir::Error>> {
    WalkDir::new(dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|entry| match entry {
            Ok(entry)
                if entry.file_type().is_file()
                    && entry.path().extension().and_then(|os| os.to_str()) == Some("md") =>
            {
                Some(Ok(entry.into_path()))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
}

pub(super) fn parse_markdown_slug(path: &std::path::Path) -> anyhow::Result<String> {
    use anyhow::Context;
    let text = fs::read_to_string(path)?;
    let (yaml, _) = split_front_matter(&text)?;
    let matter: serde_yaml::Mapping = serde_yaml::from_str(yaml)?;
    let slug = matter
        .get(serde_yaml::Value::String("slug".into()))
        .context("slug is missing")?;
    serde_yaml::from_value(slug.clone()).context("slug must be a string")
}

pub fn parse_markdown_to_front_matter(
    path: &std::path::Path,
) -> Result<(FrontMatter, String), Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let (yaml, body) = split_front_matter(&text)?;
    let front_matter = serde_yaml::from_str(yaml)?;
    let body = body.trim_start().to_string();
    Ok((front_matter, body))
}

pub fn parse_markdown_to_fixed_content_matter(
    path: &std::path::Path,
) -> Result<(FixedContentMatter, String), Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let (yaml, body) = split_front_matter(&text)?;
    let fixed_content_matter = serde_yaml::from_str(yaml)?;
    let body = body.trim_start().to_string();
    Ok((fixed_content_matter, body))
}

fn split_front_matter(text: &str) -> Result<(&str, &str), std::io::Error> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let missing = || std::io::Error::new(std::io::ErrorKind::InvalidData, "FrontMatter not found");
    let mut lines = text.split_inclusive('\n');
    let first = lines.next().ok_or_else(missing)?;
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return Err(missing());
    }
    let start = first.len();
    let mut end = start;
    for line in lines {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return Ok((&text[start..end], &text[end + line.len()..]));
        }
        end += line.len();
    }
    Err(missing())
}

#[cfg(test)]
mod tests {
    use super::{
        markdown_files, parse_markdown_to_fixed_content_matter, parse_markdown_to_front_matter,
    };
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_temp_dir() -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("seed_markdown_test_{}_{}", std::process::id(), ts));
        fs::create_dir_all(&dir).expect("failed to create temp dir");
        dir
    }

    #[test]
    fn accepts_optional_bom_for_articles_and_fixed_pages() {
        let dir = create_temp_dir();
        let path = dir.join("bom.md");
        for newline in ["\n", "\r\n"] {
            let text =
                "\u{feff}---\ntitle: Test\nslug: test\ntags: []\ncategories: []\n---\n本文\n"
                    .replace('\n', newline);
            fs::write(&path, text).unwrap();
            let (article, body) = parse_markdown_to_front_matter(&path).unwrap();
            assert_eq!(article.title, "Test");
            assert_eq!(body, format!("本文{newline}"));
            let (fixed, body) = parse_markdown_to_fixed_content_matter(&path).unwrap();
            assert_eq!(fixed.title, "Test");
            assert_eq!(body, format!("本文{newline}"));
        }
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn markdown_files_collects_only_md_files_recursively() {
        let dir = create_temp_dir();
        let nested = dir.join("nested");
        fs::create_dir_all(&nested).expect("failed to create nested dir");
        fs::write(dir.join("a.md"), "# a").expect("failed to write a.md");
        fs::write(nested.join("b.md"), "# b").expect("failed to write b.md");
        fs::write(dir.join("c.txt"), "not markdown").expect("failed to write c.txt");

        let mut paths = markdown_files(dir.to_str().expect("invalid temp dir"))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        paths.sort();
        assert_eq!(paths.len(), 2);
        assert!(
            paths
                .iter()
                .all(|p| p.extension().and_then(|x| x.to_str()) == Some("md"))
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_markdown_to_front_matter_parses_yaml_and_body() {
        let dir = create_temp_dir();
        let path = dir.join("article.md");
        fs::write(
            &path,
            r#"---
title: "Test title"
slug: "test-slug"
excerpt: "short"
icatch_path: null
tags:
  - rust
categories:
  - dev
---
Body **text**
"#,
        )
        .expect("failed to write markdown file");

        let (front_matter, body) =
            parse_markdown_to_front_matter(&path).expect("failed to parse front matter");
        assert_eq!(front_matter.title, "Test title");
        assert_eq!(front_matter.slug, "test-slug");
        assert!(!front_matter.deleted);
        assert_eq!(front_matter.excerpt, Some("short".to_string()));
        assert_eq!(front_matter.tags, vec!["rust".to_string()]);
        assert_eq!(front_matter.categories, vec!["dev".to_string()]);
        assert_eq!(body, "Body **text**\n");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_markdown_to_front_matter_accepts_date_alias_for_created_at() {
        let dir = create_temp_dir();
        let path = dir.join("article.md");
        fs::write(
            &path,
            r#"---
title: "Test title"
slug: "test-slug"
date: 2021-08-01
tags:
  - rust
categories:
  - dev
---
Body **text**
"#,
        )
        .expect("failed to write markdown file");

        let (front_matter, body) =
            parse_markdown_to_front_matter(&path).expect("failed to parse front matter");
        assert_eq!(front_matter.created_at, Some("2021-08-01".to_string()));
        assert_eq!(body, "Body **text**\n");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_markdown_to_front_matter_allows_missing_date_and_created_at() {
        let dir = create_temp_dir();
        let path = dir.join("article.md");
        fs::write(
            &path,
            r#"---
title: "Test title"
slug: "test-slug"
tags:
  - rust
categories:
  - dev
---
Body **text**
"#,
        )
        .expect("failed to write markdown file");

        let (front_matter, body) =
            parse_markdown_to_front_matter(&path).expect("failed to parse front matter");
        assert_eq!(front_matter.created_at, None);
        assert_eq!(body, "Body **text**\n");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_markdown_to_fixed_content_matter_parses_yaml_and_body() {
        let dir = create_temp_dir();
        let path = dir.join("about.md");
        fs::write(
            &path,
            r#"---
title: "About"
slug: "about"
excerpt: "About page"
---
Hello world
"#,
        )
        .expect("failed to write markdown file");

        let (matter, body) = parse_markdown_to_fixed_content_matter(&path)
            .expect("failed to parse fixed content matter");
        assert_eq!(matter.title, "About");
        assert_eq!(matter.slug, "about");
        assert_eq!(matter.excerpt, Some("About page".to_string()));
        assert_eq!(body, "Hello world\n");

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_markdown_to_front_matter_errors_when_delimiter_missing() {
        let dir = create_temp_dir();
        let path = dir.join("broken.md");
        fs::write(&path, "title: no delimiter").expect("failed to write markdown file");
        assert!(parse_markdown_to_front_matter(&path).is_err());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn parse_markdown_to_front_matter_errors_on_invalid_yaml() {
        let dir = create_temp_dir();
        let path = dir.join("invalid.md");
        fs::write(
            &path,
            r#"---
title: [invalid
slug: "invalid"
tags: ["rust"]
categories: ["dev"]
---
text
"#,
        )
        .expect("failed to write markdown file");
        let result = parse_markdown_to_front_matter(&path);
        assert!(result.is_err());
        let _ = fs::remove_dir_all(dir);
    }
}
