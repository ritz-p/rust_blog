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

#[cfg(test)]
mod tests {
    use super::{PathConfig, PathConfigTrait};

    #[test]
    fn new_uses_defaults_when_none_is_passed() {
        let config = PathConfig::new(None, None, None);
        assert_eq!(config.fixed_content_path, "content/fixed_contents");
        assert_eq!(config.article_path, "content/articles");
        assert_eq!(config.config_toml_path, "blog_config.toml");
    }

    #[test]
    fn new_uses_given_values_when_some_is_passed() {
        let config = PathConfig::new(
            Some("fixed".to_string()),
            Some("articles".to_string()),
            Some("config.toml".to_string()),
        );
        assert_eq!(config.fixed_content_path, "fixed");
        assert_eq!(config.article_path, "articles");
        assert_eq!(config.config_toml_path, "config.toml");
    }

    #[test]
    fn update_replaces_all_fields() {
        let mut current = PathConfig::new(None, None, None);
        let next = PathConfig::new(
            Some("new_fixed".to_string()),
            Some("new_articles".to_string()),
            Some("new_config.toml".to_string()),
        );
        current.update(next);
        assert_eq!(current.fixed_content_path, "new_fixed");
        assert_eq!(current.article_path, "new_articles");
        assert_eq!(current.config_toml_path, "new_config.toml");
    }
}
