use rust_blog::entity::category;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub async fn get_all_categories(db: &DatabaseConnection) -> Result<Vec<category::Model>, DbErr> {
    category::Entity::find()
        .order_by(category::Column::Name, sea_orm::Order::Asc)
        .all(db)
        .await
}

pub async fn get_tag_by_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<Option<category::Model>, DbErr> {
    category::Entity::find()
        .filter(category::Column::Slug.eq(slug.to_string()))
        .one(db)
        .await
}
