use rocket::{
    Request, State,
    http::Status,
    request::{FromRequest, Outcome},
    response::Redirect,
};
use rust_blog::redirects::RedirectMap;
use sea_orm::DatabaseConnection;

pub struct AliasTarget<const ARTICLE: bool>(String);

#[rocket::async_trait]
impl<'r, const ARTICLE: bool> FromRequest<'r> for AliasTarget<ARTICLE> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let Some(map) = request.rocket().state::<RedirectMap>() else {
            return Outcome::Error((Status::InternalServerError, ()));
        };
        let Some(Ok(slug)) = request.param::<&str>(if ARTICLE { 1 } else { 0 }) else {
            return Outcome::Forward(Status::NotFound);
        };
        let entries = if ARTICLE { &map.articles } else { &map.pages };
        match entries.get(slug) {
            Some(target) => Outcome::Success(Self(target.clone())),
            None => Outcome::Forward(Status::NotFound),
        }
    }
}

#[get("/posts/<_>", rank = 1)]
pub async fn article_redirect(
    target: AliasTarget<true>,
    db: &State<DatabaseConnection>,
) -> Result<Redirect, Status> {
    rust_blog::repository::article::get_article_by_slug(db, &target.0)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    Ok(Redirect::permanent(format!(
        "/posts/{}",
        crate::utils::url_segment(&target.0)
    )))
}

#[get("/<_>", rank = 1)]
pub async fn page_redirect(
    target: AliasTarget<false>,
    db: &State<DatabaseConnection>,
) -> Result<Redirect, Status> {
    rust_blog::repository::fixed_content::get_fixed_content_by_slug(db, &target.0)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound)?;
    Ok(Redirect::permanent(format!(
        "/{}",
        crate::utils::url_segment(&target.0)
    )))
}
#[cfg(test)]
mod tests {
    use super::*;
    use rocket::{http::Status, local::asynchronous::Client};
    use rust_blog::entity::article;
    use sea_orm::{ConnectionTrait, Database, EntityTrait, Schema, Set};

    async fn content_client(map: &str) -> Client {
        use rust_blog::entity::{article_category, article_tag, category, fixed_content, tag};
        use sea_orm::ActiveModelTrait;
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        for table in [
            schema.create_table_from_entity(article::Entity),
            schema.create_table_from_entity(fixed_content::Entity),
            schema.create_table_from_entity(tag::Entity),
            schema.create_table_from_entity(category::Entity),
            schema.create_table_from_entity(article_tag::Entity),
            schema.create_table_from_entity(article_category::Entity),
        ] {
            db.execute(backend.build(&table)).await.unwrap();
        }
        let now = chrono::Utc::now() - chrono::Duration::days(1);
        article::ActiveModel {
            title: Set("Current article".into()),
            slug: Set("current-slug".into()),
            content: Set("Article body".into()),
            created_at: Set(now),
            updated_at: Set(now),
            table_of_contents: Set(false),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        fixed_content::ActiveModel {
            title: Set("Current page".into()),
            slug: Set("current-page".into()),
            content: Set("Page body".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        Client::tracked(
            rocket::custom(rocket::Config::figment().merge((
                "template_dir",
                concat!(env!("CARGO_MANIFEST_DIR"), "/../templates"),
            )))
            .manage(db)
            .manage(RedirectMap::parse(map).unwrap())
            .manage(crate::utils::config::CommonConfig {
                articles_per_page: 10,
                site_name: Some("Test Blog".into()),
                default_icatch_path: None,
                favicon_path: None,
                public_url: Some("https://example.com".into()),
            })
            .attach(rocket_dyn_templates::Template::fairing())
            .mount(
                "/",
                routes![
                    article_redirect,
                    page_redirect,
                    crate::route::get::article::article_detail,
                    crate::route::get::fixed_content::fixed_content_detail
                ],
            ),
        )
        .await
        .unwrap()
    }

    #[rocket::async_test]
    async fn non_aliases_reach_real_content_routes_with_empty_or_populated_maps() {
        for map in [
            "",
            "[articles]\n'old#article' = 'current-slug'\n[pages]\n'old#page' = 'current-page'\n",
        ] {
            let client = content_client(map).await;
            for (url, title) in [
                ("/posts/current-slug", "Current article"),
                ("/current-page", "Current page"),
            ] {
                let response = client.get(url).dispatch().await;
                assert_eq!(response.status(), Status::Ok, "{url}");
                assert!(response.headers().get_one("Location").is_none());
                assert!(response.into_string().await.unwrap().contains(title));
            }
            for url in ["/posts/missing", "/missing"] {
                assert_eq!(client.get(url).dispatch().await.status(), Status::NotFound);
            }
            if !map.is_empty() {
                for (url, target) in [
                    ("/posts/old%23article", "/posts/current-slug"),
                    ("/old%23page", "/current-page"),
                ] {
                    let response = client.get(url).dispatch().await;
                    assert_eq!(response.status(), Status::PermanentRedirect);
                    assert_eq!(response.headers().get_one("Location"), Some(target));
                }
                let db = client.rocket().state::<DatabaseConnection>().unwrap();
                article::Entity::delete_many().exec(db).await.unwrap();
                rust_blog::entity::fixed_content::Entity::delete_many()
                    .exec(db)
                    .await
                    .unwrap();
                for url in ["/posts/old%23article", "/old%23page"] {
                    assert_eq!(client.get(url).dispatch().await.status(), Status::NotFound);
                }
            }
        }
    }

    #[rocket::async_test]
    async fn database_failures_are_internal_server_errors_for_both_alias_types() {
        let client =
            content_client("[articles]\nold = 'current-slug'\n[pages]\nold = 'current-page'\n")
                .await;
        let db = client.rocket().state::<DatabaseConnection>().unwrap();
        for table in ["article", "fixed_content"] {
            db.execute(sea_orm::Statement::from_string(
                db.get_database_backend(),
                format!("DROP TABLE {table}"),
            ))
            .await
            .unwrap();
        }
        for url in ["/posts/old", "/old"] {
            let response = client.get(url).dispatch().await;
            assert_eq!(response.status(), Status::InternalServerError, "{url}");
            assert!(response.headers().get_one("Location").is_none());
        }
    }

    #[rocket::async_test]
    async fn renamed_markdown_redirects_to_encoded_server_url_only_when_published() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        db.execute(backend.build(&Schema::new(backend).create_table_from_entity(article::Entity)))
            .await
            .unwrap();
        let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let (matter, body) = rust_blog::seed::markdown::parse_markdown_to_front_matter(
            &fixture.join("content/articles/32.md"),
        )
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
