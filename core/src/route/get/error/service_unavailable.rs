use rocket::{Request, State, http::Status};
use rocket_dyn_templates::{Template, context};

use crate::utils::config::CommonConfig;

#[catch(503)]
pub fn service_unavailable(_status: Status, req: &Request<'_>) -> Template {
    let favicon_path = req
        .rocket()
        .state::<CommonConfig>()
        .and_then(|config| config.favicon_path.as_deref());
    Template::render(
        "503",
        context! {
            site_name: "503 Service Unavailable",
            favicon_path: favicon_path
        },
    )
}
