use crate::entity::{article, article_category, category};
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, ModelTrait, QueryFilter, QueryOrder,
};

pub async fn get_all_categories(db: &DatabaseConnection) -> Result<Vec<category::Model>, DbErr> {
    category::Entity::find()
        .filter(
            category::Column::Id.in_subquery(
                sea_orm::sea_query::Query::select()
                    .column(article_category::Column::CategoryId)
                    .from(article_category::Entity)
                    .and_where(
                        article_category::Column::ArticleId.in_subquery(
                            sea_orm::sea_query::Query::select()
                                .column(article::Column::Id)
                                .from(article::Entity)
                                .and_where(article::Column::CreatedAt.lte(chrono::Utc::now()))
                                .to_owned(),
                        ),
                    )
                    .to_owned(),
            ),
        )
        .order_by(category::Column::Name, sea_orm::Order::Asc)
        .all(db)
        .await
}

#[allow(dead_code)]
pub async fn get_category_by_slug(
    db: &DatabaseConnection,
    slug: &str,
) -> Result<Option<category::Model>, DbErr> {
    category::Entity::find()
        .filter(category::Column::Slug.eq(slug.to_string()))
        .one(db)
        .await
}

pub async fn get_categories_by_article(
    db: &DatabaseConnection,
    article: &article::Model,
) -> Result<Vec<category::Model>, DbErr> {
    article.find_related(category::Entity).all(db).await
}
