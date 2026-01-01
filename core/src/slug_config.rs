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
