use rocket_dyn_templates::{Template, context};

#[catch(408)]
pub fn request_timeout() -> Template {
    Template::render(
        "408",
        context! {
            site_name: "408 Request Timeout"
        },
    )
}
