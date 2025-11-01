use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PathConfig {
    pub fixed_content_path: Option<String>,
    pub article_path: Option<String>,
    pub tag_config_toml_path: Option<String>,
    pub category_config_toml_path: Option<String>,
}

impl PathConfigTrait for PathConfig {
    fn init() -> Self {
        PathConfig {
            fixed_content_path: None,
            article_path: None,
            tag_config_toml_path: None,
            category_config_toml_path: None,
        }
    }
    fn new(
        fixed_content_path: Option<String>,
        article_path: Option<String>,
        tag_config_toml_path: Option<String>,
        category_config_toml_path: Option<String>,
    ) -> Self {
        PathConfig {
            fixed_content_path,
            article_path,
            tag_config_toml_path,
            category_config_toml_path,
        }
    }
    fn with_default(self) -> Self {
        Self {
            fixed_content_path: self
                .fixed_content_path
                .or_else(|| Some("content/fixed_contents".to_string())),
            article_path: self
                .article_path
                .or_else(|| Some("content/articles".to_string())),
            tag_config_toml_path: self
                .tag_config_toml_path
                .or_else(|| Some("content/config/slug.toml".to_string())),
            category_config_toml_path: self
                .category_config_toml_path
                .or_else(|| Some("content/config/slug.toml".to_string())),
        }
    }

    fn update(&mut self, new: Self) {
        self.article_path = new
            .fixed_content_path
            .clone()
            .or_else(|| self.fixed_content_path.clone());
        self.fixed_content_path = new
            .fixed_content_path
            .clone()
            .or_else(|| self.fixed_content_path.clone());
        self.tag_config_toml_path = new
            .tag_config_toml_path
            .clone()
            .or_else(|| self.tag_config_toml_path.clone());
        self.category_config_toml_path = new
            .category_config_toml_path
            .clone()
            .or_else(|| self.category_config_toml_path.clone());
    }
}

pub trait PathConfigTrait {
    fn init() -> Self;
    fn new(
        fixed_content_path: Option<String>,
        article_path: Option<String>,
        tag_config_toml_path: Option<String>,
        category_config_toml_path: Option<String>,
    ) -> Self;
    fn update(&mut self, new: Self);
    fn with_default(self) -> Self;
}
