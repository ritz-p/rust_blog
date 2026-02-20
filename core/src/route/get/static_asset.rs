use rocket::http::ContentType;

const BULMA_CSS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/assets/bulma.min.css"
));
const NAV_JS: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/nav.js"));

#[get("/css/bulma.min.css")]
pub fn bulma_css() -> (ContentType, &'static str) {
    (ContentType::CSS, BULMA_CSS)
}

#[get("/js/nav.js")]
pub fn nav_js() -> (ContentType, &'static str) {
    (ContentType::JavaScript, NAV_JS)
}
