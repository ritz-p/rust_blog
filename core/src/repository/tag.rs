use crate::entity::{article, tag};
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, ModelTrait, QueryFilter, QueryOrder,
};

pub async fn get_all_tags(db: &DatabaseConnection) -> Result<Vec<tag::Model>, DbErr> {
    tag::Entity::find()
        .order_by(tag::Column::Name, sea_orm::Order::Asc)
        .all(db)
        .await
}

pub async fn get_tag_by_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<Option<tag::Model>, DbErr> {
    tag::Entity::find()
        .filter(tag::Column::Slug.eq(slug.to_string()))
        .one(db)
        .await
}

pub async fn get_tags_by_article(
    db: &DatabaseConnection,
    article: &article::Model,
) -> Result<Vec<tag::Model>, DbErr> {
    article.find_related(tag::Entity).all(db).await
}
