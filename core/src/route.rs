use std::collections::HashMap;

use rocket::fs::FileServer;
use rocket::{Ignite, Rocket};
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
    tag::{tag_detail, tag_list},
};

use crate::utils::config::CommonConfig;

pub async fn launch(
    db: DatabaseConnection,
    config_map: HashMap<String, String>,
) -> Result<Rocket<Ignite>, rocket::Error> {
    return rocket::build()
        .manage(db)
        .manage(CommonConfig {
            site_name: config_map.get("site_name").cloned(),
            default_icatch_path: config_map.get("default_icatch_path").cloned(),
        })
        .attach(Template::fairing())
        .mount(
            "/",
            routes![
                fixed_content_detail,
                index,
                post_detail,
                tag_list,
                tag_detail,
                category_list,
                category_detail
            ],
        )
        .mount("/image", FileServer::from("content/image"))
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
