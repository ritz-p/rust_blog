use crate::entity;
use crate::utils;
use chrono::Timelike;
use chrono::Utc;
use entity::fixed_content::{
    Column as FixedContentColumn, Entity as FixedContentEntity, Model as FixedContentModel,
};
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
) -> Result<i32, DbErr> {
    let model: Option<FixedContentModel> = FixedContentEntity::find()
        .filter(FixedContentColumn::Slug.eq(fixed_content_matter.slug.clone()))
        .one(db)
        .await?;
    let mut active_model: entity::fixed_content::ActiveModel = match model {
        Some(model) => model.into_active_model(),
        None => Default::default(),
    };
    active_model.title = Set(fixed_content_matter.title.clone());
    active_model.slug = Set(fixed_content_matter.slug.clone());
    active_model.excerpt = Set(fixed_content_matter.excerpt.clone());
    active_model.content = Set(body.to_string());
    if check_update(&active_model) {
        if let Some(utc) = Utc::now().with_nanosecond(0) {
            active_model.updated_at = Set(utc);
        }
    }

    let saved = active_model.save(db).await?;
    let article_id: i32 = match saved.id {
        ActiveValue::Set(id) | ActiveValue::Unchanged(id) => id,
        ActiveValue::NotSet => return Err(DbErr::Custom("fixed content id not set".into())),
    };
    Ok(article_id)
}

fn check_update(active_model: &entity::fixed_content::ActiveModel) -> bool {
    let title_changed = matches!(active_model.title, Set(_));
    let content_changed = matches!(active_model.content, Set(_));
    let excerpt_changed = matches!(active_model.excerpt, Set(_));
    title_changed || content_changed || excerpt_changed
}
