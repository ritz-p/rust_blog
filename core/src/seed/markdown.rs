use crate::utils::{self, fixed_content_matter::FixedContentMatter};
use std::fs;
use utils::front_matter::FrontMatter;
use walkdir::WalkDir;

pub fn markdown_files(dir: &str) -> impl Iterator<Item = std::path::PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.path().extension().and_then(|os| os.to_str()) == Some("md")
        })
        .map(|entry| entry.into_path())
}

pub fn parse_markdown_to_front_matter(
    path: &std::path::Path,
) -> Result<(FrontMatter, String), Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let parts: Vec<&str> = text.splitn(3, "---").collect();
    if parts.len() != 3 {
        panic!("FrontMatter not found in {:?}", path);
    }
    let front_matter = serde_yaml::from_str(parts[1])?;
    let body = parts[2].trim_start().to_string();
    Ok((front_matter, body))
}

pub fn parse_markdown_to_fixed_content_matter(
    path: &std::path::Path,
) -> Result<(FixedContentMatter, String), Box<dyn std::error::Error>> {
    let text = fs::read_to_string(path)?;
    let parts: Vec<&str> = text.splitn(3, "---").collect();
    if parts.len() != 3 {
        panic!("FrontMatter not found in {:?}", path);
    }
    let fixed_content_matter = serde_yaml::from_str(parts[1])?;
    let body = parts[2].trim_start().to_string();
    Ok((fixed_content_matter, body))
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
    fn markdown_files_collects_only_md_files_recursively() {
        let dir = create_temp_dir();
        let nested = dir.join("nested");
        fs::create_dir_all(&nested).expect("failed to create nested dir");
        fs::write(dir.join("a.md"), "# a").expect("failed to write a.md");
        fs::write(nested.join("b.md"), "# b").expect("failed to write b.md");
        fs::write(dir.join("c.txt"), "not markdown").expect("failed to write c.txt");

        let mut paths = markdown_files(dir.to_str().expect("invalid temp dir")).collect::<Vec<_>>();
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
    #[should_panic(expected = "FrontMatter not found")]
    fn parse_markdown_to_front_matter_panics_when_delimiter_missing() {
        let dir = create_temp_dir();
        let path = dir.join("broken.md");
        fs::write(&path, "title: no delimiter").expect("failed to write markdown file");
        let _ = parse_markdown_to_front_matter(&path);
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
