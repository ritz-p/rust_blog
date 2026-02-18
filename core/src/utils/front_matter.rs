use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct FrontMatter {
    pub title: String,
    pub slug: String,
    #[serde(default)]
    pub deleted: bool,
    pub excerpt: Option<String>,
    pub icatch_path: Option<String>,
    pub tags: Vec<String>,
    pub categories: Vec<String>,
}

impl FrontMatter {
    #[allow(dead_code)]
    pub fn new(
        title: String,
        slug: String,
        deleted: bool,
        excerpt: Option<String>,
        icatch_path: Option<String>,
        tags: Vec<String>,
        categories: Vec<String>,
    ) -> Self {
        Self {
            title,
            slug,
            deleted,
            excerpt,
            icatch_path,
            tags,
            categories,
        }
    }
}
