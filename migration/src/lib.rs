pub use sea_orm_migration::prelude::*;

mod m20250706_065150_create_article_table;
mod m20250706_143055_create_tag_table;
mod m20250706_144332_create_category_table;
mod m20250706_145116_create_article_tag_table;
mod m20250824_170452_create_article_category;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250706_065150_create_article_table::Migration),
            Box::new(m20250706_143055_create_tag_table::Migration),
            Box::new(m20250706_144332_create_category_table::Migration),
            Box::new(m20250706_145116_create_article_tag_table::Migration),
            Box::new(m20250824_170452_create_article_category::Migration),
        ]
    }
}
