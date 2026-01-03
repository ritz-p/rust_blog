use crate::{
    entity_trait::{
        name_slug_entity::{NameSlugEntity, set_name_slug},
        name_slug_model::NameSlugModel,
    },
    slug_config::SlugConfig,
};
use anyhow::Context;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Value,
};

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
        match T::find()
            .filter(T::col_slug().eq(slug.as_str()))
            .one(db)
            .await
            .with_context(|| format!("DB find failed for slug={}", slug))?
        {
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
