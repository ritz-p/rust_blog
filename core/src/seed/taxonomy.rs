use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, Statement, TransactionTrait};
use std::collections::HashMap;

pub async fn normalize(db: &DatabaseConnection) -> anyhow::Result<()> {
    let tx = db.begin().await?;
    for (table, relation, column) in [
        ("tag", "article_tag", "tag_id"),
        ("category", "article_category", "category_id"),
    ] {
        let rows = tx
            .query_all(Statement::from_string(
                DbBackend::Sqlite,
                format!("SELECT id, slug FROM {table} ORDER BY id"),
            ))
            .await?;
        let mut groups: HashMap<String, Vec<i32>> = HashMap::new();
        for row in rows {
            let id: i32 = row.try_get("", "id")?;
            let slug: String = row.try_get("", "slug")?;
            groups.entry(slug.to_lowercase()).or_default().push(id);
        }
        for (slug, ids) in groups {
            let canonical = ids[0];
            for duplicate in &ids[1..] {
                tx.execute(Statement::from_sql_and_values(DbBackend::Sqlite,
                    format!("INSERT OR IGNORE INTO {relation} (article_id, {column}) SELECT article_id, ? FROM {relation} WHERE {column} = ?"),
                    [canonical.into(), (*duplicate).into()],
                )).await?;
                tx.execute(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    format!("DELETE FROM {relation} WHERE {column} = ?"),
                    [(*duplicate).into()],
                ))
                .await?;
                tx.execute(Statement::from_sql_and_values(
                    DbBackend::Sqlite,
                    format!("DELETE FROM {table} WHERE id = ?"),
                    [(*duplicate).into()],
                ))
                .await?;
            }
            tx.execute(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                format!("UPDATE {table} SET slug = ? WHERE id = ?"),
                [slug.into(), canonical.into()],
            ))
            .await?;
        }
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;

    #[rocket::async_test]
    async fn merges_case_variants_without_losing_or_duplicating_links() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        for (table, relation, column) in [
            ("tag", "article_tag", "tag_id"),
            ("category", "article_category", "category_id"),
        ] {
            for sql in [
                format!(
                    "CREATE TABLE {table} (id INTEGER PRIMARY KEY, name TEXT UNIQUE, slug TEXT UNIQUE)"
                ),
                format!(
                    "CREATE TABLE {relation} (article_id INTEGER, {column} INTEGER REFERENCES {table}(id), PRIMARY KEY(article_id, {column}))"
                ),
                format!("INSERT INTO {table} VALUES (1, 'Rust', 'Rust'), (2, 'rust', 'rust')"),
                format!("INSERT INTO {relation} VALUES (10, 1), (10, 2), (20, 2)"),
            ] {
                db.execute(Statement::from_string(DbBackend::Sqlite, sql))
                    .await
                    .unwrap();
            }
        }
        normalize(&db).await.unwrap();
        normalize(&db).await.unwrap();
        let matter = crate::utils::front_matter::FrontMatter::new(
            "Test".into(),
            "test".into(),
            false,
            None,
            None,
            None,
            vec!["RUST".into(), "rust".into()],
            vec!["Rust".into(), "RUST".into()],
        );
        crate::seed::article::seed_tag(&db, &matter, 10)
            .await
            .unwrap();
        crate::seed::article::seed_category(&db, &matter, 10)
            .await
            .unwrap();
        for (table, relation, column) in [
            ("tag", "article_tag", "tag_id"),
            ("category", "article_category", "category_id"),
        ] {
            let rows = db
                .query_all(Statement::from_string(
                    DbBackend::Sqlite,
                    format!("SELECT slug FROM {table}"),
                ))
                .await
                .unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].try_get::<String>("", "slug").unwrap(), "rust");
            let links = db
                .query_all(Statement::from_string(
                    DbBackend::Sqlite,
                    format!("SELECT {column} FROM {relation}"),
                ))
                .await
                .unwrap();
            assert_eq!(links.len(), 2);
            assert!(
                links
                    .iter()
                    .all(|row| row.try_get::<i32>("", column).unwrap() == 1)
            );
        }
    }
}
