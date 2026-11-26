use crate::{
    repository::{
        article::{get_article_by_slug, get_latest_articles, get_surrounding_articles},
        category::get_categories_by_article,
        tag::get_tags_by_article,
    },
    utils::{config::CommonConfig, format_datetime, markdown::toc},
};
use rocket::{State, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::DatabaseConnection;
use serde_json::json;

#[derive(rocket::Responder)]
pub struct ArticlePage {
    template: Template,
    policy: rocket::http::Header<'static>,
}

#[get("/posts/<slug>", rank = 2)]
pub async fn article_detail(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    adsense: &State<rust_blog::adsense::AdSense>,
    navigation: &State<rust_blog::sections::Navigation>,
    slug: &str,
) -> Result<ArticlePage, Status> {
    let conn = db.inner();
    let maybe = get_article_by_slug(conn, slug)
        .await
        .map_err(|_| Status::InternalServerError)?
        .ok_or(Status::NotFound);

    let article = match maybe {
        Ok(model) => model,
        Err(_) => return Err(Status::NotFound),
    };

    let content = rust_blog::link_cards::article_html(&article.content).await;
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
    let created_at = format_datetime(article.created_at, config.timezone);
    let updated_at = format_datetime(article.updated_at, config.timezone);

    let neighbors = get_surrounding_articles(db, &article)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let key = (article.created_at, article.id);
    let adjacent = [
        ("前の記事", neighbors.iter().find(|a| (a.created_at, a.id) < key)),
        ("次の記事", neighbors.iter().rev().find(|a| (a.created_at, a.id) > key)),
    ].into_iter().filter_map(|(label, item)| item.map(|a| json!({
        "label": label, "title": a.title, "url": format!("/posts/{}", crate::utils::url_segment(&a.slug))
    }))).collect::<Vec<_>>();
    let surrounding: Vec<_> = neighbors
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

    let article_ads = adsense
        .article_ads(article.adsense_auto_ads, article.adsense_sidebar)
        .map_err(|error| {
            error!("article {slug}: {error}");
            Status::InternalServerError
        })?;
    let policy = if article_ads.is_some() {
        rust_blog::adsense::AD_CSP.as_str()
    } else {
        rust_blog::adsense::DEFAULT_CSP.as_str()
    };
    let successor = if let Some(slug) = article.successor.as_deref() {
        get_article_by_slug(conn, slug).await.map_err(|_| Status::InternalServerError)?
            .map(|a| json!({"title": a.title, "url": format!("/posts/{}", crate::utils::url_segment(&a.slug))}))
    } else {
        None
    };
    let template = Template::render(
        "article_detail",
        context! {
            site_name: &config.site_name,
            feed_url: rust_blog::discovery::site_origin(config.public_url.as_deref()).ok().flatten().map(|_| "/feed.xml"),
            favicon_path: &config.favicon_path,
            tags_url: "/tags",
            categories_url: "/categories",
            about_url: "/about",
            metadata: crate::utils::metadata::article_metadata(&article, config, false),
            title: article.title,
            content_html: content,
            successor: successor,
            listing: navigation.article_listing(&article.section, false),
            deprecated: article.deprecated,
            created_at: created_at,
            updated_at: updated_at,
            tags: &tags,
            categories: &categories,
            latest_articles: latest_articles,
            article_ads: article_ads,
            nav_integrity: rust_blog::adsense::NAV_INTEGRITY.as_str(),
            adjacent: adjacent,
            surrounding_articles: surrounding
        },
    );
    Ok(ArticlePage {
        template,
        policy: rocket::http::Header::new("Content-Security-Policy", policy),
    })
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

    async fn fixture_db() -> sea_orm::DatabaseConnection {
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
        db
    }

    #[rocket::async_test]
    async fn adsense_publication_and_article_opt_in_match_in_both_outputs() {
        use crate::route::get::static_asset::ads_txt;
        use rust_blog::{
            adsense::AdSense,
            static_site::{ExportPaths, export_site},
        };
        use std::{fs, path::PathBuf};

        let root = std::env::temp_dir().join(format!(
            "adsense-{}-{}",
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap()
        ));
        fs::create_dir_all(&root).unwrap();
        let paths = ExportPaths {
            templates_dir: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../templates"),
            content_dir: root.join("content"),
        };
        let out = root.join("site");
        let seller = "google.com, pub-0123456789012345, DIRECT, f08c47fec0942fa0\n";
        let articles = [
            ("sidebar", false, true),
            ("auto", true, false),
            ("both", true, true),
            ("plain", false, false),
        ];
        for enabled in [true, false] {
            let db = fixture_db().await;
            for (slug, auto, sidebar) in articles {
                let yaml = format!(
                    "title: Example\nslug: {slug}\ncreated_at: '2020-01-01'\nadsense_sidebar: {sidebar}\nadsense_auto_ads: {auto}"
                );
                let matter = serde_yaml::from_str(&yaml).unwrap();
                rust_blog::seed::article::seed_article(&db, &matter, "Body")
                    .await
                    .unwrap();
            }
            let config_path = root.join("blog_config.toml");
            fs::write(&config_path, format!(
                "[common]\nsite_name = 'Test'\n[adsense]\nenabled = {enabled}\nads_txt = '''\n{seller}'''\npublisher_id = 'ca-pub-0123456789012345'\nsidebar_slot = '1234567890'"
            )).unwrap();
            let map = rust_blog::utils::config::load_config_from_file(&config_path);
            export_site(&db, &map, &out, &paths).await.unwrap();
            assert_eq!(out.join("ads.txt").exists(), enabled);
            if enabled {
                assert_eq!(fs::read_to_string(out.join("ads.txt")).unwrap(), seller);
            }
            let headers = fs::read_to_string(out.join("_headers")).unwrap();
            assert_eq!(
                headers.contains(rust_blog::adsense::AD_CSP.as_str()),
                enabled
            );
            let rocket = rocket::custom(
                rocket::Config::figment().merge(("template_dir", &paths.templates_dir)),
            )
            .manage(db)
            .manage(rust_blog::sections::Navigation::default())
            .manage(AdSense::from_config(&map).unwrap())
            .manage(CommonConfig {
                articles_per_page: 10,
                site_name: Some("Test".into()),
                description: None,
                timezone: chrono_tz::UTC,
                default_icatch_path: None,
                favicon_path: None,
                public_url: None,
            })
            .attach(Template::fairing())
            .attach(crate::route::SecurityHeaders)
            .mount("/", routes![article_detail, ads_txt]);
            let client = Client::tracked(rocket).await.unwrap();
            let response = client.get("/ads.txt").dispatch().await;
            if enabled {
                assert_eq!(response.content_type(), Some(ContentType::Plain));
                assert_eq!(response.into_string().await.as_deref(), Some(seller));
            } else {
                assert_eq!(response.status(), Status::NotFound);
            }
            for (slug, auto, sidebar) in articles {
                let show = enabled && (auto || sidebar);
                let response = client.get(format!("/posts/{slug}")).dispatch().await;
                assert_eq!(response.status(), Status::Ok);
                let csp = response
                    .headers()
                    .get_one("Content-Security-Policy")
                    .unwrap();
                assert_eq!(csp.contains("'strict-dynamic'"), show);
                let exported =
                    fs::read_to_string(out.join(format!("posts/{slug}/index.html"))).unwrap();
                assert_eq!(
                    exported.contains("http-equiv=\"Content-Security-Policy\""),
                    enabled && !show
                );
                for html in [exported, response.into_string().await.unwrap()] {
                    assert_eq!(
                        html.matches("data-ad-slot=\"1234567890\"").count(),
                        usize::from(enabled && sidebar)
                    );
                    assert_eq!(html.matches("/js/adsense.js").count(), usize::from(show));
                    assert_eq!(html.contains(" data-sidebar defer"), enabled && sidebar);
                    if enabled && sidebar {
                        assert!(
                            html.find("aria-label=\"前後の記事\"").unwrap()
                                < html.find("class=\"adsbygoogle").unwrap()
                        );
                    }
                    if show {
                        assert!(html.contains(rust_blog::adsense::AD_INTEGRITY.as_str()));
                    }
                }
            }
        }
        fs::remove_dir_all(root).unwrap();
    }

    #[rocket::async_test]
    async fn confirmation_articles_render_through_server_routes() {
        let db = fixture_db().await;
        let cases = [
            31, 33, 34, 35, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
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
        .manage(rust_blog::sections::Navigation::default())
        .manage(rust_blog::adsense::AdSense::default())
        .manage(CommonConfig {
            articles_per_page: 10,
            site_name: Some("Fixture tests".into()),
            description: None,
            timezone: chrono_tz::UTC,
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
            let notice = html.find("この記事の内容は古いです");
            assert_eq!(notice.is_some(), number == 50);
            if let Some(position) = notice {
                assert!(position < html.find("<div class=\"content is-medium\">").unwrap());
                assert!(html.contains("class=\"notification article-deprecated\" role=\"note\""));
            }
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
            deprecated: false,
            successor: None,
            adsense_sidebar: false,
            adsense_auto_ads: false,
            section: "blog".into(),
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
        .manage(rust_blog::sections::Navigation::default())
        .manage(rust_blog::adsense::AdSense::default())
        .manage(CommonConfig {
            articles_per_page: 10,
            site_name: Some("Test Blog".to_owned()),
            description: None,
            timezone: chrono_tz::UTC,
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
        assert!(html.contains("href=\"/css/site.css?v=theme-1\""));
        let content = html
            .split("<div class=\"content is-medium\">")
            .nth(1)
            .expect("missing article content")
            .split("</div>")
            .next()
            .unwrap();
        assert!(
            content.contains("<pre><code data-language=") && content.contains("<span"),
            "{content}"
        );
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
