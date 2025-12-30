use crate::entity;
use crate::entity_extension::fixed_content::FixedContentValidator;
use crate::utils;
use chrono::Timelike;
use chrono::Utc;
use entity::fixed_content::{
    Column as FixedContentColumn, Entity as FixedContentEntity, Model as FixedContentModel,
};
use garde::Validate;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, Set,
};
use std::default::Default;
use utils::fixed_content_matter::FixedContentMatter;

pub async fn seed_fixed_content(
    db: &DatabaseConnection,
    fixed_content_matter: &FixedContentMatter,
    body: &str,
) -> Result<i32, anyhow::Error> {
    let model: Option<FixedContentModel> = FixedContentEntity::find()
        .filter(FixedContentColumn::Slug.eq(fixed_content_matter.slug.clone()))
        .one(db)
        .await?;
    let mut active_model: entity::fixed_content::ActiveModel = match model {
        Some(model) => model.into_active_model(),
        None => Default::default(),
    };
    active_model
        .title
        .set_if_not_equals(fixed_content_matter.title.clone());
    active_model
        .slug
        .set_if_not_equals(fixed_content_matter.slug.clone());
    active_model
        .excerpt
        .set_if_not_equals(fixed_content_matter.excerpt.clone());
    active_model.content.set_if_not_equals(body.to_string());

    let now = Utc::now().with_nanosecond(0).unwrap_or_else(Utc::now);
    let validator = FixedContentValidator {
        title: fixed_content_matter.title.clone(),
        slug: fixed_content_matter.slug.clone(),
        excerpt: fixed_content_matter.excerpt.clone(),
        content: body.to_string(),
        created_at: now,
        updated_at: now,
    };
    match validator.validate() {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e);
            return Err(e.into());
        }
    }

    if active_model.is_changed() {
        if let Some(utc) = Utc::now().with_nanosecond(0) {
            active_model.updated_at = Set(utc);
        }
        println!("{} updated", fixed_content_matter.title);
    }

    let saved = active_model.save(db).await?;
    let article_id: i32 = match saved.id {
        ActiveValue::Set(id) | ActiveValue::Unchanged(id) => {
            println!("{} updated", fixed_content_matter.title);
            id
        }
        ActiveValue::NotSet => return Err(DbErr::Custom("fixed content id not set".into()).into()),
    };
    Ok(article_id)
}
