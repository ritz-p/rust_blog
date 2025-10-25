use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FixedContent::Table)
                    .if_not_exists()
                    .col(pk_auto(FixedContent::Id))
                    .col(string(FixedContent::Title))
                    .col(string(FixedContent::Content))
                    .col(string_uniq(FixedContent::Slug).not_null())
                    .col(
                        ColumnDef::new(FixedContent::CreatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FixedContent::UpdatedAt)
                            .timestamp()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FixedContent::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum FixedContent {
    Table,
    Id,
    Title,
    Content,
    Slug,
    CreatedAt,
    UpdatedAt,
}
