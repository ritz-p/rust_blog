use garde::Validate;
use sea_orm::prelude::DateTimeUtc;

#[derive(Validate, Debug)]
#[allow(dead_code)]
pub struct ArticleValidator {
    #[garde(length(utf16, min = 1, max = 50))]
    pub title: String,
    #[garde(length(utf16, min = 1, max = 100))]
    pub slug: String,
    #[garde(length(utf16, min = 1, max = 100))]
    pub excerpt: Option<String>,
    #[garde(length(utf16, min = 1, max = 200))]
    pub icatch_path: Option<String>,
    #[garde(skip)]
    pub content: String,
    #[garde(skip)]
    pub created_at: DateTimeUtc,
    #[garde(skip)]
    pub updated_at: DateTimeUtc,
}
