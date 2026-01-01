use sea_orm_migration::{prelude::*, schema::*};

use crate::{
    m20250706_065150_create_article_table::Article,
    m20250706_144332_create_category_table::Category,
};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ArticleCategory::Table)
                    .if_not_exists()
                    .col(integer(ArticleCategory::ArticleId).not_null())
                    .col(integer(ArticleCategory::CategoryId).not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_article_category_article")
                            .from(ArticleCategory::Table, ArticleCategory::ArticleId)
                            .to(Article::Table, Article::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_article_category_category")
                            .from(ArticleCategory::Table, ArticleCategory::CategoryId)
                            .to(Category::Table, Category::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .primary_key(
                        Index::create()
                            .col(ArticleCategory::ArticleId)
                            .col(ArticleCategory::CategoryId),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ArticleCategory::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ArticleCategory {
    Table,
    ArticleId,
    CategoryId,
}
