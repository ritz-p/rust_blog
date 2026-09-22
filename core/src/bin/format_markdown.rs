use anyhow::{Context, Result, bail, ensure};
use rust_blog::{seed::article::seed, utils::front_matter::FrontMatter};
use serde_yaml::{Mapping, Value};
use std::{fs, path::PathBuf};
use walkdir::WalkDir;

const ORDER: &[&str] = &[
    "title",
    "slug",
    "deleted",
    "table_of_contents",
    "created_at",
    "excerpt",
    "icatch_path",
    "tags",
    "categories",
];

struct Document<'a> {
    matter: Mapping,
    body: &'a str,
    newline: &'a str,
}

fn parse(text: &str) -> Result<Document<'_>> {
    let newline = if text.starts_with("---\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines = text.split_inclusive('\n');
    let first = lines.next().context("front matter is missing")?;
    ensure!(
        first.trim_end_matches(['\r', '\n']) == "---",
        "front matter must start with --- on the first line"
    );
    let start = first.len();
    let mut end = start;
    for line in lines {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return Ok(Document {
                matter: serde_yaml::from_str(&text[start..end])?,
                body: &text[end + line.len()..],
                newline,
            });
        }
        end += line.len();
    }
    bail!("closing --- delimiter is missing")
}

fn validate_length(field: &str, value: &str, max: usize) -> Result<()> {
    ensure!(
        (1..=max).contains(&value.encode_utf16().count()),
        "{field} must contain 1 to {max} UTF-16 code units"
    );
    Ok(())
}

fn validate_title(title: &str) -> Result<()> {
    ensure!(!title.trim().is_empty(), "title must not be blank");
    validate_length("title", title, 50)
}

fn validate_slug(slug: &str) -> Result<()> {
    ensure!(!slug.trim().is_empty(), "slug must not be blank");
    validate_length("slug", slug, 100)
}

fn validate_created_at(created_at: Option<&str>) -> Result<()> {
    if let Some(value) = created_at {
        seed::parse_created_at(value)?;
    }
    Ok(())
}

fn validate_excerpt(excerpt: Option<&str>) -> Result<()> {
    if let Some(value) = excerpt {
        validate_length("excerpt", value, 100)?;
    }
    Ok(())
}

fn validate_icatch_path(icatch_path: Option<&str>) -> Result<()> {
    if let Some(value) = icatch_path {
        validate_length("icatch_path", value, 200)?;
    }
    Ok(())
}

fn validate_names(field: &str, names: &[String]) -> Result<()> {
    for name in names {
        ensure!(
            !name.trim().is_empty(),
            "{field} must not contain blank names"
        );
        validate_length(field, name, 50)?;
    }
    Ok(())
}

fn validate_tags(tags: &[String]) -> Result<()> {
    validate_names("tags", tags)
}

fn validate_categories(categories: &[String]) -> Result<()> {
    validate_names("categories", categories)
}

fn validate_fields(source: &Mapping) -> Result<()> {
    for key in source.keys() {
        ensure!(
            key.as_str()
                .is_some_and(|key| ORDER.contains(&key) || key == "date"),
            "unknown front matter field: {key:?}"
        );
    }
    Ok(())
}

fn validate(source: &Mapping) -> Result<()> {
    validate_fields(source)?;
    let matter: FrontMatter = serde_yaml::from_value(Value::Mapping(source.clone()))?;
    validate_title(&matter.title)?;
    validate_slug(&matter.slug)?;
    validate_created_at(matter.created_at.as_deref())?;
    validate_excerpt(matter.excerpt.as_deref())?;
    validate_icatch_path(matter.icatch_path.as_deref())?;
    validate_tags(&matter.tags)?;
    validate_categories(&matter.categories)?;
    Ok(())
}

fn format(source: &Mapping) -> Mapping {
    let mut entries: Vec<_> = source.clone().into_iter().collect();
    entries.sort_by_key(|(key, _)| {
        let key = match key.as_str() {
            Some("date") => Some("created_at"),
            other => other,
        };
        ORDER
            .iter()
            .position(|field| Some(*field) == key)
            .unwrap_or(ORDER.len())
    });
    entries.into_iter().collect()
}

fn render(document: &Document<'_>, matter: &Mapping) -> Result<String> {
    let newline = document.newline;
    let yaml = serde_yaml::to_string(matter)?.replace('\n', newline);
    Ok(format!("---{newline}{yaml}---{newline}{}", document.body))
}

fn main() -> Result<()> {
    let mut check = false;
    let mut roots = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--check" => check = true,
            "--help" | "-h" => {
                println!(
                    "Usage: format_markdown [--check] [FILE_OR_DIRECTORY ...]\nDefault: content/articles. --check validates and checks formatting without writing."
                );
                return Ok(());
            }
            _ if arg.starts_with('-') => bail!("unknown option: {arg}"),
            _ => roots.push(PathBuf::from(arg)),
        }
    }
    if roots.is_empty() {
        roots.push("content/articles".into());
    }
    let mut paths = Vec::new();
    for root in roots {
        for entry in WalkDir::new(root) {
            let entry = entry?;
            if entry.file_type().is_file() && entry.path().extension().is_some_and(|x| x == "md") {
                paths.push(entry.into_path());
            }
        }
    }
    paths.sort();
    paths.dedup();
    ensure!(!paths.is_empty(), "no Markdown files found");
    let mut pending = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        let result = fs::read_to_string(&path)
            .map_err(anyhow::Error::from)
            .and_then(|text| {
                let document = parse(&text)?;
                validate(&document.matter)?;
                let formatted = render(&document, &format(&document.matter))?;
                Ok((text, formatted))
            });
        match result {
            Ok((text, formatted)) if text != formatted => pending.push((path, formatted)),
            Ok(_) => {}
            Err(error) => errors.push(format!("{}: {error:#}", path.display())),
        }
    }
    ensure!(errors.is_empty(), "{}", errors.join("\n"));
    if check {
        for (path, _) in &pending {
            eprintln!("{}: formatting required", path.display());
        }
        ensure!(pending.is_empty(), "front matter formatting required");
    } else {
        for (path, formatted) in pending {
            fs::write(&path, formatted).with_context(|| path.display().to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checked_format(text: &str) -> Result<String> {
        let document = parse(text)?;
        validate(&document.matter)?;
        render(&document, &format(&document.matter))
    }

    #[test]
    fn format_only_reorders_without_validation_or_renaming() {
        let source: Mapping =
            serde_yaml::from_str("date: invalid\nslug: ''\ntitle: ''\ncustom: keep\n").unwrap();
        assert!(validate(&source).is_err());
        let ordered = format(&source);
        assert_eq!(ordered.len(), source.len());
        for (key, value) in &source {
            assert_eq!(ordered.get(key), Some(value));
        }
        let keys: Vec<_> = ordered.keys().map(|key| key.as_str().unwrap()).collect();
        assert_eq!(keys, ["title", "slug", "date", "custom"]);
    }

    #[test]
    fn validates_field_limits_and_optional_values() {
        assert!(validate_title(&"a".repeat(50)).is_ok());
        assert!(validate_title(&"a".repeat(51)).is_err());
        assert!(validate_title(" ").is_err());
        assert!(validate_slug(&"a".repeat(100)).is_ok());
        assert!(validate_slug(&"a".repeat(101)).is_err());
        assert!(validate_slug(" ").is_err());
        assert!(validate_excerpt(None).is_ok());
        assert!(validate_excerpt(Some("")).is_err());
        assert!(validate_excerpt(Some(&"a".repeat(101))).is_err());
        assert!(validate_icatch_path(None).is_ok());
        assert!(validate_icatch_path(Some(&"a".repeat(201))).is_err());
        assert!(validate_created_at(None).is_ok());
        assert!(validate_created_at(Some("2026-01-01")).is_ok());
        assert!(validate_created_at(Some("2026-02-30")).is_err());
        assert!(validate_tags(&[]).is_ok());
        assert!(validate_tags(&[" ".into()]).is_err());
        assert!(validate_categories(&["a".repeat(51)]).is_err());
    }

    #[test]
    fn orders_fields_preserves_body_and_is_idempotent() {
        let input = "---\ncategories: []\ntags: [rust]\nslug: test\ntitle: 'a --- title'\ndate: 2026-01-01\n---\n\n  body\n---\n";
        let output = checked_format(input).unwrap();
        assert!(output.starts_with("---\ntitle: a --- title\nslug: test\ndate:"));
        assert!(output.ends_with("---\n\n  body\n---\n"));
        assert_eq!(checked_format(&output).unwrap(), output);
        let crlf = input.replace('\n', "\r\n");
        assert_eq!(checked_format(&crlf).unwrap(), output.replace('\n', "\r\n"));
    }

    #[test]
    fn rejects_missing_invalid_and_unknown_fields() {
        let base = "title: Test\nslug: test\ntags: []\ncategories: []\n";
        for yaml in [
            base.replace("title: Test\n", ""),
            format!("{base}created_at: yesterday\n"),
            format!("{base}typo: true\n"),
            format!("{base}table_of_contents: nope\n"),
            base.replace("tags: []", "tags: ['']"),
            format!("{base}title: Duplicate\n"),
        ] {
            assert!(checked_format(&format!("---\n{yaml}---\nbody")).is_err());
        }
        assert!(checked_format("body\n---\ntitle: Test\n---\n").is_err());
        assert!(checked_format("---\ntitle: Test\n").is_err());
    }
}
