pub use sea_orm_migration::prelude::*;

mod m20250706_065150_create_articles_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m20250706_065150_create_articles_table::Migration)]
    }
}
