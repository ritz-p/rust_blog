use crate::seed::config::{PathConfig, PathConfigTrait};
use dotenvy::dotenv;
use std::env;
pub fn load_env() -> PathConfig {
    dotenv().expect(".env not found");
    PathConfig::new(
        env::var("FIXED_CONTENT_PATH").ok().or_else(|| None),
        env::var("ARTICLE_PATH").ok().or_else(|| None),
        env::var("CONFIG_TOML_PATH").ok().or_else(|| None),
    )
}
