use crate::domain::page::{Page, PageInfo};
use crate::entity::{article, category, tag};
use crate::repository::SQLITE_MAX;
use chrono::{DateTime, Datelike, NaiveDate, TimeZone, Utc};
use chrono_tz::Asia::Tokyo;
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
    prelude::*,
    sea_query::{Expr, SimpleExpr},
};

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
        let start = Tokyo
            .from_local_datetime(&start_date.and_hms_opt(0, 0, 0)?)
            .single()?
            .with_timezone(&Utc);
        let end = Tokyo
            .from_local_datetime(&end_date.and_hms_opt(0, 0, 0)?)
            .single()?
            .with_timezone(&Utc);
        Some((start, end))
    }

    fn sqlite_datetime_range_filter(self) -> Option<SimpleExpr> {
        let (start, end) = self.range()?;
        let start = start.format("%Y-%m-%d %H:%M:%S").to_string();
        let end = end.format("%Y-%m-%d %H:%M:%S").to_string();
        Some(Expr::cust_with_values(
            "datetime(created_at) >= datetime(?) AND datetime(created_at) < datetime(?)",
            [start, end],
        ))
    }
}

pub async fn get_all_articles(
    db: &DatabaseConnection,
    page: Page,
    period: Option<ArticlePeriod>,
) -> Result<(Vec<article::Model>, PageInfo), DbErr> {
    let now = Utc::now();
    let mut base_query = article::Entity::find().filter(article::Column::CreatedAt.lte(now));
    if let Some(period) = period
        && let Some(filter) = period.sqlite_datetime_range_filter()
    {
        base_query = base_query.filter(filter);
    }

    let total = base_query.clone().count(db).await?;
    let page = page.normalize(SQLITE_MAX);
    let page_info = PageInfo::new(page, total);
    let offset = (page_info.current_page - 1) * page_info.per;
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
    let now = Utc::now();
    let mut query = article::Entity::find().filter(article::Column::CreatedAt.lte(now));
    if let Some(period) = period
        && let Some(filter) = period.sqlite_datetime_range_filter()
    {
        query = query.filter(filter);
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
        let created_at_jst = created_at.with_timezone(&Tokyo);
        let period = ArticlePeriod {
            year: created_at_jst.year(),
            month: created_at_jst.month(),
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
    let now = Utc::now();
    article::Entity::find()
        .filter(article::Column::Slug.eq(slug.to_string()))
        .filter(article::Column::CreatedAt.lte(now))
        .one(db)
        .await
}

#[allow(dead_code)]
pub async fn get_all_published_articles(
    db: &DatabaseConnection,
) -> Result<Vec<article::Model>, DbErr> {
    let now = Utc::now();
    article::Entity::find()
        .filter(article::Column::CreatedAt.lte(now))
        .order_by_desc(article::Column::CreatedAt)
        .all(db)
        .await
}

pub async fn get_latest_articles(
    db: &DatabaseConnection,
    limit: u64,
) -> Result<Vec<article::Model>, DbErr> {
    let now = Utc::now();
    let articles = article::Entity::find()
        .filter(article::Column::CreatedAt.lte(now))
        .order_by_desc(article::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await?;
    Ok(articles)
}

#[allow(dead_code)]
pub fn surrounding_articles(articles: &[article::Model], index: usize) -> Vec<&article::Model> {
    if index >= articles.len() {
        return Vec::new();
    }
    articles[index.saturating_sub(3)..(index + 4).min(articles.len())]
        .iter()
        .filter(|a| a.id != articles[index].id)
        .collect()
}

pub async fn get_surrounding_articles(
    db: &DatabaseConnection,
    current: &article::Model,
) -> Result<Vec<article::Model>, DbErr> {
    let base = article::Entity::find().filter(article::Column::CreatedAt.lte(Utc::now()));
    let mut newer = base
        .clone()
        .filter(
            Condition::any()
                .add(article::Column::CreatedAt.gt(current.created_at))
                .add(
                    Condition::all()
                        .add(article::Column::CreatedAt.eq(current.created_at))
                        .add(article::Column::Id.gt(current.id)),
                ),
        )
        .order_by_asc(article::Column::CreatedAt)
        .order_by_asc(article::Column::Id)
        .limit(3)
        .all(db)
        .await?;
    newer.reverse();
    let older = base
        .filter(
            Condition::any()
                .add(article::Column::CreatedAt.lt(current.created_at))
                .add(
                    Condition::all()
                        .add(article::Column::CreatedAt.eq(current.created_at))
                        .add(article::Column::Id.lt(current.id)),
                ),
        )
        .order_by_desc(article::Column::CreatedAt)
        .order_by_desc(article::Column::Id)
        .limit(3)
        .all(db)
        .await?;
    newer.extend(older);
    Ok(newer)
}

pub async fn get_articles_by_tag_slug(
    db: &DatabaseConnection,
    page: Page,
    tag_slug: &str,
    sort_key: &str,
) -> Result<(Vec<article::Model>, PageInfo), DbErr> {
    let now = Utc::now();
    if let Some(tag) = tag::Entity::find()
        .filter(tag::Column::Slug.eq(tag_slug))
        .one(db)
        .await?
    {
        let total = tag
            .find_related(article::Entity)
            .filter(article::Column::CreatedAt.lte(now))
            .distinct()
            .count(db)
            .await?;
        let page = page.normalize(SQLITE_MAX);
        let page_info = PageInfo::new(page, total);
        let offset = (page_info.current_page - 1) * page_info.per;
        let articles = match sort_key {
            "updated_at" => {
                tag.find_related(article::Entity)
                    .filter(article::Column::CreatedAt.lte(now))
                    .distinct()
                    .order_by_desc(article::Column::UpdatedAt)
                    .offset(offset)
                    .limit(page_info.per)
                    .all(db)
                    .await?
            }
            "created_at" => {
                tag.find_related(article::Entity)
                    .filter(article::Column::CreatedAt.lte(now))
                    .distinct()
                    .order_by_desc(article::Column::CreatedAt)
                    .offset(offset)
                    .limit(page_info.per)
                    .all(db)
                    .await?
            }
            _ => {
                tag.find_related(article::Entity)
                    .filter(article::Column::CreatedAt.lte(now))
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
    let now = Utc::now();
    if let Some(category) = category::Entity::find()
        .filter(category::Column::Slug.eq(category_slug))
        .one(db)
        .await?
    {
        let total = category
            .find_related(article::Entity)
            .filter(article::Column::CreatedAt.lte(now))
            .distinct()
            .count(db)
            .await?;
        let page = page.normalize(SQLITE_MAX);
        let page_info = PageInfo::new(page, total);
        let offset = (page_info.current_page - 1) * page_info.per;
        let articles = match sort_key {
            "updated_at" => {
                category
                    .find_related(article::Entity)
                    .filter(article::Column::CreatedAt.lte(now))
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
                    .filter(article::Column::CreatedAt.lte(now))
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
                    .filter(article::Column::CreatedAt.lte(now))
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
    #[rocket::async_test]
    async fn database_neighbors_match_export_and_exclude_future_articles() {
        use crate::entity::article;
        use sea_orm::{ConnectionTrait, Database, EntityTrait, IntoActiveModel, Schema};

        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        db.execute(backend.build(&Schema::new(backend).create_table_from_entity(article::Entity)))
            .await
            .unwrap();
        let now = chrono::Utc::now();
        let mut published = Vec::new();
        for id in 1..=10 {
            let created_at = if id == 10 {
                now + chrono::Duration::days(1)
            } else {
                now - chrono::Duration::days(10 - i64::from(id / 3))
            };
            let model = article::Model {
                id,
                title: id.to_string(),
                slug: id.to_string(),
                content: "body".repeat(1000),
                excerpt: None,
                icatch_path: None,
                table_of_contents: false,
                created_at,
                updated_at: now,
            };
            article::Entity::insert(model.clone().into_active_model())
                .exec(&db)
                .await
                .unwrap();
            if id != 10 {
                published.push(model);
            }
        }
        published.sort_by_key(|a| std::cmp::Reverse((a.created_at, a.id)));
        for (index, current) in published.iter().enumerate() {
            let actual = super::get_surrounding_articles(&db, current).await.unwrap();
            let expected = super::surrounding_articles(&published, index);
            assert_eq!(
                actual.iter().map(|a| a.id).collect::<Vec<_>>(),
                expected.iter().map(|a| a.id).collect::<Vec<_>>()
            );
            assert!(actual.len() <= 6);
            assert!(actual.iter().all(|a| a.id != current.id && a.id != 10));
        }
    }

    #[test]
    fn neighbors_shrink_at_both_ends_and_break_timestamp_ties() {
        let now = chrono::Utc::now();
        let articles: Vec<_> = (1..=9)
            .rev()
            .map(|id| crate::entity::article::Model {
                id,
                title: id.to_string(),
                slug: id.to_string(),
                content: String::new(),
                excerpt: None,
                icatch_path: None,
                table_of_contents: false,
                created_at: now,
                updated_at: now,
            })
            .collect();
        for (id, expected) in [
            (9, vec![8, 7, 6]),
            (8, vec![9, 7, 6, 5]),
            (7, vec![9, 8, 6, 5, 4]),
            (5, vec![8, 7, 6, 4, 3, 2]),
            (3, vec![6, 5, 4, 2, 1]),
            (2, vec![5, 4, 3, 1]),
            (1, vec![4, 3, 2]),
        ] {
            let actual: Vec<_> = super::surrounding_articles(&articles, 9 - id)
                .iter()
                .map(|a| a.id)
                .collect();
            assert_eq!(actual, expected);
        }
        assert!(super::surrounding_articles(&articles[..1], 0).is_empty());
        assert!(super::surrounding_articles(&[], 1).is_empty());
    }
    use super::ArticlePeriod;
    use chrono::{TimeZone, Utc};

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

    #[test]
    fn article_period_range_uses_jst_month_boundaries() {
        let period = ArticlePeriod::new(2026, 2).expect("valid period");
        let (start, end) = period.range().expect("range should exist");
        assert_eq!(start, Utc.with_ymd_and_hms(2026, 1, 31, 15, 0, 0).unwrap());
        assert_eq!(end, Utc.with_ymd_and_hms(2026, 2, 28, 15, 0, 0).unwrap());
    }
}
