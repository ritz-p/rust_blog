use crate::entity::{category, tag};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, Value};
pub trait NameSlugEntity: EntityTrait {
    fn col_name() -> Self::Column;
    fn col_slug() -> Self::Column;
}

impl NameSlugEntity for tag::Entity {
    fn col_name() -> Self::Column {
        tag::Column::Name
    }
    fn col_slug() -> Self::Column {
        tag::Column::Slug
    }
}

impl NameSlugEntity for category::Entity {
    fn col_name() -> Self::Column {
        category::Column::Name
    }
    fn col_slug() -> Self::Column {
        category::Column::Slug
    }
}

pub trait ActiveNameSlugExt {
    fn set_name_slug(&mut self, name: &str, slug: &str);
    fn get_name(&self) -> Option<String>;
    fn get_slug(&self) -> Option<String>;
}

pub fn set_name_slug<T>(am: &mut T::ActiveModel, name: &str, slug: &str)
where
    T: EntityTrait + NameSlugEntity,
    T::ActiveModel: ActiveModelTrait,
    T::Column: ColumnTrait + Copy,
{
    am.set(T::col_name(), Value::from(name));
    am.set(T::col_slug(), Value::from(slug));
}
