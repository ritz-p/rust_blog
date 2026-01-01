use rocket::{Request, State, http::Status};
use rocket_dyn_templates::{Template, context};

use crate::utils::config::CommonConfig;

#[catch(504)]
pub fn gateway_timeout(_status: Status, req: &Request<'_>) -> Template {
    let favicon_path = req
        .rocket()
        .state::<CommonConfig>()
        .and_then(|config| config.favicon_path.as_deref());
    Template::render(
        "504",
        context! {
            site_name: "504 Gateway Timeout",
            favicon_path: favicon_path
        },
    )
}
