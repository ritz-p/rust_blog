use rocket::{Request, http::Status};
use rocket_dyn_templates::{Template, context};

use crate::utils::config::CommonConfig;

#[catch(404)]
pub fn not_found(_status: Status, req: &Request<'_>) -> Template {
    let favicon_path = req
        .rocket()
        .state::<CommonConfig>()
        .and_then(|config| config.favicon_path.as_deref());
    Template::render(
        "404",
        context! {
            site_name: "404 Not Found",
            favicon_path: favicon_path
        },
    )
}
