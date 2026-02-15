use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};
#[derive(Debug, Deserialize)]
pub struct SlugConfig {
    pub map: HashMap<String, String>,
}

#[derive(Deserialize)]
struct Top {
    #[serde(flatten)]
    tables: HashMap<String, HashMap<String, String>>,
}

impl SlugConfig {
    pub fn from_toml_file(path: impl AsRef<Path>) -> Result<Self> {
        let s = fs::read_to_string(path)?;
        let cfg: SlugConfig = toml::from_str(&s)?;
        Ok(cfg)
    }
    pub fn from_toml_file_key(path: impl AsRef<Path>, key: &str) -> Result<Self> {
        let s = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {:?}", path.as_ref()))?;
        let mut top: Top = toml::from_str(&s)
            .with_context(|| format!("failed to parse TOML {:?}", path.as_ref()))?;

        if let Some(map) = top.tables.remove(key) {
            Ok(SlugConfig { map })
        } else {
            let keys: Vec<_> = top.tables.keys().cloned().collect();
            bail!("table {:?} not found. available: {:?}", key, keys)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SlugConfig;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_temp_toml(contents: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "slug_config_test_{}_{}.toml",
            std::process::id(),
            ts
        ));
        fs::write(&path, contents).expect("failed to write temp toml");
        path
    }

    #[test]
    fn from_toml_file_parses_map() {
        let path = write_temp_toml("map = { rust = \"Rust\", web = \"Web\" }");
        let cfg = SlugConfig::from_toml_file(&path).expect("failed to parse slug config");
        assert_eq!(cfg.map.get("rust"), Some(&"Rust".to_string()));
        assert_eq!(cfg.map.get("web"), Some(&"Web".to_string()));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn from_toml_file_key_returns_selected_table() {
        let path = write_temp_toml(
            r#"
[article]
rust = "rust-post"

[category]
dev = "developer"
"#,
        );
        let cfg =
            SlugConfig::from_toml_file_key(&path, "article").expect("failed to load article table");
        assert_eq!(cfg.map.len(), 1);
        assert_eq!(cfg.map.get("rust"), Some(&"rust-post".to_string()));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn from_toml_file_key_returns_error_for_missing_table() {
        let path = write_temp_toml(
            r#"
[article]
rust = "rust-post"
"#,
        );
        let err = SlugConfig::from_toml_file_key(&path, "tag").expect_err("should return an error");
        assert!(err.to_string().contains("table \"tag\" not found"));
        let _ = fs::remove_file(path);
    }
}
