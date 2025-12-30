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
