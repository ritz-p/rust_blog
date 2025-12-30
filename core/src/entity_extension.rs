use sea_orm::EntityTrait;

pub mod article;
pub mod category;
pub mod fixed_content;
pub mod tag;

pub trait ValidateModel<T> {
    fn new(model: T) -> Self;
}
