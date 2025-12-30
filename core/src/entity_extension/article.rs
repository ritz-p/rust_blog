use crate::{entity::article::Model as ArticleModel, entity_extension::ValidateModel};
use garde::Validate;
use sea_orm::prelude::DateTimeUtc;

#[derive(Validate, Debug)]
pub struct ArticleValidator {
    #[garde(length(utf16, min = 1, max = 50))]
    pub title: String,
    #[garde(length(utf16, min = 1, max = 100))]
    pub slug: String,
    #[garde(length(utf16, min = 1, max = 100))]
    pub excerpt: Option<String>,
    #[garde(skip)]
    pub content: String,
    #[garde(skip)]
    pub created_at: DateTimeUtc,
    #[garde(skip)]
    pub updated_at: DateTimeUtc,
}

impl ValidateModel<ArticleModel> for ArticleValidator {
    fn new(model: ArticleModel) -> Self {
        Self {
            title: model.title,
            slug: model.slug,
            excerpt: model.excerpt,
            content: model.content,
            created_at: model.created_at,
            updated_at: model.updated_at,
        }
    }
}
