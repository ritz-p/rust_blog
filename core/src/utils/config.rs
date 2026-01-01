use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};

#[derive(Debug, Deserialize)]
pub struct CommonConfig {
    pub site_name: Option<String>,
    pub default_icatch_path: Option<String>,
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
    let toml_path = "blog_config.toml";
    let common_config = CommonConfigMap::from_toml_file_key(toml_path, "common")
        .with_context(|| format!("failed to read common config: {}", toml_path));
    let mut map = match common_config {
        Ok(config) => config.map,
        Err(_) => HashMap::new(),
    };
    if !map.contains_key("site_name") {
        map.insert("site_name".to_string(), "My Blog".to_string());
    }
    if !map.contains_key("default_icatch_path") {
        map.insert("default_icatch_path".to_string(), "".to_string());
    }
    map
}
