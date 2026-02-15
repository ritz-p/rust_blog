use crate::entity::{article, category};
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, ModelTrait, QueryFilter, QueryOrder,
};

pub async fn get_all_categories(db: &DatabaseConnection) -> Result<Vec<category::Model>, DbErr> {
    category::Entity::find()
        .order_by(category::Column::Name, sea_orm::Order::Asc)
        .all(db)
        .await
}

#[allow(dead_code)]
pub async fn get_category_by_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<Option<category::Model>, DbErr> {
    category::Entity::find()
        .filter(category::Column::Slug.eq(slug.to_string()))
        .one(db)
        .await
}

pub async fn get_categories_by_article(
    db: &DatabaseConnection,
    article: &article::Model,
) -> Result<Vec<category::Model>, DbErr> {
    article.find_related(category::Entity).all(db).await
}
