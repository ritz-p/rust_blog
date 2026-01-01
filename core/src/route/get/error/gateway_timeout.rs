use rocket_dyn_templates::{Template, context};

#[catch(504)]
pub fn gateway_timeout() -> Template {
    Template::render(
        "504",
        context! {
            site_name: "504 Gateway Timeout"
        },
    )
}
