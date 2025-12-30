use crate::{entity::category::Model as CategoryModel, entity_extension::ValidateModel};
use garde::Validate;

#[derive(Validate, Debug)]
pub struct CategoryValidator {
    #[garde(length(utf16, min = 1, max = 50))]
    pub name: String,
    #[garde(length(utf16, min = 1, max = 50))]
    pub slug: String,
}

impl ValidateModel<CategoryModel> for CategoryValidator {
    fn new(model: CategoryModel) -> Self {
        Self {
            name: model.name,
            slug: model.slug,
        }
    }
}
