use super::m20250706_065150_create_article_table::Article;
use super::m20250706_143055_create_tag_table::Tag;
use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ArticleTag::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(ArticleTag::ArticleId).not_null().integer())
                    .col(ColumnDef::new(ArticleTag::TagId).not_null().integer())
                    .primary_key(
                        Index::create()
                            .col(ArticleTag::ArticleId)
                            .col(ArticleTag::TagId),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_article_tag_article")
                            .from(ArticleTag::Table, ArticleTag::ArticleId)
                            .to(Article::Table, Article::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_article_tag_tag")
                            .from(ArticleTag::Table, ArticleTag::TagId)
                            .to(Tag::Table, Tag::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ArticleTag::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum ArticleTag {
    Table,
    ArticleId,
    TagId,
}
