use crate::domain::page::{Page, PageInfo};
use chrono::{DateTime, Datelike, NaiveDate, Utc};
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    prelude::*,
};

use crate::entity::{article, category, tag};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArticlePeriod {
    pub year: i32,
    pub month: u32,
}

impl ArticlePeriod {
    pub fn new(year: i32, month: u32) -> Option<Self> {
        if Self::range_bounds(year, month).is_some() {
            Some(Self { year, month })
        } else {
            None
        }
    }

    fn range_bounds(year: i32, month: u32) -> Option<(NaiveDate, NaiveDate)> {
        let start_date = NaiveDate::from_ymd_opt(year, month, 1)?;
        let (next_year, next_month) = if month == 12 {
            (year.checked_add(1)?, 1)
        } else {
            (year, month + 1)
        };
        let end_date = NaiveDate::from_ymd_opt(next_year, next_month, 1)?;
        Some((start_date, end_date))
    }

    fn range(self) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
        let (start_date, end_date) = Self::range_bounds(self.year, self.month)?;
        let start =
            DateTime::<Utc>::from_naive_utc_and_offset(start_date.and_hms_opt(0, 0, 0)?, Utc);
        let end = DateTime::<Utc>::from_naive_utc_and_offset(end_date.and_hms_opt(0, 0, 0)?, Utc);
        Some((start, end))
    }
}

pub async fn get_all_articles(
    db: &DatabaseConnection,
    page: Page,
    period: Option<ArticlePeriod>,
) -> Result<(Vec<article::Model>, PageInfo), DbErr> {
    let mut base_query = article::Entity::find();
    if let Some(period) = period {
        if let Some((start, end)) = period.range() {
            base_query = base_query
                .filter(article::Column::CreatedAt.gte(start))
                .filter(article::Column::CreatedAt.lt(end));
        }
    }

    let total = base_query.clone().count(db).await?;
    let page = page.normalize(50);
    let page_info = PageInfo::new(page, total);
    let offset = (page_info.count - 1) * page_info.per;
    let articles = base_query
        .order_by_desc(article::Column::CreatedAt)
        .offset(offset)
        .limit(page_info.per)
        .all(db)
        .await?;
    Ok((articles, page_info))
}

pub async fn get_article_periods(
    db: &DatabaseConnection,
    period: Option<ArticlePeriod>,
) -> Result<Vec<ArticlePeriod>, DbErr> {
    let mut query = article::Entity::find();
    if let Some(period) = period {
        if let Some((start, end)) = period.range() {
            query = query
                .filter(article::Column::CreatedAt.gte(start))
                .filter(article::Column::CreatedAt.lt(end));
        }
    }

    let created_ats: Vec<DateTime<Utc>> = query
        .select_only()
        .column(article::Column::CreatedAt)
        .order_by_desc(article::Column::CreatedAt)
        .into_tuple()
        .all(db)
        .await?;

    let mut periods = Vec::<ArticlePeriod>::new();
    for created_at in created_ats {
        let period = ArticlePeriod {
            year: created_at.year(),
            month: created_at.month(),
        };
        if periods.last().copied() != Some(period) {
            periods.push(period);
        }
    }
    Ok(periods)
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

#[cfg(test)]
mod tests {
    use super::ArticlePeriod;

    #[test]
    fn article_period_new_rejects_invalid_month() {
        assert!(ArticlePeriod::new(2025, 0).is_none());
        assert!(ArticlePeriod::new(2025, 13).is_none());
    }

    #[test]
    fn article_period_new_rejects_out_of_range_year() {
        assert!(ArticlePeriod::new(999_999, 1).is_none());
        assert!(ArticlePeriod::new(i32::MAX, 12).is_none());
    }

    #[test]
    fn article_period_new_accepts_valid_year_month() {
        assert_eq!(
            ArticlePeriod::new(2025, 12),
            Some(ArticlePeriod {
                year: 2025,
                month: 12
            })
        );
    }
}
