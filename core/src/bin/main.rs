use anyhow::Context;
use rust_blog::entity::{
    article::Column as ArticleColumn, article::Entity as ArticleEntity,
    article::Model as ArticleModel, article_category, article_tag, category, tag,
};
use rust_blog::entity_trait::name_slug_entity::set_name_slug;
use rust_blog::entity_trait::{name_slug_entity::NameSlugEntity, name_slug_model::NameSlugModel};
use rust_blog::slug_config::SlugConfig;
use rust_blog::utils::front_matter::FrontMatter;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Database, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, ModelTrait, QueryFilter, Set, Value,
};
use serde_yaml;
use std::default::Default;
use std::fs;
use walkdir::WalkDir;

#[rocket::main]
async fn main() -> anyhow::Result<()> {
    let db = connect_db().await?;
    run_seed(&db, "content/articles").await?;
    println!("✅ Markdown → DB のシード完了");
    seed_from_toml::<tag::Entity>(&db, "content/config/slug.toml", "tags").await?;
    println!("✅ Tag Toml → DB のシード完了");
    seed_from_toml::<category::Entity>(&db, "content/config/slug.toml", "categories").await?;
    println!("✅ Category Toml → DB のシード完了");
    Ok(())
}

async fn connect_db() -> Result<DatabaseConnection, DbErr> {
    let url = std::env::var("DATABASE_URL").expect("DATABASE URL must be set");
    Database::connect(&url).await
}

async fn run_seed(db: &DatabaseConnection, dir: &str) -> Result<(), DbErr> {
    for path in markdown_files(dir) {
        println!("{:?}", path);
        let (front_matter, body) = match parse_markdown(&path) {
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

fn markdown_files(dir: &str) -> impl Iterator<Item = std::path::PathBuf> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.path().extension().and_then(|os| os.to_str()) == Some("md")
        })
        .map(|entry| entry.into_path())
}

fn parse_markdown(path: &std::path::Path) -> Result<(FrontMatter, String), serde_yaml::Error> {
    let text = fs::read_to_string(path).expect("Failed to load file");
    let parts: Vec<&str> = text.splitn(3, "---").collect();
    if parts.len() != 3 {
        panic!("FrontMatter not found in {:?}", path);
    }
    let front_matter = serde_yaml::from_str(parts[1])?;
    let body = parts[2].trim_start().to_string();
    Ok((front_matter, body))
}

async fn seed_article(
    db: &DatabaseConnection,
    front_matter: &FrontMatter,
    body: &str,
) -> Result<i32, DbErr> {
    let model: Option<ArticleModel> = ArticleEntity::find()
        .filter(ArticleColumn::Slug.eq(front_matter.slug.clone()))
        .one(db)
        .await?;
    let mut active_model = match model {
        Some(model) => model.into_active_model(),
        None => Default::default(),
    };
    active_model.title = Set(front_matter.title.clone());
    active_model.slug = Set(front_matter.slug.clone());
    active_model.content = Set(body.to_string());
    active_model.created_at = Set(front_matter.created_at.clone());
    active_model.updated_at = Set(front_matter.updated_at.clone());

    let saved = active_model.save(db).await?;
    let article_id: i32 = match saved.id {
        ActiveValue::Set(id) | ActiveValue::Unchanged(id) => id,
        ActiveValue::NotSet => return Err(DbErr::Custom("article id not set".into())),
    };
    Ok(article_id)
}

async fn seed_tag(
    db: &DatabaseConnection,
    front_matter: &FrontMatter,
    article_id: i32,
) -> Result<(), DbErr> {
    for tag_slug in &front_matter.tags {
        let existing = tag::Entity::find()
            .filter(tag::Column::Slug.eq(tag_slug.as_str()))
            .one(db)
            .await?;
        let tag_id = if let Some(m) = existing {
            m.id
        } else {
            tag::ActiveModel {
                name: Set(tag_slug.clone()),
                slug: Set(tag_slug.clone()),
                ..Default::default()
            }
            .insert(db)
            .await?
            .id
        };

        let exists_link = article_tag::Entity::find()
            .filter(article_tag::Column::ArticleId.eq(article_id))
            .filter(article_tag::Column::TagId.eq(tag_id))
            .one(db)
            .await?
            .is_some();

        if !exists_link {
            article_tag::ActiveModel {
                article_id: Set(article_id),
                tag_id: Set(tag_id),
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}

async fn seed_category(
    db: &DatabaseConnection,
    front_matter: &FrontMatter,
    article_id: i32,
) -> Result<(), DbErr> {
    for category_slug in &front_matter.categories {
        let existing = category::Entity::find()
            .filter(category::Column::Slug.eq(category_slug.as_str()))
            .one(db)
            .await?;
        let category_id = if let Some(m) = existing {
            m.id
        } else {
            category::ActiveModel {
                name: Set(category_slug.clone()),
                slug: Set(category_slug.clone()),
                ..Default::default()
            }
            .insert(db)
            .await?
            .id
        };

        let exists_link = article_category::Entity::find()
            .filter(article_category::Column::ArticleId.eq(article_id))
            .filter(article_category::Column::CategoryId.eq(category_id))
            .one(db)
            .await?
            .is_some();

        if !exists_link {
            article_category::ActiveModel {
                article_id: Set(article_id),
                category_id: Set(category_id),
            }
            .insert(db)
            .await?;
        }
    }
    Ok(())
}

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

pub async fn seed_from_toml<T>(
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
