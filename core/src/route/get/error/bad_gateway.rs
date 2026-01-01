use rocket_dyn_templates::{Template, context};

#[catch(502)]
pub fn bad_gateway() -> Template {
    Template::render(
        "502",
        context! {
            site_name: "502 Bad Gateway"
        },
    )
}
