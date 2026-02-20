use std::collections::HashMap;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::FileServer;
use rocket::http::Header;
use rocket::{Ignite, Request, Response, Rocket};
use rocket_dyn_templates::Template;
use sea_orm::DatabaseConnection;

mod get;

use get::{
    article::post_detail,
    category::{category_detail, category_list},
    error::{
        bad_gateway::bad_gateway, bad_request::bad_request, forbidden::forbidden,
        gateway_timeout::gateway_timeout, internal_server_error::internal_server_error,
        not_found::not_found, request_timeout::request_timeout,
        service_unavailable::service_unavailable, unauthorized::unauthorized,
    },
    fixed_component::fixed_content_detail,
    index::index,
    static_asset::{bulma_css, nav_js},
    tag::{tag_detail, tag_list},
};

use crate::utils::config::CommonConfig;

pub struct SecurityHeaders;

#[rocket::async_trait]
impl Fairing for SecurityHeaders {
    fn info(&self) -> Info {
        Info {
            name: "Security Headers",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _req: &'r Request<'_>, res: &mut Response<'r>) {
        res.set_header(Header::new("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' https: data:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'; form-action 'self'"));
        res.set_header(Header::new("Referrer-Policy", "strict-origin-when-cross-origin"));
        res.set_header(Header::new("X-Content-Type-Options", "nosniff"));
        res.set_header(Header::new("X-Frame-Options", "DENY"));
        res.set_header(Header::new("Permissions-Policy", "geolocation=(), microphone=(), camera=()"));
        res.set_header(Header::new(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains",
        ));
    }
}

pub async fn launch(
    db: DatabaseConnection,
    config_map: HashMap<String, String>,
) -> Result<Rocket<Ignite>, rocket::Error> {
    return rocket::build()
        .manage(db)
        .manage(CommonConfig {
            site_name: config_map.get("site_name").cloned(),
            default_icatch_path: config_map.get("default_icatch_path").cloned(),
            favicon_path: config_map.get("favicon_path").cloned(),
        })
        .attach(SecurityHeaders)
        .attach(Template::fairing())
        .mount(
            "/",
            routes![
                fixed_content_detail,
                index,
                post_detail,
                bulma_css,
                nav_js,
                tag_list,
                tag_detail,
                category_list,
                category_detail
            ],
        )
        .mount("/image", FileServer::from("content/image"))
        .mount("/icon", FileServer::from("content/icon"))
        .register(
            "/",
            catchers![
                not_found,
                bad_gateway,
                bad_request,
                forbidden,
                gateway_timeout,
                internal_server_error,
                request_timeout,
                service_unavailable,
                unauthorized
            ],
        )
        .launch()
        .await;
}
