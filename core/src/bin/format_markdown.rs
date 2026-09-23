use anyhow::{Context, Result, bail, ensure};
use rust_blog::seed::article::seed;
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
    bom: &'a str,
    matter: Mapping,
    body: &'a str,
    newline: &'a str,
}

fn parse(text: &str) -> Result<Document<'_>> {
    let (bom, text) = match text.strip_prefix('\u{feff}') {
        Some(text) => ("\u{feff}", text),
        None => ("", text),
    };
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
                bom,
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
    rust_blog::slug::validate(slug, 100)
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

fn validate_field<T: serde::de::DeserializeOwned>(
    source: &Mapping,
    field: &str,
    default: Option<Value>,
    validate: impl FnOnce(T) -> Result<()>,
) -> Result<()> {
    let value = source
        .get(Value::String(field.into()))
        .cloned()
        .or(default)
        .with_context(|| format!("{field}: required field is missing"))?;
    let value = serde_yaml::from_value(value).with_context(|| format!("{field}: invalid type"))?;
    validate(value).with_context(|| field.to_string())
}

fn validate(source: &Mapping) -> Result<()> {
    let mut errors = Vec::new();
    for key in source.keys() {
        if !key
            .as_str()
            .is_some_and(|key| ORDER.contains(&key) || key == "date")
        {
            errors.push(format!("unknown front matter field: {key:?}"));
        }
    }
    let date_field = if source.contains_key(Value::String("date".into())) {
        "date"
    } else {
        "created_at"
    };
    if date_field == "date" && source.contains_key(Value::String("created_at".into())) {
        errors.push("created_at and date must not both be specified".into());
    }
    for result in [
        validate_field(source, "title", None, |value: String| {
            validate_title(&value)
        }),
        validate_field(source, "slug", None, |value: String| validate_slug(&value)),
        validate_field(
            source,
            date_field,
            Some(Value::Null),
            |value: Option<String>| validate_created_at(value.as_deref()),
        ),
        validate_field(
            source,
            "excerpt",
            Some(Value::Null),
            |value: Option<String>| validate_excerpt(value.as_deref()),
        ),
        validate_field(
            source,
            "icatch_path",
            Some(Value::Null),
            |value: Option<String>| validate_icatch_path(value.as_deref()),
        ),
        validate_field(source, "tags", None, |value: Vec<String>| {
            validate_tags(&value)
        }),
        validate_field(source, "categories", None, |value: Vec<String>| {
            validate_categories(&value)
        }),
        validate_field(
            source,
            "deleted",
            Some(Value::Bool(false)),
            |_: bool| Ok(()),
        ),
        validate_field(
            source,
            "table_of_contents",
            Some(Value::Bool(false)),
            |_: bool| Ok(()),
        ),
    ] {
        if let Err(error) = result {
            errors.push(format!("{error:#}"));
        }
    }
    ensure!(errors.is_empty(), "{}", errors.join("; "));
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
    Ok(format!(
        "{}---{newline}{yaml}---{newline}{}",
        document.bom, document.body
    ))
}

fn collect_paths(roots: Vec<PathBuf>, errors: &mut Vec<String>) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for root in roots {
        let result = (|| -> Result<()> {
            let metadata = fs::metadata(&root)?;
            let canonical = fs::canonicalize(&root)?;
            if metadata.is_file() {
                if root.extension().is_some_and(|extension| extension == "md") {
                    paths.push(canonical);
                }
            } else if metadata.is_dir() {
                for entry in WalkDir::new(canonical).follow_links(false) {
                    match entry {
                        Ok(entry)
                            if entry.file_type().is_file()
                                && entry.path().extension().is_some_and(|x| x == "md") =>
                        {
                            paths.push(entry.into_path())
                        }
                        Ok(_) => {}
                        Err(error) => {
                            errors.push(format!("{}: traversal: {error}", root.display()))
                        }
                    }
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            errors.push(format!("{}: {error:#}", root.display()));
        }
    }
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        errors.push("no Markdown files found".into());
    }
    paths
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
fn run() -> Result<()> {
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
    process(roots, check)
}

fn process(roots: Vec<PathBuf>, check: bool) -> Result<()> {
    use std::collections::{HashMap, HashSet};
    let mut errors = Vec::new();
    let paths = collect_paths(roots, &mut errors);
    let mut groups: HashMap<String, Vec<PathBuf>> = HashMap::new();
    let mut prepared = Vec::new();
    for path in paths {
        let result = (|| -> Result<_> {
            let text = fs::read_to_string(&path).context("read")?;
            let document = parse(&text).context("parse")?;
            if let Some(slug) = document
                .matter
                .get(Value::String("slug".into()))
                .and_then(Value::as_str)
            {
                groups
                    .entry(rust_blog::slug::collision_key(slug))
                    .or_default()
                    .push(path.clone());
            }
            validate(&document.matter).context("validate")?;
            let formatted = render(&document, &format(&document.matter)).context("format")?;
            Ok((text, formatted))
        })();
        prepared.push((path, result));
    }
    let mut duplicates = HashSet::new();
    let mut duplicate_groups: Vec<_> = groups
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .collect();
    duplicate_groups.sort_by(|a, b| a.0.cmp(&b.0));
    for (slug, paths) in duplicate_groups {
        let names = paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        errors.push(format!("duplicate slug {slug:?}: {names}"));
        duplicates.extend(paths);
    }
    let mut succeeded = 0;
    for (path, result) in prepared {
        let (text, formatted) = match result {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("{}: {error:#}", path.display()));
                continue;
            }
        };
        if duplicates.contains(&path) {
            continue;
        }
        let result = (|| -> Result<()> {
            if text != formatted {
                ensure!(!check, "formatting required");
                fs::write(&path, formatted).context("write")?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => succeeded += 1,
            Err(error) => errors.push(format!("{}: {error:#}", path.display())),
        }
    }
    println!(
        "Markdown completed: {succeeded} file(s) succeeded, {} error(s)",
        errors.len()
    );
    ensure!(errors.is_empty(), "Markdown errors:\n{}", errors.join("\n"));
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplicate_groups_are_never_written_and_other_files_continue() {
        for (slugs, invalid_title) in [
            (["foo", "FOO", "Foo"], false),
            (["Σ", "ς", "σ"], false),
            (["foo", "FOO", "Foo"], true),
        ] {
            let root = std::env::temp_dir().join(format!(
                "format-duplicates-{}-{}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap()
            ));
            fs::create_dir_all(&root).unwrap();
            let input = "---\nslug: unique\ntitle: Test\ntags: []\ncategories: []\n---\nbody\n";
            let mut originals = Vec::new();
            for (index, slug) in slugs.into_iter().enumerate() {
                let path = root.join(format!("{index}.md"));
                let mut text = input.replace("slug: unique", &format!("slug: {slug}"));
                if invalid_title && index == 0 {
                    text = text.replace("title: Test\n", "");
                }
                fs::write(&path, &text).unwrap();
                originals.push((path, text));
            }
            let good = root.join("good.md");
            fs::write(&good, input).unwrap();
            for check in [true, false] {
                let error = process(vec![root.clone()], check).unwrap_err().to_string();
                assert!(error.contains("duplicate slug"), "{error}");
                for (path, text) in &originals {
                    assert!(error.contains(path.to_str().unwrap()), "{error}");
                    assert_eq!(fs::read_to_string(path).unwrap(), *text);
                }
                if invalid_title {
                    assert!(error.contains("title"), "{error}");
                }
                assert_eq!(
                    fs::read_to_string(&good).unwrap(),
                    if check {
                        input.to_string()
                    } else {
                        checked_format(input).unwrap()
                    }
                );
            }
            fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn collects_field_errors_independently() {
        let source = serde_yaml::from_str(
            "slug: ''\ntags: false\ncategories: []\ncreated_at: yesterday\nextra: true\n",
        )
        .unwrap();
        let error = validate(&source).unwrap_err().to_string();
        for field in ["title", "slug", "tags", "created_at", "extra"] {
            assert!(error.contains(field), "{error}");
        }
    }

    #[test]
    fn continues_after_discovery_and_validation_errors() {
        let root = std::env::temp_dir().join(format!(
            "format-errors-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir_all(&root).unwrap();
        let good = root.join("good.md");
        let broken = root.join("broken.md");
        let input = "---\nslug: test\ntitle: Test\ntags: []\ncategories: []\n---\nbody\n";
        fs::write(&good, input).unwrap();
        fs::write(&broken, "broken").unwrap();
        let roots = vec![root.join("missing"), root.clone()];
        let error = process(roots.clone(), true).unwrap_err().to_string();
        for expected in ["missing", "broken.md", "good.md: formatting required"] {
            assert!(error.contains(expected), "{error}");
        }
        assert_eq!(fs::read_to_string(&good).unwrap(), input);
        let error = process(roots, false).unwrap_err().to_string();
        assert!(error.contains("missing") && error.contains("broken.md"));
        assert_eq!(
            fs::read_to_string(&good).unwrap(),
            checked_format(input).unwrap()
        );
        assert_eq!(fs::read_to_string(&broken).unwrap(), "broken");
        assert!(process(vec![good], true).is_ok());
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn continues_after_write_failure() {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "format-write-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir_all(&root).unwrap();
        let locked = root.join("a.md");
        let good = root.join("b.md");
        let input = "---\nslug: test\ntitle: Test\ntags: []\ncategories: []\n---\nbody\n";
        fs::write(&locked, input.replace("slug: test", "slug: locked")).unwrap();
        fs::write(&good, input).unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o444)).unwrap();
        let result = process(vec![root.clone()], false);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(result.unwrap_err().to_string().contains("a.md: write"));
        assert_eq!(
            fs::read_to_string(&good).unwrap(),
            checked_format(input).unwrap()
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn collect_paths(roots: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
        let mut errors = Vec::new();
        let paths = super::collect_paths(roots, &mut errors);
        ensure!(errors.is_empty(), "{}", errors.join("\n"));
        Ok(paths)
    }

    fn checked_format(text: &str) -> Result<String> {
        let document = parse(text)?;
        validate(&document.matter)?;
        render(&document, &format(&document.matter))
    }

    #[test]
    fn preserves_optional_bom_and_line_endings() {
        for newline in ["\n", "\r\n"] {
            for bom in ["", "\u{feff}"] {
                let text = format!("{bom}---\ncategories: []\ntags: []\nslug: test\ntitle: Test\n---\n\n  本文\n\u{feff}body\n").replace('\n', newline);
                let output = checked_format(&text).unwrap();
                assert!(output.starts_with(&format!("{bom}---{newline}title: Test{newline}")));
                assert!(output.ends_with(&format!(
                    "---{newline}{newline}  本文{newline}\u{feff}body{newline}"
                )));
                assert_eq!(checked_format(&output).unwrap(), output);
            }
        }
        assert!(parse("\u{feff}\u{feff}---\ntitle: Test\n---\n").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn follows_explicit_links_but_not_nested_links() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!(
            "format-links-{}-{}",
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let directory = root.join("articles");
        fs::create_dir_all(&directory).unwrap();
        let article = directory.join("article.md");
        let text = "---\ncategories: []\ntags: []\nslug: test\ntitle: Test\n---\nbody\n";
        fs::write(&article, text).unwrap();
        fs::write(directory.join("notes.txt"), "ignore").unwrap();
        let file_link = root.join("linked.md");
        let directory_link = root.join("linked-directory");
        symlink(&article, &file_link).unwrap();
        symlink(&directory, &directory_link).unwrap();
        symlink(&directory, directory.join("cycle")).unwrap();
        symlink(root.join("missing.md"), directory.join("broken.md")).unwrap();
        let expected = vec![fs::canonicalize(&article).unwrap()];
        assert_eq!(collect_paths(vec![file_link.clone()]).unwrap(), expected);
        assert_eq!(
            collect_paths(vec![directory_link.clone()]).unwrap(),
            expected
        );
        let paths =
            collect_paths(vec![article.clone(), file_link.clone(), directory_link]).unwrap();
        assert_eq!(paths, expected);
        for path in paths {
            let formatted = checked_format(&fs::read_to_string(&path).unwrap()).unwrap();
            fs::write(path, formatted).unwrap();
        }
        assert_eq!(
            fs::read_to_string(&article).unwrap(),
            checked_format(text).unwrap()
        );
        assert!(
            fs::symlink_metadata(file_link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(collect_paths(vec![directory.join("broken.md")]).is_err());
        fs::remove_dir_all(root).unwrap();
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
