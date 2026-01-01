use crate::entity;
use crate::slug_config;
use anyhow::Context;
use entity::tag;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use slug_config::SlugConfig;
use std::default::Default;

pub async fn seed_tags_from_toml(db: &DatabaseConnection, toml_path: &str) -> anyhow::Result<()> {
    let cfg = SlugConfig::from_toml_file(toml_path)
        .with_context(|| format!("failed to read slug config: {}", toml_path))?;

    for (name, slug) in cfg.map {
        let existing = tag::Entity::find()
            .filter(tag::Column::Slug.eq(slug.as_str()))
            .one(db)
            .await
            .with_context(|| format!("DB find failed for slug={}", slug))?;

        match existing {
            Some(model) => {
                if model.name != name {
                    let mut am: tag::ActiveModel = model.into();
                    am.name = Set(name.clone());
                    am.update(db)
                        .await
                        .with_context(|| format!("DB update failed for slug={}", slug))?;
                    println!("[tags] updated: slug={} name={}", slug, name);
                }
            }
            None => {
                let am = tag::ActiveModel {
                    name: Set(name.clone()),
                    slug: Set(slug.clone()),
                    ..Default::default()
                };
                am.insert(db)
                    .await
                    .with_context(|| format!("DB insert failed for slug={}", slug))?;
                println!("[tags] inserted: slug={} name={}", slug, name);
            }
        }
    }

    Ok(())
}
