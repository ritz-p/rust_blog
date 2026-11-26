use crate::domain::page::{Page, PageInfo};
use crate::entity::{article, category, tag};
use crate::repository::SQLITE_MAX;
use chrono::{DateTime, Datelike, Days, LocalResult, NaiveDate, TimeZone, Utc};
use chrono_tz::{GapInfo, Tz};
use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
    prelude::*,
    sea_query::{Expr, SimpleExpr},
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

    pub(crate) fn sqlite_datetime_range_filter(
        self,
        timezone: Tz,
    ) -> Result<Option<SimpleExpr>, DbErr> {
        let invalid = || DbErr::Custom("invalid article period".into());
        let (start, end) = Self::range_bounds(self.year, self.month).ok_or_else(invalid)?;
        let boundary = |date: NaiveDate| match timezone
            .from_local_datetime(&date.and_time(chrono::NaiveTime::MIN))
        {
            LocalResult::Single(date) => Ok(Some(date.to_utc())),
            LocalResult::Ambiguous(..) => Ok(None),
            LocalResult::None => GapInfo::new(&date.and_time(chrono::NaiveTime::MIN), &timezone)
                .and_then(|gap| gap.end)
                .map(|date| Some(date.to_utc()))
                .ok_or_else(invalid),
        };
        let (Some(start), Some(end)) = (boundary(start)?, boundary(end)?) else {
            return Ok(None);
        };
        Ok(Some(Expr::cust_with_values(
            "datetime(created_at) >= datetime(?) AND datetime(created_at) < datetime(?)",
            [start.naive_utc().to_string(), end.naive_utc().to_string()],
        )))
    }

    fn from_timestamp(timestamp: DateTime<Utc>, timezone: Tz) -> Self {
        let local = timestamp.with_timezone(&timezone);
        Self {
            year: local.year(),
            month: local.month(),
        }
    }
}

pub(crate) async fn articles_in_period(
    db: &DatabaseConnection,
    query: Select<article::Entity>,
    period: ArticlePeriod,
    timezone: Tz,
) -> Result<Vec<article::Model>, DbErr> {
    let (start, end) = ArticlePeriod::range_bounds(period.year, period.month)
        .ok_or_else(|| DbErr::Custom("invalid article period".into()))?;
    // UTC offsets are less than one day in magnitude. Local month membership
    // is decided below so a rollback across midnight can split the month.
    let start = start
        .checked_sub_days(Days::new(1))
        .unwrap_or(start)
        .to_string();
    let end = end
        .checked_add_days(Days::new(1))
        .unwrap_or(end)
        .to_string();
    let mut articles = query
        .filter(Expr::cust_with_values(
            "datetime(created_at) >= datetime(?) AND datetime(created_at) < datetime(?)",
            [start, end],
        ))
        .order_by_desc(article::Column::CreatedAt)
        .order_by_desc(article::Column::Id)
        .all(db)
        .await?;
    articles
        .retain(|article| ArticlePeriod::from_timestamp(article.created_at, timezone) == period);
    Ok(articles)
}

pub(crate) fn paginate(
    articles: Vec<article::Model>,
    page: Page,
) -> (Vec<article::Model>, PageInfo) {
    let info = PageInfo::new(page.normalize(SQLITE_MAX), articles.len() as u64);
    let offset = (info.current_page - 1) * info.per;
    let articles = articles
        .into_iter()
        .skip(offset as usize)
        .take(info.per as usize)
        .collect();
    (articles, info)
}

pub(crate) async fn article_page(
    db: &DatabaseConnection,
    mut query: Select<article::Entity>,
    page: Page,
    period: Option<ArticlePeriod>,
    timezone: Tz,
) -> Result<(Vec<article::Model>, PageInfo), DbErr> {
    if let Some(period) = period {
        let Some(filter) = period.sqlite_datetime_range_filter(timezone)? else {
            return Ok(paginate(
                articles_in_period(db, query, period, timezone).await?,
                page,
            ));
        };
        query = query.filter(filter);
    }
    let total = query.clone().count(db).await?;
    let info = PageInfo::new(page.normalize(SQLITE_MAX), total);
    let articles = query
        .order_by_desc(article::Column::CreatedAt)
        .order_by_desc(article::Column::Id)
        .offset((info.current_page - 1) * info.per)
        .limit(info.per)
        .all(db)
        .await?;
    Ok((articles, info))
}

pub async fn get_all_articles(
    db: &DatabaseConnection,
    page: Page,
    period: Option<ArticlePeriod>,
    timezone: Tz,
) -> Result<(Vec<article::Model>, PageInfo), DbErr> {
    let query = article::Entity::find().filter(article::Column::CreatedAt.lte(Utc::now()));
    article_page(db, query, page, period, timezone).await
}

pub async fn search_articles(
    db: &DatabaseConnection,
    page: Page,
    period: Option<ArticlePeriod>,
    query: &str,
    timezone: Tz,
) -> Result<(Vec<article::Model>, PageInfo, u64), DbErr> {
    let terms = crate::utils::search::terms(query);
    let mut base = article::Entity::find().filter(article::Column::CreatedAt.lte(Utc::now()));
    let mut ambiguous_period = None;
    if let Some(period) = period {
        match period.sqlite_datetime_range_filter(timezone)? {
            Some(filter) => base = base.filter(filter),
            None => ambiguous_period = Some(period),
        }
    }
    let articles = if let Some(period) = ambiguous_period {
        articles_in_period(db, base, period, timezone).await?
    } else {
        base.order_by_desc(article::Column::CreatedAt)
            .order_by_desc(article::Column::Id)
            .all(db)
            .await?
    };
    let matches: Vec<_> = articles
        .into_iter()
        .filter(|article| {
            let text = crate::utils::search::article_text(article);
            terms.iter().all(|term| text.contains(term))
        })
        .collect();
    let total = matches.len() as u64;
    let (articles, info) = paginate(matches, page);
    Ok((articles, info, total))
}

pub async fn get_article_periods(
    db: &DatabaseConnection,
    timezone: Tz,
) -> Result<Vec<ArticlePeriod>, DbErr> {
    let query = article::Entity::find().filter(article::Column::CreatedAt.lte(Utc::now()));
    Ok(period_counts(db, query, timezone)
        .await?
        .into_iter()
        .map(|(period, _)| period)
        .collect())
}

pub(crate) async fn period_counts(
    db: &DatabaseConnection,
    query: sea_orm::Select<article::Entity>,
    timezone: Tz,
) -> Result<Vec<(ArticlePeriod, u64)>, DbErr> {
    let created_ats: Vec<DateTime<Utc>> = query
        .select_only()
        .column(article::Column::CreatedAt)
        .order_by_desc(article::Column::CreatedAt)
        .into_tuple()
        .all(db)
        .await?;

    let mut periods = BTreeMap::new();
    for created_at in created_ats {
        let period = ArticlePeriod::from_timestamp(created_at, timezone);
        *periods.entry(period).or_default() += 1;
    }
    Ok(periods.into_iter().rev().collect())
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
                deprecated: false,
                successor: None,
                adsense_sidebar: false,
                adsense_auto_ads: false,
                section: "blog".into(),
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
                deprecated: false,
                successor: None,
                adsense_sidebar: false,
                adsense_auto_ads: false,
                section: "blog".into(),
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

    #[rocket::async_test]
    async fn rollback_across_midnight_keeps_month_counts_pages_and_search_in_sync() {
        use super::{Page, article, get_all_articles, period_counts, search_articles};
        use sea_orm::{ConnectionTrait, Database, EntityTrait, Schema};
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        db.execute(backend.build(&Schema::new(backend).create_table_from_entity(article::Entity)))
            .await
            .unwrap();

        for (slug, utc) in [
            ("first-november", "2009-11-01T02:30:00Z"),
            ("back-in-october", "2009-11-01T03:00:00Z"),
            ("second-november", "2009-11-01T03:30:00Z"),
        ] {
            db.execute(sea_orm::Statement::from_sql_and_values(
                backend,
                "INSERT INTO article (title, slug, created_at, updated_at, content, table_of_contents, deprecated, adsense_sidebar, adsense_auto_ads, section) VALUES (?, ?, ?, ?, 'rollback test', false, false, false, false, 'blog')",
                [slug.into(), slug.into(), utc.into(), utc.into()],
            )).await.unwrap();
        }

        let timezone = chrono_tz::America::St_Johns;
        let october = ArticlePeriod::new(2009, 10).unwrap();
        let november = ArticlePeriod::new(2009, 11).unwrap();
        let counts = period_counts(&db, article::Entity::find(), timezone)
            .await
            .unwrap();
        assert_eq!(counts, [(november, 2), (october, 1)]);
        for (period, number, expected, total) in [
            (october, 1, "back-in-october", 1),
            (november, 1, "second-november", 2),
            (november, 2, "first-november", 2),
        ] {
            let page = Page { number, per: 1 };
            let (articles, info) = get_all_articles(&db, page, Some(period), timezone)
                .await
                .unwrap();
            assert_eq!(info.total_pages, total);
            assert_eq!(articles[0].slug, expected);
            let (matches, _, count) =
                search_articles(&db, page, Some(period), "rollback", timezone)
                    .await
                    .unwrap();
            assert_eq!(count, total);
            assert_eq!(matches[0].slug, expected);
        }
    }
}
