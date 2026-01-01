use rocket_dyn_templates::{Template, context};

#[catch(500)]
pub fn internal_server_error() -> Template {
    Template::render(
        "500",
        context! {
            site_name: "500 Internal Server Error"
        },
    )
}
