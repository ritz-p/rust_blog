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
                    .col(ColumnDef::new(ArticleTag::Id).not_null().integer())
                    .col(ColumnDef::new(ArticleTag::TagId).not_null().integer())
                    .primary_key(Index::create().col(ArticleTag::Id).col(ArticleTag::TagId))
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
    Id,
    TagId,
}
