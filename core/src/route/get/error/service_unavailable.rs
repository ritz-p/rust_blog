use rocket_dyn_templates::{Template, context};

#[catch(503)]
pub fn service_unavailable() -> Template {
    Template::render(
        "503",
        context! {
            site_name: "503 Service Unavailable"
        },
    )
}
