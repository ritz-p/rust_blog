use rocket::{State, response::Redirect};
use rust_blog::redirects::RedirectMap;
use sea_orm::DatabaseConnection;

#[get("/posts/<slug>", rank = 1)]
pub async fn article_redirect(
    map: &State<RedirectMap>,
    db: &State<DatabaseConnection>,
    slug: &str,
) -> Option<Redirect> {
    let target = map.articles.get(slug)?;
    rust_blog::repository::article::get_article_by_slug(db, target)
        .await
        .ok()??;
    Some(Redirect::permanent(format!(
        "/posts/{}",
        crate::utils::url_segment(target)
    )))
}

#[get("/<slug>", rank = 1)]
pub async fn page_redirect(
    map: &State<RedirectMap>,
    db: &State<DatabaseConnection>,
    slug: &str,
) -> Option<Redirect> {
    let target = map.pages.get(slug)?;
    rust_blog::repository::fixed_content::get_fixed_content_by_slug(db, target)
        .await
        .ok()??;
    Some(Redirect::permanent(format!(
        "/{}",
        crate::utils::url_segment(target)
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::{http::Status, local::asynchronous::Client};
    use rust_blog::entity::article;
    use sea_orm::{ConnectionTrait, Database, EntityTrait, Schema, Set};

    #[rocket::async_test]
    async fn renamed_markdown_redirects_to_encoded_server_url_only_when_published() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        db.execute(backend.build(&Schema::new(backend).create_table_from_entity(article::Entity)))
            .await
            .unwrap();
        let fixture =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/redirects");
        let (matter, body) =
            rust_blog::seed::markdown::parse_markdown_to_front_matter(&fixture.join("renamed.md"))
                .unwrap();
        let id = rust_blog::seed::article::seed_article(&db, &matter, &body)
            .await
            .unwrap();
        let map =
            RedirectMap::parse(&std::fs::read_to_string(fixture.join("redirects.toml")).unwrap())
                .unwrap();
        let client = Client::tracked(
            rocket::build()
                .manage(db)
                .manage(map)
                .mount("/", routes![article_redirect]),
        )
        .await
        .unwrap();
        for source in ["old-article", "older-article"] {
            let response = client.get(format!("/posts/{source}")).dispatch().await;
            assert_eq!(response.status(), Status::PermanentRedirect);
            assert_eq!(
                response.headers().get_one("Location"),
                Some("/posts/new-%E8%A8%98%E4%BA%8B")
            );
        }
        let db = client.rocket().state::<DatabaseConnection>().unwrap();
        article::Entity::update(article::ActiveModel {
            id: Set(id),
            created_at: Set(chrono::Utc::now() + chrono::Duration::days(1)),
            ..Default::default()
        })
        .exec(db)
        .await
        .unwrap();
        assert_eq!(
            client.get("/posts/old-article").dispatch().await.status(),
            Status::NotFound
        );
        assert_eq!(
            client.get("/posts/unknown").dispatch().await.status(),
            Status::NotFound
        );
    }
}
