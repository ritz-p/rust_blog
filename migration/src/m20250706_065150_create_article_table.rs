use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Article::Table)
                    .if_not_exists()
                    .col(pk_auto(Article::Id))
                    .col(ColumnDef::new(Article::Title).string().not_null())
                    .col(string_uniq(Article::Slug).not_null())
                    .col(ColumnDef::new(Article::Content).text().not_null())
                    .col(ColumnDef::new(Article::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Article::UpdatedAt).timestamp().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Article::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Article {
    Table,
    Id,
    Title,
    Slug,
    Content,
    CreatedAt,
    UpdatedAt,
}
