use serde::Deserialize;
use std::{collections::HashMap, fs, path::Path};
#[derive(Debug, Deserialize)]
pub struct SlugConfig {
    pub tags: HashMap<String, String>,
}

impl SlugConfig {
    pub fn from_toml_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let s = fs::read_to_string(path)?;
        let cfg: SlugConfig = toml::from_str(&s)?;
        Ok(cfg)
    }
}
