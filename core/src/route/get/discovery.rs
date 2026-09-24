use crate::utils::config::CommonConfig;
use rocket::{
    State,
    http::Status,
    response::content::{RawText, RawXml},
};
use sea_orm::DatabaseConnection;

#[get("/sitemap.xml")]
pub async fn sitemap(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
) -> Result<RawXml<String>, Status> {
    let origin = rust_blog::discovery::site_origin(config.public_url.as_deref())
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    rust_blog::discovery::sitemap(db, &origin, false)
        .await
        .map(RawXml)
        .map_err(|_| Status::InternalServerError)
}

#[get("/robots.txt")]
pub fn robots(config: &State<CommonConfig>) -> Result<RawText<String>, Status> {
    let origin = rust_blog::discovery::site_origin(config.public_url.as_deref())
        .map_err(|_| Status::InternalServerError)?;
    Ok(RawText(rust_blog::discovery::robots(origin.as_deref())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::{http::ContentType, local::asynchronous::Client};
    use rust_blog::entity::{article, category, fixed_content, tag};
    use sea_orm::{DatabaseBackend, MockDatabase};

    #[rocket::async_test]
    async fn endpoints_have_correct_content_types_and_handle_missing_origin() {
        for public_url in [Some("https://example.com".to_string()), None] {
            let db = MockDatabase::new(DatabaseBackend::Sqlite)
                .append_query_results([Vec::<article::Model>::new()])
                .append_query_results([Vec::<fixed_content::Model>::new()])
                .append_query_results([Vec::<tag::Model>::new()])
                .append_query_results([Vec::<category::Model>::new()])
                .into_connection();
            let enabled = public_url.is_some();
            let client = Client::tracked(
                rocket::build()
                    .manage(db)
                    .manage(CommonConfig {
                        articles_per_page: 10,
                        site_name: None,
                        default_icatch_path: None,
                        favicon_path: None,
                        public_url,
                    })
                    .mount("/", routes![sitemap, robots]),
            )
            .await
            .unwrap();
            let response = client.get("/robots.txt").dispatch().await;
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(response.content_type(), Some(ContentType::Plain));
            assert_eq!(
                response
                    .into_string()
                    .await
                    .unwrap()
                    .contains("Sitemap: https://example.com/sitemap.xml"),
                enabled
            );
            let response = client.get("/sitemap.xml").dispatch().await;
            if enabled {
                assert_eq!(response.status(), Status::Ok);
                assert_eq!(response.content_type(), Some(ContentType::XML));
                let xml = response.into_string().await.unwrap();
                assert!(xml.contains("<loc>https://example.com/tags</loc>"));
            } else {
                assert_eq!(response.status(), Status::NotFound);
            }
        }
    }
}
