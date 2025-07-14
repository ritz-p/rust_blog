use rust_blog::entity::article::Model;
use rust_blog::entity::{
    article::ActiveModel as ArticleActiveModel, article::Column as ArticleColumn,
    article::Entity as ArticleEntity, article::Model as ArticleModel,
    article_tag::ActiveModel as ArticleTagActiveModel, tag, tag::ActiveModel as TagActiveModel,
    tag::Entity as TagEntity,
};
use rust_blog::utils::front_matter::FrontMatter;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, Database, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, Set,
};
use serde_yaml;
use std::default::Default;
use std::fs;
use walkdir::WalkDir;
#[rocket::main]
async fn main() -> Result<(), DbErr> {
    let db = connect_db().await?;
    run_seed(&db, "content/articles").await?;
    println!("✅ Markdown → DB のシード完了");
    Ok(())
}

async fn connect_db() -> Result<DatabaseConnection, DbErr> {
    let url = std::env::var("DATABASE_URL").expect("DATABASE URL must be set");
    Database::connect(&url).await
}

async fn run_seed(db: &DatabaseConnection, dir: &str) -> Result<(), DbErr> {
    for path in markdown_files(dir) {
        let (front_matter, body) = match parse_markdown(&path) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("FrontMatter parse error {:?}", e);
                continue;
            }
        };

        let article_id = seed_article(db, &front_matter, &body).await?;
        seed_tag(db, front_matter, article_id).await?;
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
    front_matter: FrontMatter,
    article_id: i32,
) -> Result<(), DbErr> {
    for tag_slug in &front_matter.tags {
        let tag_model = TagEntity::find()
            .filter(tag::Column::Slug.eq(tag_slug.clone()))
            .one(db)
            .await?;
        let tag_active_model = match tag_model {
            Some(tag) => tag.into_active_model(),
            None => {
                let mut active_model: TagActiveModel = Default::default();
                active_model.slug = Set(tag_slug.clone());
                active_model.name = Set(tag_slug.clone());
                active_model.save(db).await?
            }
        };
        let tag_id: i32 = match tag_active_model.id {
            ActiveValue::Set(id) | ActiveValue::Unchanged(id) => id,
            ActiveValue::NotSet => return Err(DbErr::Custom("tag id not set".into())),
        };

        let mut article_tag_active_model: ArticleTagActiveModel = Default::default();
        article_tag_active_model.article_id = Set(article_id);
        article_tag_active_model.tag_id = Set(tag_id);
        article_tag_active_model.save(db).await?;
    }
    Ok(())
}
