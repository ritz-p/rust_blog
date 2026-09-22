use crate::{entity::article, utils::markdown::markdown_to_text};

pub fn terms(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

pub fn article_text(article: &article::Model) -> String {
    format!(
        "{}\n{}\n{}",
        article.title,
        markdown_to_text(article.excerpt.as_deref().unwrap_or_default()),
        markdown_to_text(&article.content),
    )
    .to_lowercase()
}
