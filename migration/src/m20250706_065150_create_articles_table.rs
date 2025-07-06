use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Articles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Articles::Id)
                            .integer()
                            .not_null()
                            .auto_increment()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Articles::Title).string().not_null())
                    .col(
                        ColumnDef::new(Articles::Slug)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(ColumnDef::new(Articles::Content).text().not_null())
                    .col(ColumnDef::new(Articles::CreatedAt).timestamp().not_null())
                    .col(ColumnDef::new(Articles::UpdatedAt).timestamp().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Articles::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
pub enum Articles {
    Table,
    Id,
    Title,
    Slug,
    Content,
    CreatedAt,
    UpdatedAt,
}
