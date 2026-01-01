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
