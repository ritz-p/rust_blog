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

pub async fn get_all_articles_with_updated_at(
    db: &DatabaseConnection,
) -> Result<Vec<article::Model>, DbErr> {
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
    sort_key: &str,
) -> Result<Vec<article::Model>, DbErr> {
    if let Some(tag) = tag::Entity::find()
        .filter(tag::Column::Slug.eq(tag_slug))
        .one(db)
        .await?
    {
        let mut find = tag.find_related(article::Entity).distinct();
        match sort_key {
            "updated_at" => find = find.order_by_desc(article::Column::UpdatedAt),
            "created_at" => find = find.order_by_desc(article::Column::CreatedAt),
            _ => find = find.order_by_desc(article::Column::CreatedAt),
        }
        let articles = find.all(db).await?;
        Ok(articles)
    } else {
        Err(DbErr::RecordNotFound("tag not found".into()))
    }
}

pub async fn get_article_by_category_slug(
    db: &DatabaseConnection,
    category_slug: &str,
    sort_key: &str,
) -> Result<Vec<article::Model>, DbErr> {
    if let Some(category) = category::Entity::find()
        .filter(category::Column::Slug.eq(category_slug))
        .one(db)
        .await?
    {
        let mut find = category.find_related(article::Entity).distinct();
        match sort_key {
            "updated_at" => {
                find = find.order_by_desc(article::Column::UpdatedAt);
            }
            "created_at" => {
                find = find.order_by_desc(article::Column::CreatedAt);
            }
            _ => {
                find = find.order_by_desc(article::Column::CreatedAt);
            }
        }
        let articles = find.all(db).await?;
        Ok(articles)
    } else {
        Err(DbErr::RecordNotFound("category not found".into()))
    }
}
