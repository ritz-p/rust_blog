pub mod blog_component;
pub mod config;
pub mod fixed_content;
pub mod markdown;
pub mod tag_toml;
use crate::{
    entity::{category, tag},
    entity_trait::{
        name_slug_entity::{NameSlugEntity, set_name_slug},
        name_slug_model::NameSlugModel,
    },
    seed::{
        config::{PathConfig, PathConfigTrait},
        fixed_content::seed_fixed_content,
        markdown::{
            markdown_files, parse_markdown_to_fixed_content_matter, parse_markdown_to_front_matter,
        },
    },
    slug_config::SlugConfig,
};
use anyhow::{Context, Error};
use blog_component::{seed_article, seed_category, seed_tag};
use dotenvy::dotenv;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel,
    QueryFilter, Value,
};
use std::{env, fs};
use toml;

pub async fn run_all(db: DatabaseConnection) -> anyhow::Result<()> {
    let mut config = load_env();
    println!("{:?}", config);
    config = load_config_toml("blog_config.toml", config)?;
    println!("{:?}", config);
    run_fixed_content_seed(&db, &config.fixed_content_path.unwrap()).await?;
    println!("✅ 固定ページ Markdown → DB のシード完了");
    run_article_seed(&db, &config.article_path.unwrap()).await?;
    println!("✅ Article Markdown → DB のシード完了");
    seed_from_toml::<tag::Entity>(&db, &config.tag_config_toml_path.unwrap(), "tags").await?;
    println!("✅ Tag Toml → DB のシード完了");
    seed_from_toml::<category::Entity>(
        &db,
        &config.category_config_toml_path.unwrap(),
        "categories",
    )
    .await?;
    println!("✅ Category Toml → DB のシード完了");

    Ok(())
}

async fn run_article_seed(db: &DatabaseConnection, dir: &str) -> Result<(), DbErr> {
    for path in markdown_files(dir) {
        println!("{:?}", path);
        let (front_matter, body) = match parse_markdown_to_front_matter(&path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("FrontMatter parse error {:?}", e);
                continue;
            }
        };

        let article_id = seed_article(db, &front_matter, &body).await?;
        seed_tag(db, &front_matter, article_id).await?;
        seed_category(db, &front_matter, article_id).await?;
    }
    Ok(())
}

async fn run_fixed_content_seed(db: &DatabaseConnection, dir: &str) -> Result<(), DbErr> {
    for path in markdown_files(dir) {
        println!("{:?}", path);
        let (front_matter, body) = match parse_markdown_to_fixed_content_matter(&path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("FrontMatter parse error {:?}", e);
                continue;
            }
        };

        seed_fixed_content(db, &front_matter, &body).await?;
    }
    Ok(())
}

async fn seed_from_toml<T>(
    db: &DatabaseConnection,
    toml_path: &str,
    entity_name: &str,
) -> anyhow::Result<()>
where
    T: EntityTrait + NameSlugEntity,
    T::Column: ColumnTrait + Copy,
    T::Model: NameSlugModel,
    T::ActiveModel: ActiveModelTrait + Default,
    <T as EntityTrait>::ActiveModel: From<<T as EntityTrait>::Model>,
    <T as EntityTrait>::Model: IntoActiveModel<<T as EntityTrait>::ActiveModel>,
    <T as EntityTrait>::ActiveModel: std::marker::Send,
{
    let cfg = SlugConfig::from_toml_file_key(toml_path, entity_name)
        .with_context(|| format!("failed to read slug config: {}", toml_path))?;

    for (name, slug) in cfg.map {
        let existing = T::find()
            .filter(T::col_slug().eq(slug.as_str()))
            .one(db)
            .await
            .with_context(|| format!("DB find failed for slug={}", slug))?;

        match existing {
            Some(model) => {
                if model.name() != name {
                    let mut am: T::ActiveModel = model.into();
                    am.set(T::col_name(), Value::from(name.clone()));
                    am.set(T::col_slug(), Value::from(slug.clone()));

                    am.update(db)
                        .await
                        .with_context(|| format!("DB update failed for slug={}", slug))?;
                    println!("[{}] updated: slug={} name={}", entity_name, slug, name);
                }
            }
            None => {
                let mut am: T::ActiveModel = Default::default();
                set_name_slug::<T>(&mut am, &name, &slug);
                am.insert(db)
                    .await
                    .with_context(|| format!("DB insert failed for slug={}", slug))?;
                println!("[{}] inserted: slug={} name={}", entity_name, slug, name);
            }
        }
    }

    Ok(())
}

fn load_env() -> PathConfig {
    dotenv().expect(".env not found");
    PathConfig::new(
        env::var("FIXED_CONTENT_PATH").ok().or_else(|| None),
        env::var("ARTICLE_PATH").ok().or_else(|| None),
        env::var("TAG_CONFIG_TOML_PATH").ok().or_else(|| None),
        env::var("CATEGORY_CONFIG_TOML_PATH").ok().or_else(|| None),
    )
}

fn load_config_toml(path: &str, mut config: PathConfig) -> Result<PathConfig, Error> {
    let toml = fs::read_to_string(path)?;
    let value: toml::Value = toml::from_str(&toml)?;
    let new_config = value.get("path_config").unwrap().clone().try_into()?;
    println!("{:?}", new_config);
    config.update(new_config);
    Ok(config)
}
