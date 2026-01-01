use crate::entity::fixed_content;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

pub async fn get_fixed_content_by_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<Option<fixed_content::Model>, DbErr> {
    fixed_content::Entity::find()
        .filter(fixed_content::Column::Slug.eq(slug.to_string()))
        .one(db)
        .await
}
