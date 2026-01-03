use serde::{Deserialize, Serialize};
pub mod env;
pub mod seed;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PathConfig {
    pub fixed_content_path: String,
    pub article_path: String,
    pub config_toml_path: String,
}

pub trait PathConfigTrait {
    fn new(
        fixed_content_path: Option<String>,
        article_path: Option<String>,
        config_toml_path: Option<String>,
    ) -> Self;
    fn update(&mut self, new: Self);
}

impl PathConfigTrait for PathConfig {
    fn new(
        fixed_content_path: Option<String>,
        article_path: Option<String>,
        config_toml_path: Option<String>,
    ) -> Self {
        PathConfig {
            fixed_content_path: fixed_content_path.unwrap_or("content/fixed_contents".to_string()),
            article_path: article_path.unwrap_or("content/articles".to_string()),
            config_toml_path: config_toml_path.unwrap_or("blog_config.toml".to_string()),
        }
    }
    fn update(&mut self, new: Self) {
        self.article_path = new.article_path;
        self.fixed_content_path = new.fixed_content_path;
        self.config_toml_path = new.config_toml_path;
    }
}
