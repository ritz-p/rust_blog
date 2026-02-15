use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct CommonConfig {
    pub site_name: Option<String>,
    pub default_icatch_path: Option<String>,
    pub favicon_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommonConfigMap {
    pub map: HashMap<String, String>,
}

#[derive(Deserialize)]
struct Top {
    #[serde(flatten)]
    tables: HashMap<String, HashMap<String, String>>,
}

impl CommonConfigMap {
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let s = fs::read_to_string(path)?;
        let cfg: CommonConfigMap = toml::from_str(&s)?;
        Ok(cfg)
    }
    pub fn from_toml_file_key(path: impl AsRef<Path>, key: &str) -> Result<Self> {
        let s = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {:?}", path.as_ref()))?;
        let mut top: Top = toml::from_str(&s)
            .with_context(|| format!("failed to parse TOML {:?}", path.as_ref()))?;

        if let Some(map) = top.tables.remove(key) {
            Ok(CommonConfigMap { map })
        } else {
            let keys: Vec<_> = top.tables.keys().cloned().collect();
            bail!("table {:?} not found. available: {:?}", key, keys)
        }
    }
}

pub fn load_config() -> HashMap<String, String> {
    load_config_from_path("blog_config.toml")
}

fn with_defaults(mut map: HashMap<String, String>) -> HashMap<String, String> {
    if !map.contains_key("site_name") {
        map.insert("site_name".to_string(), "My Blog".to_string());
    }
    if !map.contains_key("default_icatch_path") {
        map.insert("default_icatch_path".to_string(), "".to_string());
    }
    if !map.contains_key("favicon_path") {
        map.insert(
            "favicon_path".to_string(),
            "/icon/rust-logo-128x128-blk.png".to_string(),
        );
    }
    map
}

fn load_config_from_path(toml_path: &str) -> HashMap<String, String> {
    let common_config = CommonConfigMap::from_toml_file_key(toml_path, "common")
        .with_context(|| format!("failed to read common config: {}", toml_path));
    let map = match common_config {
        Ok(config) => config.map,
        Err(_) => HashMap::new(),
    };
    with_defaults(map)
}

#[cfg(test)]
mod tests {
    use super::CommonConfigMap;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_temp_toml(contents: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "common_config_test_{}_{}.toml",
            std::process::id(),
            ts
        ));
        fs::write(&path, contents).expect("failed to write temp toml");
        path
    }

    #[test]
    fn from_toml_file_parses_map() {
        let path = write_temp_toml(
            r#"
map = { site_name = "Blog", favicon_path = "/favicon.ico" }
"#,
        );
        let cfg = CommonConfigMap::from_toml_file(&path).expect("failed to parse common config");
        assert_eq!(cfg.map.get("site_name"), Some(&"Blog".to_string()));
        assert_eq!(cfg.map.get("favicon_path"), Some(&"/favicon.ico".to_string()));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn from_toml_file_key_returns_selected_table() {
        let path = write_temp_toml(
            r#"
[common]
site_name = "Blog"
favicon_path = "/favicon.ico"

[seed]
article_path = "content/articles"
"#,
        );
        let cfg = CommonConfigMap::from_toml_file_key(&path, "common")
            .expect("failed to read common table");
        assert_eq!(cfg.map.get("site_name"), Some(&"Blog".to_string()));
        assert_eq!(cfg.map.get("favicon_path"), Some(&"/favicon.ico".to_string()));
        assert_eq!(cfg.map.len(), 2);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn from_toml_file_key_returns_error_for_missing_table() {
        let path = write_temp_toml(
            r#"
[common]
site_name = "Blog"
"#,
        );
        let err =
            CommonConfigMap::from_toml_file_key(&path, "seed").expect_err("should return an error");
        assert!(err.to_string().contains("table \"seed\" not found"));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_config_from_path_fills_missing_defaults() {
        let path = write_temp_toml(
            r#"
[common]
site_name = "My Site"
"#,
        );
        let map = super::load_config_from_path(path.to_str().expect("invalid temp path"));
        assert_eq!(map.get("site_name"), Some(&"My Site".to_string()));
        assert_eq!(map.get("default_icatch_path"), Some(&"".to_string()));
        assert_eq!(
            map.get("favicon_path"),
            Some(&"/icon/rust-logo-128x128-blk.png".to_string())
        );
        let _ = fs::remove_file(path);
    }

    #[test]
    fn load_config_from_path_returns_defaults_when_file_missing() {
        let path = std::env::temp_dir().join(format!(
            "missing_common_config_{}_{}.toml",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time before UNIX_EPOCH")
                .as_nanos()
        ));
        let map = super::load_config_from_path(path.to_str().expect("invalid temp path"));
        assert_eq!(map.get("site_name"), Some(&"My Blog".to_string()));
        assert_eq!(map.get("default_icatch_path"), Some(&"".to_string()));
        assert_eq!(
            map.get("favicon_path"),
            Some(&"/icon/rust-logo-128x128-blk.png".to_string())
        );
    }
}
