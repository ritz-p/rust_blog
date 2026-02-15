use rocket::{Request, http::Status};
use rocket_dyn_templates::{Template, context};

use crate::utils::config::CommonConfig;

#[catch(500)]
pub fn internal_server_error(_status: Status, req: &Request<'_>) -> Template {
    let favicon_path = req
        .rocket()
        .state::<CommonConfig>()
        .and_then(|config| config.favicon_path.as_deref());
    Template::render(
        "500",
        context! {
            site_name: "500 Internal Server Error",
            favicon_path: favicon_path
        },
    )
}
