use anyhow::{Context, Result, bail, ensure};
use rust_blog::redirects::RedirectMap;
use std::process::ExitCode;

fn readonly_url(url: &str) -> Result<String> {
    ensure!(
        url.starts_with("sqlite:"),
        "only SQLite databases are supported"
    );
    let (path, query) = url.split_once('?').unwrap_or((url, ""));
    ensure!(
        !path.contains(":memory:"),
        "an existing database file is required"
    );
    let mut params: Vec<_> = url::form_urlencoded::parse(query.as_bytes())
        .filter(|(key, _)| key != "mode")
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect();
    params.push(("mode".into(), "ro".into()));
    Ok(format!(
        "{path}?{}",
        url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(params)
            .finish()
    ))
}

#[rocket::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<()> {
    let mut args = std::env::args().skip(1).collect::<Vec<_>>().into_iter();
    let mut map_path = "redirects.toml".to_owned();
    let mut database = std::env::var("DATABASE_URL").ok();
    let mut syntax_only = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--map" => map_path = args.next().context("--map requires a path")?,
            "--database-url" => {
                database = Some(args.next().context("--database-url requires a URL")?)
            }
            "--syntax-only" => syntax_only = true,
            "--help" | "-h" => {
                println!(
                    "validate_redirects [--map redirects.toml] [--database-url sqlite://blog.db] [--syntax-only]\nDATABASE_URL is used by default. The database is opened read-only; no migrations or writes are performed."
                );
                return Ok(());
            }
            _ => bail!("unknown argument: {arg}"),
        }
    }
    let text = std::fs::read_to_string(&map_path).with_context(|| format!("read {map_path}"))?;
    let map = RedirectMap::parse(&text).with_context(|| format!("validate {map_path}"))?;
    if !syntax_only {
        let url = readonly_url(
            database
                .as_deref()
                .context("set DATABASE_URL, --database-url, or --syntax-only")?,
        )?;
        let db = sea_orm::Database::connect(&url)
            .await
            .context("open existing database read-only")?;
        map.validate_database(&db).await?;
        db.close().await?;
    }
    println!(
        "Redirects valid: {} article(s), {} page(s){}",
        map.articles.len(),
        map.pages.len(),
        if syntax_only {
            " (database not checked)"
        } else {
            ""
        }
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forces_read_only_without_losing_other_options() {
        assert_eq!(
            readonly_url("sqlite://blog.db?mode=rwc&cache=shared").unwrap(),
            "sqlite://blog.db?cache=shared&mode=ro"
        );
        assert!(readonly_url("postgres://localhost/db").is_err());
        assert!(readonly_url("sqlite::memory:").is_err());
    }

    #[test]
    fn reports_independent_invalid_entries_together() {
        let error = RedirectMap::parse(
            "[articles]\n'bad/path' = 'target'\na = 'b'\nb = 'a'\n[pages]\nother = 'CON'",
        )
        .unwrap_err()
        .to_string();
        for expected in ["bad/path", "cycle", "CON"] {
            assert!(error.contains(expected), "{error}");
        }
    }

    #[rocket::async_test]
    async fn reports_all_missing_database_targets() {
        use sea_orm::{ConnectionTrait, Schema};
        let db = sea_orm::Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        for table in [
            schema.create_table_from_entity(rust_blog::entity::article::Entity),
            schema.create_table_from_entity(rust_blog::entity::fixed_content::Entity),
        ] {
            db.execute(backend.build(&table)).await.unwrap();
        }
        let map =
            RedirectMap::parse("[articles]\none = 'missing-one'\n[pages]\ntwo = 'missing-two'")
                .unwrap();
        let error = map.validate_database(&db).await.unwrap_err().to_string();
        assert!(error.contains("missing-one") && error.contains("missing-two"));
    }
}
