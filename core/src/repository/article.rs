use crate::entity::article_tag;
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, JoinType, Order, QueryFilter, QueryOrder,
    QuerySelect, prelude::*,
};

use crate::entity::{article, article_category, category, tag};

pub async fn get_all_articles(db: &DatabaseConnection) -> Result<Vec<article::Model>, DbErr> {
    article::Entity::find()
        .order_by(article::Column::CreatedAt, Order::Desc)
        .all(db)
        .await
}
pub async fn get_article_by_id(
    db: &DatabaseConnection,
    id: i32,
) -> Result<Option<article::Model>, DbErr> {
    article::Entity::find_by_id(id).one(db).await
}

pub async fn get_article_by_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<Option<article::Model>, DbErr> {
    article::Entity::find()
        .filter(article::Column::Slug.eq(slug.to_string()))
        .one(db)
        .await
}

pub async fn get_articles_by_tag_slug(
    db: &DatabaseConnection,
    tag_slug: &str,
) -> Result<Vec<article::Model>, DbErr> {
    let Some(_tag) = tag::Entity::find()
        .filter(tag::Column::Slug.eq(tag_slug))
        .one(db)
        .await?
    else {
        return Err(DbErr::RecordNotFound("tag not found".into()));
    };

    let articles = article::Entity::find()
        .join(JoinType::InnerJoin, article::Relation::ArticleTag.def())
        .join(JoinType::InnerJoin, article_tag::Relation::Tag.def())
        .filter(tag::Column::Slug.eq(tag_slug))
        .order_by_desc(article::Column::CreatedAt)
        .distinct()
        .all(db)
        .await?;

    Ok(articles)
}

pub async fn get_article_by_category_slug(
    db: &DatabaseConnection,
    category_slug: &str,
) -> Result<Vec<article::Model>, DbErr> {
    let rows = category::Entity::find()
        .filter(category::Column::Slug.eq(category_slug))
        .find_with_related(article::Entity)
        .all(db)
        .await?;
    Ok(rows.into_iter().flat_map(|(_, arts)| arts).collect())
}
