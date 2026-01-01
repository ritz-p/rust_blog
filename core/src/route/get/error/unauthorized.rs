use rocket::{Request, State, http::Status};
use rocket_dyn_templates::{Template, context};

use crate::utils::config::CommonConfig;

#[catch(401)]
pub fn unauthorized(_status: Status, req: &Request<'_>) -> Template {
    let favicon_path = req
        .rocket()
        .state::<CommonConfig>()
        .and_then(|config| config.favicon_path.as_deref());
    Template::render(
        "401",
        context! {
            site_name: "401 Unauthorized",
            favicon_path: favicon_path
        },
    )
}
