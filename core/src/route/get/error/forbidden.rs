use rocket_dyn_templates::{Template, context};

#[catch(403)]
pub fn forbidden() -> Template {
    Template::render(
        "403",
        context! {
            site_name: "403 Forbidden"
        },
    )
}
