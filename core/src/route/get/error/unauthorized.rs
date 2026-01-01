use rocket_dyn_templates::{Template, context};

#[catch(401)]
pub fn unauthorized() -> Template {
    Template::render(
        "401",
        context! {
            site_name: "401 Unauthorized"
        },
    )
}
