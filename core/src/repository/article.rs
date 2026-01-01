use crate::domain::page::{Page, PageInfo};
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    prelude::*,
};

use crate::entity::{article, category, tag};

pub async fn get_all_articles(
    db: &DatabaseConnection,
    page: Page,
) -> Result<(Vec<article::Model>, PageInfo), DbErr> {
    let total = article::Entity::find().count(db).await?;
    let page = page.normalize(50);
    let page_info = PageInfo::new(page, total);
    let offset = (page_info.count - 1) * page_info.per;
    let articles = article::Entity::find()
        .order_by_desc(article::Column::CreatedAt)
        .offset(offset)
        .limit(page_info.per)
        .all(db)
        .await?;
    Ok((articles, page_info))
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
        let total = tag
            .find_related(article::Entity)
            .distinct()
            .count(db)
            .await?;
        let page = page.normalize(50);
        let page_info = PageInfo::new(page, total);
        let offset = (page_info.count - 1) * page_info.per;
        let articles = match sort_key {
            "updated_at" => {
                tag.find_related(article::Entity)
                    .distinct()
                    .order_by_desc(article::Column::UpdatedAt)
                    .offset(offset)
                    .limit(page_info.per)
                    .all(db)
                    .await?
            }
            "created_at" => {
                tag.find_related(article::Entity)
                    .distinct()
                    .order_by_desc(article::Column::CreatedAt)
                    .offset(offset)
                    .limit(page_info.per)
                    .all(db)
                    .await?
            }
            _ => {
                tag.find_related(article::Entity)
                    .distinct()
                    .order_by_desc(article::Column::UpdatedAt)
                    .offset(offset)
                    .limit(page_info.per)
                    .all(db)
                    .await?
            }
        };
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
        let total = category
            .find_related(article::Entity)
            .distinct()
            .count(db)
            .await?;
        let page = page.normalize(50);
        let page_info = PageInfo::new(page, total);
        let offset = (page_info.count - 1) * page_info.per;
        let articles = match sort_key {
            "updated_at" => {
                category
                    .find_related(article::Entity)
                    .distinct()
                    .order_by_desc(article::Column::UpdatedAt)
                    .offset(offset)
                    .limit(page_info.per)
                    .all(db)
                    .await?
            }
            "created_at" => {
                category
                    .find_related(article::Entity)
                    .distinct()
                    .order_by_desc(article::Column::CreatedAt)
                    .offset(offset)
                    .limit(page_info.per)
                    .all(db)
                    .await?
            }
            _ => {
                category
                    .find_related(article::Entity)
                    .distinct()
                    .order_by_desc(article::Column::UpdatedAt)
                    .offset(offset)
                    .limit(page_info.per)
                    .all(db)
                    .await?
            }
        };
        Ok((articles, page_info))
    } else {
        Err(DbErr::RecordNotFound("category not found".into()))
    }
}
