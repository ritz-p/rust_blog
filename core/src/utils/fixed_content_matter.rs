use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct FixedContentMatter {
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
}
