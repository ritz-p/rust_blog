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
                    .col(string(Article::Title).not_null())
                    .col(string_uniq(Article::Slug).not_null())
                    .col(string_null(Article::Excerpt))
                    .col(text(Article::Content).not_null())
                    .col(
                        timestamp(Article::CreatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
                    .col(
                        timestamp(Article::UpdatedAt)
                            .timestamp()
                            .default(Expr::current_timestamp())
                            .not_null(),
                    )
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
    Excerpt,
    Content,
    CreatedAt,
    UpdatedAt,
}
