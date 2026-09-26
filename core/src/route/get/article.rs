use crate::{
    repository::{
        article::{get_article_by_slug, get_latest_articles, get_surrounding_articles},
        category::get_categories_by_article,
        tag::get_tags_by_article,
    },
    utils::{
        config::CommonConfig,
        markdown::{markdown_to_html, toc},
        utc_to_jst,
    },
};
use rocket::{State, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::DatabaseConnection;
use serde_json::json;

#[get("/posts/<slug>", rank = 2)]
pub async fn article_detail(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    slug: &str,
) -> Result<Template, Status> {
    let conn = db.inner();
    let maybe = get_article_by_slug(conn, slug)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound);

    let article = match maybe {
        Ok(model) => model,
        Err(_) => return Err(Status::NotFound),
    };

    let content = markdown_to_html(&article.content);
    let content = if article.table_of_contents {
        toc(&content)
    } else {
        content
    };

    let tags: Vec<_> = get_tags_by_article(conn, &article)
        .await
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|tag| {
            let slug = tag.slug;
            json!({
                "name": tag.name,
                "slug": slug.clone(),
                "url": format!("/tag/{}", crate::utils::url_segment(&slug)),
            })
        })
        .collect();

    let categories: Vec<_> = get_categories_by_article(conn, &article)
        .await
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|category| {
            let slug = category.slug;
            json!({
                "name": category.name,
                "slug": slug.clone(),
                "url": format!("/category/{}", crate::utils::url_segment(&slug)),
            })
        })
        .collect();
    let created_at = utc_to_jst(article.created_at);
    let updated_at = utc_to_jst(article.updated_at);

    let surrounding: Vec<_> = get_surrounding_articles(db, &article)
        .await
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|model| {
            let slug = model.slug.clone();
            json!({
                "title":      model.title,
                "slug":       slug.clone(),
                "url":        format!("/posts/{}", crate::utils::url_segment(&slug)),
            })
        })
        .collect();

    let latest_articles: Vec<_> = get_latest_articles(db, 5)
        .await
        .map_err(|_| Status::InternalServerError)?
        .into_iter()
        .map(|model| {
            json!({
                "title": model.title,
                "url": format!("/posts/{}", crate::utils::url_segment(&model.slug)),
            })
        })
        .collect();

    Ok(Template::render(
        "article_detail",
        context! {
            site_name: &config.site_name,
            favicon_path: &config.favicon_path,
            tags_url: "/tags",
            categories_url: "/categories",
            about_url: "/about",
            metadata: crate::utils::metadata::article_metadata(&article, config, false),
            title: article.title,
            content_html: content,
            created_at: created_at,
            updated_at: updated_at,
            tags: &tags,
            categories: &categories,
            latest_articles: latest_articles,
            surrounding_articles: surrounding
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::article_detail;
    use crate::{
        entity::{article, category, tag},
        route::get::static_asset::site_css,
        utils::config::CommonConfig,
    };
    use chrono::Utc;
    use rocket::{
        http::{ContentType, Status},
        local::asynchronous::Client,
    };
    use rocket_dyn_templates::Template;
    use sea_orm::{DatabaseBackend, MockDatabase};

    #[rocket::async_test]
    async fn confirmation_articles_render_through_server_routes() {
        use crate::entity::{article_category, article_tag, fixed_content};
        use sea_orm::{ConnectionTrait, Database, Schema};
        let db = Database::connect("sqlite::memory:").await.unwrap();
        let backend = db.get_database_backend();
        let schema = Schema::new(backend);
        for table in [
            schema.create_table_from_entity(article::Entity),
            schema.create_table_from_entity(tag::Entity),
            schema.create_table_from_entity(category::Entity),
            schema.create_table_from_entity(article_tag::Entity),
            schema.create_table_from_entity(article_category::Entity),
            schema.create_table_from_entity(fixed_content::Entity),
        ] {
            db.execute(backend.build(&table)).await.unwrap();
        }
        let cases = [
            31, 33, 34, 35, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49,
        ];
        for number in cases {
            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("../content/articles/{number}.md"));
            let (matter, body) =
                rust_blog::seed::markdown::parse_markdown_to_front_matter(&path).unwrap();
            rust_blog::seed::article::seed_article(&db, &matter, &body)
                .await
                .unwrap();
        }
        let rocket = rocket::custom(rocket::Config::figment().merge((
            "template_dir",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../templates"),
        )))
        .manage(db)
        .manage(CommonConfig {
            articles_per_page: 10,
            site_name: Some("Fixture tests".into()),
            default_icatch_path: None,
            favicon_path: None,
            public_url: None,
        })
        .attach(Template::fairing())
        .mount("/", routes![article_detail]);
        let client = Client::tracked(rocket).await.unwrap();
        for number in cases {
            let response = client.get(format!("/posts/{number}")).dispatch().await;
            if number == 37 {
                assert_eq!(response.status(), Status::NotFound);
                continue;
            }
            assert_eq!(response.status(), Status::Ok, "article {number}");
            let html = response.into_string().await.unwrap();
            if [33, 34, 35, 39, 40, 41, 42, 43, 45, 49].contains(&number) {
                assert!(
                    html.contains("syntax-"),
                    "article {number} must be highlighted"
                );
            }
            if [31, 46].contains(&number) {
                assert!(html.contains("<nav class=\"table-of-contents\""));
            }
            if number == 38 {
                assert!(!html.contains("<nav class=\"table-of-contents\""));
            }
            if number == 48 {
                assert!(html.contains("src=\"/image/fox_girl.png\""));
            }
            if number == 47 {
                assert!(html.contains("本文がない記事"));
            }
        }
    }

    #[rocket::async_test]
    async fn article_renders_highlighted_rust_and_serves_matching_css() {
        let article = article::Model {
            id: 1,
            title: "Highlighted Rust".to_owned(),
            slug: "highlighted-rust#intro".to_owned(),
            excerpt: None,
            content: "```rust\nfn main() {\n\tlet s = \"<script> & hello\"; // comment\n\tprintln!(\"Hello,World\");\n\tlet n = 42;\n}\n```\n\n```kotlin\nfun main() { println(\"hello\") } // comment\n```\n\n```bash\n# comment\nif true; then echo \"hello\"; fi\n```\n"
                .to_owned(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            icatch_path: None,
            table_of_contents: false,
        };
        let db = MockDatabase::new(DatabaseBackend::Sqlite)
            .append_query_results([vec![article.clone()]])
            .append_query_results([Vec::<tag::Model>::new()])
            .append_query_results([Vec::<category::Model>::new()])
            .append_query_results([Vec::<article::Model>::new()])
            .append_query_results([Vec::<article::Model>::new()])
            .append_query_results([vec![article]])
            .into_connection();
        let rocket = rocket::custom(rocket::Config::figment().merge((
            "template_dir",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../templates"),
        )))
        .manage(db)
        .manage(CommonConfig {
            articles_per_page: 10,
            site_name: Some("Test Blog".to_owned()),
            default_icatch_path: None,
            favicon_path: None,
            public_url: Some("https://example.com".into()),
        })
        .attach(Template::fairing())
        .mount("/", routes![article_detail, site_css]);
        let client = Client::tracked(rocket)
            .await
            .expect("failed to build client");
        let response = client
            .get("/posts/highlighted-rust%23intro")
            .dispatch()
            .await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::HTML));
        let html = response.into_string().await.expect("missing article body");
        assert!(html.replace("&#x2F;", "/").contains(
            "rel=\"canonical\" href=\"https://example.com/posts/highlighted-rust%23intro\""
        ));
        assert!(html.contains("property=\"og:type\" content=\"article\""));
        assert!(
            html.replace("&#x2F;", "/")
                .contains("href=\"/posts/highlighted-rust%23intro\""),
            "{html}"
        );
        assert!(html.contains("href=\"/css/site.css\""));
        let content = html
            .split("<div class=\"content is-medium\">")
            .nth(1)
            .expect("missing article content")
            .split("</div>")
            .next()
            .unwrap();
        assert!(content.contains("<pre><code><span"), "{content}");
        assert!(content.contains("</code></pre>"));
        assert!(content.contains("&lt;script&gt; &amp; hello"), "{content}");
        assert!(!content.contains("<script>"));
        assert!(!content.contains("&lt;span"));
        assert!(!content.contains("style="));

        let response = client.get("/css/site.css").dispatch().await;
        assert_eq!(response.status(), Status::Ok);
        assert_eq!(response.content_type(), Some(ContentType::CSS));
        let css = response.into_string().await.expect("missing CSS body");
        assert!(
            content.split("class=\"").skip(1).all(|attribute| {
                attribute
                    .split('"')
                    .next()
                    .unwrap()
                    .split_whitespace()
                    .all(|class| class.starts_with("syntax-"))
            }),
            "{content}"
        );
        assert!(content.contains("syntax-section"), "{content}");
        assert!(content.contains("syntax-numeric"), "{content}");
        assert!(content.contains("syntax-kotlin"), "{content}");
        assert!(content.contains("syntax-shell"), "{content}");
        for scope in [
            "syntax-keyword",
            "syntax-string",
            "syntax-comment",
            "syntax-constant",
            "syntax-support",
        ] {
            assert!(
                content.split("class=\"").skip(1).any(|attribute| {
                    attribute
                        .split('"')
                        .next()
                        .unwrap()
                        .split_whitespace()
                        .any(|class| class == scope)
                }),
                "missing {scope}: {content}"
            );
            assert!(
                css.contains(&format!(".content pre code .{scope}")),
                "missing CSS for {scope}"
            );
        }
    }
}
