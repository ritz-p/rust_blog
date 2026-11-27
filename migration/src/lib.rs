pub use sea_orm_migration::prelude::*;

mod m20250706_065150_create_article_table;
mod m20250706_143055_create_tag_table;
mod m20250706_144332_create_category_table;
mod m20250706_145116_create_article_tag_table;
mod m20250824_170452_create_article_category_table;
mod m20251024_190826_create_fixed_content_table;
mod m20251231_000001_create_article_icatch_path;

mod m20261108_000001_article_section;
mod m20261122_000001_article_adsense_sidebar;
mod m20261122_000002_article_adsense_auto_ads;
mod m20261126_000001_article_successor;
pub struct Migrator;
mod m20260922_000001_article_table_of_contents;
mod m20261024_000001_article_deprecated;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20250706_065150_create_article_table::Migration),
            Box::new(m20250706_143055_create_tag_table::Migration),
            Box::new(m20250706_144332_create_category_table::Migration),
            Box::new(m20250706_145116_create_article_tag_table::Migration),
            Box::new(m20250824_170452_create_article_category_table::Migration),
            Box::new(m20251024_190826_create_fixed_content_table::Migration),
            Box::new(m20251231_000001_create_article_icatch_path::Migration),
            Box::new(m20260922_000001_article_table_of_contents::Migration),
            Box::new(m20261024_000001_article_deprecated::Migration),
            Box::new(m20261108_000001_article_section::Migration),
            Box::new(m20261122_000001_article_adsense_sidebar::Migration),
            Box::new(m20261122_000002_article_adsense_auto_ads::Migration),
            Box::new(m20261126_000001_article_successor::Migration),
        ]
    }
}
