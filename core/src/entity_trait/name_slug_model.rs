use crate::entity::{category, tag};

pub trait NameSlugModel {
    fn name(&self) -> &str;
    fn slug(&self) -> &str;
}

impl NameSlugModel for tag::Model {
    fn name(&self) -> &str {
        &self.name
    }
    fn slug(&self) -> &str {
        &self.slug
    }
}

impl NameSlugModel for category::Model {
    fn name(&self) -> &str {
        &self.name
    }
    fn slug(&self) -> &str {
        &self.slug
    }
}
