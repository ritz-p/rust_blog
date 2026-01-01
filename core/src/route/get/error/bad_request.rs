use rocket_dyn_templates::{Template, context};

#[catch(400)]
pub fn bad_request() -> Template {
    Template::render(
        "400",
        context! {
            site_name: "400 Bad Request"
        },
    )
}
