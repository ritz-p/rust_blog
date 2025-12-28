use crate::{
    domain::page::{Page, PageInfo},
    entity::article_tag,
};
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, JoinType, Order, QueryFilter, QueryOrder,
    QuerySelect, prelude::*,
};

use crate::entity::{article, article_category, category, tag};

pub async fn get_all_articles(
    db: &DatabaseConnection,
    page: Page,
) -> Result<(Vec<article::Model>, PageInfo), DbErr> {
    let base = article::Entity::find().order_by_desc(article::Column::CreatedAt);
    let total = base.clone().count(db).await?;
    let page = page.normalize(50);
    let page_info = PageInfo::new(page, total);
    let offset = (page_info.count - 1) * page_info.per;
    let items = base.limit(page_info.per).offset(offset).all(db).await?;
    Ok((items, page_info))
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

pub async fn get_latest_articles(
    db: &DatabaseConnection,
    limit: u64,
) -> Result<Vec<article::Model>, DbErr> {
    let articles = article::Entity::find()
        .order_by_desc(article::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await?;
    Ok(articles)
}

pub async fn get_articles_by_tag_slug(
    db: &DatabaseConnection,
    page: Page,
    tag_slug: &str,
    sort_key: &str,
) -> Result<(Vec<article::Model>, PageInfo), DbErr> {
    if let Some(tag) = tag::Entity::find()
        .filter(tag::Column::Slug.eq(tag_slug))
        .one(db)
        .await?
    {
        let mut find: Select<article::Entity> = tag.find_related(article::Entity).distinct();
        match sort_key {
            "updated_at" => find = find.order_by_desc(article::Column::UpdatedAt),
            "created_at" => find = find.order_by_desc(article::Column::CreatedAt),
            _ => find = find.order_by_desc(article::Column::CreatedAt),
        }
        let total = find.clone().count(db).await?;
        let page = page.normalize(50);
        let page_info = PageInfo::new(page, total);
        let offset = (page_info.count - 1) * page_info.per;
        let articles = find.limit(page_info.per).offset(offset).all(db).await?;
        Ok((articles, page_info))
    } else {
        Err(DbErr::RecordNotFound("tag not found".into()))
    }
}

pub async fn get_article_by_category_slug(
    db: &DatabaseConnection,
    page: Page,
    category_slug: &str,
    sort_key: &str,
) -> Result<(Vec<article::Model>, PageInfo), DbErr> {
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
        let total = find.clone().count(db).await?;
        let page = page.normalize(50);
        let page_info = PageInfo::new(page, total);
        let offset = (page_info.count - 1) * page_info.per;
        let articles = find.limit(page_info.per).offset(offset).all(db).await?;
        Ok((articles, page_info))
    } else {
        Err(DbErr::RecordNotFound("category not found".into()))
    }
}
