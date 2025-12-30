use crate::{entity::fixed_content::Model as FixedContentModel, entity_extension::ValidateModel};
use garde::Validate;
use sea_orm::prelude::DateTimeUtc;

#[derive(Validate, Debug)]
pub struct FixedContentValidator {
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

impl ValidateModel<FixedContentModel> for FixedContentValidator {
    fn new(model: FixedContentModel) -> Self {
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
