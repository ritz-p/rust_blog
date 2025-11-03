use rocket::{Ignite, Rocket};
use rocket_dyn_templates::Template;
use sea_orm::DatabaseConnection;

mod get;

use get::{
    article::post_detail,
    category::{category_detail, category_list},
    error::not_found,
    fixed_component::fixed_content_detail,
    index::index,
    tag::{tag_detail, tag_list},
};

pub async fn launch(db: DatabaseConnection) -> Result<Rocket<Ignite>, rocket::Error> {
    return rocket::build()
        .manage(db)
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
        .register("/", catchers![not_found])
        .launch()
        .await;
}
