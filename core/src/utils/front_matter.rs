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
