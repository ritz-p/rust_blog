use crate::utils;
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

pub fn parse_markdown(path: &std::path::Path) -> Result<(FrontMatter, String), serde_yaml::Error> {
    let text = fs::read_to_string(path).expect("Failed to load file");
    let parts: Vec<&str> = text.splitn(3, "---").collect();
    if parts.len() != 3 {
        panic!("FrontMatter not found in {:?}", path);
    }
    let front_matter = serde_yaml::from_str(parts[1])?;
    let body = parts[2].trim_start().to_string();
    Ok((front_matter, body))
}
