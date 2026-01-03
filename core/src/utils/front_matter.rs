use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct FrontMatter {
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
    pub icatch_path: Option<String>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
}

impl FrontMatter {
    pub fn new(
        title: String,
        slug: String,
        excerpt: Option<String>,
        icatch_path: Option<String>,
        tags: Vec<String>,
        categories: Vec<String>,
    ) -> Self {
        Self {
            title,
            slug,
            excerpt,
            icatch_path,
            tags,
            categories,
        }
    }
}
