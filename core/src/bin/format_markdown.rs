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

fn format(text: &str) -> Result<String> {
    let newline = if text.starts_with("---\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut lines = text.split_inclusive('\n');
    ensure!(
        lines.next().map(str::trim_end) == Some("---"),
        "front matter must start with --- on the first line"
    );
    let start = 3 + newline.len();
    let mut end = start;
    let mut body_start = None;
    for line in lines {
        if line.trim_end_matches(['\r', '\n']) == "---" {
            body_start = Some(end + line.len());
            break;
        }
        end += line.len();
    }
    let body_start = body_start.context("closing --- delimiter is missing")?;
    let yaml = &text[start..end];
    let matter: FrontMatter = serde_yaml::from_str(yaml)?;
    seed::validate(&matter, "").map_err(|e| anyhow::anyhow!("{e}"))?;
    ensure!(
        !matter.title.trim().is_empty() && !matter.slug.trim().is_empty(),
        "title and slug must not be blank"
    );
    if let Some(date) = &matter.created_at {
        seed::parse_created_at(date)?;
    }
    for name in matter.tags.iter().chain(&matter.categories) {
        ensure!(
            !name.trim().is_empty() && name.encode_utf16().count() <= 50,
            "tags and categories must contain nonblank names of at most 50 characters"
        );
    }
    let mut source: Mapping = serde_yaml::from_str(yaml)?;
    if let Some(date) = source.remove(Value::from("date")) {
        source.insert(Value::from("created_at"), date);
    }
    let mut ordered = Mapping::new();
    for key in ORDER {
        if let Some(value) = source.remove(Value::from(*key)) {
            ordered.insert(Value::from(*key), value);
        }
    }
    ensure!(
        source.is_empty(),
        "unknown front matter fields: {:?}",
        source.keys().collect::<Vec<_>>()
    );
    let yaml = serde_yaml::to_string(&ordered)?.replace('\n', newline);
    Ok(format!(
        "---{newline}{yaml}---{newline}{}",
        &text[body_start..]
    ))
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
                let formatted = format(&text)?;
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

    #[test]
    fn orders_fields_preserves_body_and_is_idempotent() {
        let input = "---\ncategories: []\ntags: [rust]\nslug: test\ntitle: 'a --- title'\ndate: 2026-01-01\n---\n\n  body\n---\n";
        let output = format(input).unwrap();
        assert!(output.starts_with("---\ntitle: a --- title\nslug: test\ncreated_at:"));
        assert!(output.ends_with("---\n\n  body\n---\n"));
        assert_eq!(format(&output).unwrap(), output);
        let crlf = input.replace('\n', "\r\n");
        assert_eq!(format(&crlf).unwrap(), output.replace('\n', "\r\n"));
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
            assert!(format(&format!("---\n{yaml}---\nbody")).is_err());
        }
        assert!(format("body\n---\ntitle: Test\n---\n").is_err());
        assert!(format("---\ntitle: Test\n").is_err());
    }
}
