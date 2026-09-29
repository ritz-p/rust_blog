use crate::utils::config::CommonConfig;
use rocket::{
    State,
    http::{ContentType, Status},
};
use sea_orm::DatabaseConnection;

#[get("/feed.xml")]
pub async fn feed(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
) -> Result<(ContentType, String), Status> {
    let origin = rust_blog::discovery::site_origin(config.public_url.as_deref())
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    let xml = rust_blog::feed::atom(
        db,
        &origin,
        config.site_name.as_deref().unwrap_or("Blog"),
        false,
    )
    .await
    .map_err(|_| Status::InternalServerError)?;
    Ok((ContentType::new("application", "atom+xml"), xml))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::local::asynchronous::Client;
    use sea_orm::{DatabaseBackend, MockDatabase};

    #[rocket::async_test]
    async fn serves_atom_or_404_when_disabled() {
        for enabled in [true, false] {
            let db = MockDatabase::new(DatabaseBackend::Sqlite)
                .append_query_results([Vec::<rust_blog::entity::article::Model>::new()])
                .into_connection();
            let client = Client::tracked(
                rocket::build()
                    .manage(db)
                    .manage(CommonConfig {
                        articles_per_page: 10,
                        site_name: Some("Blog".into()),
                        default_icatch_path: None,
                        favicon_path: None,
                        public_url: enabled.then(|| "https://example.com".into()),
                    })
                    .mount("/", routes![feed]),
            )
            .await
            .unwrap();
            let response = client.get("/feed.xml").dispatch().await;
            assert_eq!(
                response.status(),
                if enabled {
                    Status::Ok
                } else {
                    Status::NotFound
                }
            );
            if enabled {
                assert_eq!(
                    response.content_type(),
                    Some(ContentType::new("application", "atom+xml"))
                );
            }
        }
    }
}
