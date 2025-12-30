use crate::{entity::tag::Model as TagModel, entity_extension::ValidateModel};
use garde::Validate;

#[derive(Validate, Debug)]
pub struct TagValidator {
    #[garde(length(utf16, min = 1, max = 50))]
    pub name: String,
    #[garde(length(utf16, min = 1, max = 50))]
    pub slug: String,
}

impl ValidateModel<TagModel> for TagValidator {
    fn new(model: TagModel) -> Self {
        Self {
            name: model.name,
            slug: model.slug,
        }
    }
}
