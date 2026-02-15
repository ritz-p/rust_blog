use rocket::{Request, http::Status};
use rocket_dyn_templates::{Template, context};

use crate::utils::config::CommonConfig;

#[catch(502)]
pub fn bad_gateway(_status: Status, req: &Request<'_>) -> Template {
    let favicon_path = req
        .rocket()
        .state::<CommonConfig>()
        .and_then(|config| config.favicon_path.as_deref());
    Template::render(
        "502",
        context! {
            site_name: "502 Bad Gateway",
            favicon_path: favicon_path
        },
    )
}
