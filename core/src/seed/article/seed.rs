use crate::entity;
use crate::entity::article::ActiveModel;
use crate::entity_extension;
use crate::utils;
use chrono::Timelike;
use chrono::Utc;
use entity::{
    article::Column as ArticleColumn, article::Entity as ArticleEntity,
    article::Model as ArticleModel,
};
use entity_extension::article::ArticleValidator;
use garde::Report;
use garde::Validate;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, Set,
};
use std::default::Default;
use utils::front_matter::FrontMatter;

pub async fn prepare(
    db: &DatabaseConnection,
    front_matter: &FrontMatter,
    body: &str,
) -> Result<ActiveModel, DbErr> {
    let model: Option<ArticleModel> = ArticleEntity::find()
        .filter(ArticleColumn::Slug.eq(front_matter.slug.clone()))
        .one(db)
        .await?;
    let mut active_model = match model {
        Some(model) => model.into_active_model(),
        None => Default::default(),
    };
    active_model
        .title
        .set_if_not_equals(front_matter.title.clone());
    active_model
        .slug
        .set_if_not_equals(front_matter.slug.clone());
    active_model
        .excerpt
        .set_if_not_equals(front_matter.excerpt.clone());
    active_model
        .icatch_path
        .set_if_not_equals(front_matter.icatch_path.clone());
    active_model.content.set_if_not_equals(body.to_string());
    Ok(active_model)
}

pub fn validate(front_matter: &FrontMatter, body: &str) -> Result<(), Report> {
    let now = Utc::now().with_nanosecond(0).unwrap_or_else(Utc::now);

    let validator = ArticleValidator {
        title: front_matter.title.clone(),
        slug: front_matter.slug.clone(),
        excerpt: front_matter.excerpt.clone(),
        icatch_path: front_matter.icatch_path.clone(),
        content: body.to_string(),
        created_at: now,
        updated_at: now,
    };
    match validator.validate() {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("{:?}", e);
            return Err(e.into());
        }
    }
}

pub async fn upsert(db: &DatabaseConnection, mut active_model: ActiveModel) -> Result<i32, DbErr> {
    if active_model.is_changed() {
        if let Some(utc) = Utc::now().with_nanosecond(0) {
            active_model.updated_at = Set(utc);
        }
    }
    let saved: ActiveModel = active_model.save(db).await?;
    match saved.id {
        ActiveValue::Set(id) | ActiveValue::Unchanged(id) => Ok(id),
        ActiveValue::NotSet => return Err(DbErr::Custom("article id not set".into()).into()),
    }
}
