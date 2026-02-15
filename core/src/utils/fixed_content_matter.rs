use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct FixedContentMatter {
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
}
