use crate::entity;
use crate::utils;
use entity::{
    article::Column as ArticleColumn, article::Entity as ArticleEntity,
    article::Model as ArticleModel, article_category, article_tag, category, tag,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, Set,
};
use std::default::Default;
use utils::front_matter::FrontMatter;

pub async fn seed_article(
    db: &DatabaseConnection,
    front_matter: &FrontMatter,
    body: &str,
) -> Result<i32, DbErr> {
    let model: Option<ArticleModel> = ArticleEntity::find()
        .filter(ArticleColumn::Slug.eq(front_matter.slug.clone()))
        .one(db)
        .await?;
    let mut active_model = match model {
        Some(model) => model.into_active_model(),
        None => Default::default(),
    };
    active_model.title = Set(front_matter.title.clone());
    active_model.slug = Set(front_matter.slug.clone());
    active_model.excerpt = Set(front_matter.excerpt.clone());
    active_model.content = Set(body.to_string());
    active_model.created_at = Set(front_matter.created_at.clone());
    active_model.updated_at = Set(front_matter.updated_at.clone());

    let saved = active_model.save(db).await?;
    let article_id: i32 = match saved.id {
        ActiveValue::Set(id) | ActiveValue::Unchanged(id) => id,
        ActiveValue::NotSet => return Err(DbErr::Custom("article id not set".into())),
    };
    Ok(article_id)
}

pub async fn seed_tag(
    db: &DatabaseConnection,
    front_matter: &FrontMatter,
    article_id: i32,
) -> Result<(), DbErr> {
    for tag_slug in &front_matter.tags {
        let existing = tag::Entity::find()
            .filter(tag::Column::Slug.eq(tag_slug.as_str()))
            .one(db)
            .await?;
        let tag_id = if let Some(m) = existing {
            m.id
        } else {
            tag::ActiveModel {
                name: Set(tag_slug.clone()),
                slug: Set(tag_slug.clone()),
                ..Default::default()
            }
            .insert(db)
            .await?
            .id
        };

        let exists_link = article_tag::Entity::find()
            .filter(article_tag::Column::ArticleId.eq(article_id))
            .filter(article_tag::Column::TagId.eq(tag_id))
            .one(db)
            .await?
            .is_some();

        if !exists_link {
            article_tag::ActiveModel {
                article_id: Set(article_id),
                tag_id: Set(tag_id),
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}

pub async fn seed_category(
    db: &DatabaseConnection,
    front_matter: &FrontMatter,
    article_id: i32,
) -> Result<(), DbErr> {
    for category_slug in &front_matter.categories {
        let existing = category::Entity::find()
            .filter(category::Column::Slug.eq(category_slug.as_str()))
            .one(db)
            .await?;
        let category_id = if let Some(m) = existing {
            m.id
        } else {
            category::ActiveModel {
                name: Set(category_slug.clone()),
                slug: Set(category_slug.clone()),
                ..Default::default()
            }
            .insert(db)
            .await?
            .id
        };

        let exists_link = article_category::Entity::find()
            .filter(article_category::Column::ArticleId.eq(article_id))
            .filter(article_category::Column::CategoryId.eq(category_id))
            .one(db)
            .await?
            .is_some();

        if !exists_link {
            article_category::ActiveModel {
                article_id: Set(article_id),
                category_id: Set(category_id),
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}
