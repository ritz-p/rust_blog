use crate::entity;
use crate::entity::article::ActiveModel;
use crate::entity_extension;
use crate::utils;
use chrono::DateTime;
use chrono::NaiveDate;
use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::Timelike;
use chrono::Utc;
use chrono_tz::Asia::Tokyo;
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
    let now = Utc::now().with_nanosecond(0).unwrap_or_else(Utc::now);
    let model: Option<ArticleModel> = ArticleEntity::find()
        .filter(ArticleColumn::Slug.eq(front_matter.slug.clone()))
        .one(db)
        .await?;
    let existing_created_at = model.as_ref().map(|model| model.created_at);
    let mut active_model = match model {
        Some(model) => model.into_active_model(),
        None => Default::default(),
    };
    if let Some(created_at) = resolve_created_at(existing_created_at, front_matter, now)? {
        active_model.created_at.set_if_not_equals(created_at);
    }
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

fn resolve_created_at(
    existing_created_at: Option<DateTime<Utc>>,
    front_matter: &FrontMatter,
    now: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>, DbErr> {
    match front_matter.created_at.as_deref() {
        Some(raw) => Ok(Some(parse_created_at(raw)?)),
        None => {
            if existing_created_at.is_some() {
                Ok(None)
            } else {
                Ok(Some(now))
            }
        }
    }
}

fn parse_created_at(raw: &str) -> Result<DateTime<Utc>, DbErr> {
    if let Ok(dt) = DateTime::parse_from_rfc3339(raw) {
        return Ok(dt.with_timezone(&Utc));
    }

    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S") {
        let dt = Tokyo
            .from_local_datetime(&naive_dt)
            .single()
            .ok_or_else(|| DbErr::Custom(format!("invalid JST local datetime: {raw}")))?;
        return Ok(dt.with_timezone(&Utc));
    }

    if let Ok(naive_dt) = NaiveDateTime::parse_from_str(raw, "%Y-%m-%dT%H:%M:%S") {
        let dt = Tokyo
            .from_local_datetime(&naive_dt)
            .single()
            .ok_or_else(|| DbErr::Custom(format!("invalid JST local datetime: {raw}")))?;
        return Ok(dt.with_timezone(&Utc));
    }

    if let Ok(date) = NaiveDate::parse_from_str(raw, "%Y-%m-%d") {
        if let Some(naive_dt) = date.and_hms_opt(0, 0, 0) {
            let dt = Tokyo
                .from_local_datetime(&naive_dt)
                .single()
                .ok_or_else(|| DbErr::Custom(format!("invalid JST local datetime: {raw}")))?;
            return Ok(dt.with_timezone(&Utc));
        }
    }

    Err(DbErr::Custom(format!(
        "invalid created_at format: {raw} (expected RFC3339 or JST local datetime YYYY-MM-DD[ T]HH:MM:SS or YYYY-MM-DD)"
    )))
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

#[cfg(test)]
mod tests {
    use super::resolve_created_at;
    use super::parse_created_at;
    use crate::utils::front_matter::FrontMatter;
    use chrono::{TimeZone, Utc};

    fn front_matter_with_created_at(created_at: Option<&str>) -> FrontMatter {
        FrontMatter::new(
            "title".to_string(),
            "slug".to_string(),
            false,
            created_at.map(str::to_string),
            Some("excerpt".to_string()),
            None,
            vec![],
            vec![],
        )
    }

    #[test]
    fn resolve_created_at_uses_front_matter_value_when_present() {
        let fm = front_matter_with_created_at(Some("2026-01-01T12:34:56Z"));
        let now = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let result = resolve_created_at(None, &fm, now).expect("created_at should parse");
        assert_eq!(
            result,
            Some(Utc.with_ymd_and_hms(2026, 1, 1, 12, 34, 56).unwrap())
        );
    }

    #[test]
    fn resolve_created_at_uses_now_for_new_article_when_missing() {
        let fm = front_matter_with_created_at(None);
        let now = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let result = resolve_created_at(None, &fm, now).expect("should use now");
        assert_eq!(result, Some(now));
    }

    #[test]
    fn resolve_created_at_keeps_existing_for_existing_article_when_missing() {
        let fm = front_matter_with_created_at(None);
        let now = Utc.with_ymd_and_hms(2026, 2, 1, 0, 0, 0).unwrap();
        let existing = Utc.with_ymd_and_hms(2025, 2, 1, 0, 0, 0).unwrap();
        let result = resolve_created_at(Some(existing), &fm, now).expect("should keep existing");
        assert_eq!(result, None);
    }

    #[test]
    fn parse_created_at_treats_naive_datetime_as_jst() {
        let parsed = parse_created_at("2026-02-18 09:30:00").expect("must parse");
        assert_eq!(parsed, Utc.with_ymd_and_hms(2026, 2, 18, 0, 30, 0).unwrap());
    }

    #[test]
    fn parse_created_at_treats_date_only_as_jst_midnight() {
        let parsed = parse_created_at("2026-02-18").expect("must parse");
        assert_eq!(parsed, Utc.with_ymd_and_hms(2026, 2, 17, 15, 0, 0).unwrap());
    }
}
