use rocket_dyn_templates::{Template, context};

#[catch(404)]
pub fn not_found() -> Template {
    Template::render(
        "404",
        context! {
            site_name: "404 Not Found"
        },
    )
}
