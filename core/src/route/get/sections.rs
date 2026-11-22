use crate::utils::config::CommonConfig;
use rocket::{State, http::Status, response::Redirect};
use rocket_dyn_templates::Template;
use rust_blog::{domain::page::Page, repository::article::ArticlePeriod, sections::Navigation};
use sea_orm::DatabaseConnection;

pub struct SelectedHome<'r>(&'r Navigation);

#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for SelectedHome<'r> {
    type Error = ();

    async fn from_request(
        request: &'r rocket::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        match request.rocket().state::<Navigation>() {
            Some(navigation) if navigation.home != "all" => {
                rocket::request::Outcome::Success(Self(navigation))
            }
            Some(_) => rocket::request::Outcome::Forward(Status::NotFound),
            None => rocket::request::Outcome::Error((Status::InternalServerError, ())),
        }
    }
}

#[get("/", rank = 1)]
pub fn home(selected: SelectedHome<'_>) -> Option<Redirect> {
    let navigation = selected.0;
    navigation
        .tabs
        .iter()
        .find(|tab| tab.id() == navigation.home)
        .map(|tab| Redirect::to(navigation.url(tab, false)))
}

#[derive(FromForm)]
pub struct ListingQuery {
    page: Option<u64>,
    year: Option<i32>,
    month: Option<u32>,
}

#[get("/all?<query..>")]
pub async fn all(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    navigation: &State<Navigation>,
    query: ListingQuery,
) -> Result<Template, Status> {
    render(config, db, navigation, None, query).await
}

#[get("/sections/<section>?<query..>")]
pub async fn section(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    navigation: &State<Navigation>,
    section: &str,
    query: ListingQuery,
) -> Result<Template, Status> {
    if navigation.validate_section(section).is_err() {
        return Err(Status::NotFound);
    }
    render(config, db, navigation, Some(section), query).await
}

async fn render(
    config: &CommonConfig,
    db: &DatabaseConnection,
    navigation: &Navigation,
    section: Option<&str>,
    query: ListingQuery,
) -> Result<Template, Status> {
    let period = match (query.year, query.month) {
        (None, None) => None,
        (Some(year), Some(month)) => Some(ArticlePeriod::new(year, month).ok_or(Status::NotFound)?),
        _ => return Err(Status::NotFound),
    };
    let mut context = rust_blog::sections::context(
        db,
        navigation,
        section,
        Page {
            number: query.page.unwrap_or(1),
            per: config.articles_per_page,
        },
        period,
        false,
        config,
    )
    .await
    .map_err(|error| {
        error!("section: {error:#}");
        Status::InternalServerError
    })?;
    if section == Some(navigation.home.as_str()) && period.is_none() && query.page.unwrap_or(1) <= 1
    {
        context.insert("metadata", &crate::utils::metadata::site_metadata(config));
    }
    context.insert("site_name", &config.site_name);
    context.insert("favicon_path", &config.favicon_path);
    Ok(Template::render("sections", context.into_json()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rocket::local::asynchronous::Client;
    use rust_blog::sections::Tab;

    #[get("/?<q>", rank = 2)]
    fn fallback(q: Option<&str>) -> String {
        format!("all articles: {}", q.unwrap_or_default())
    }

    #[rocket::async_test]
    async fn default_home_forwards_to_article_index_including_search() {
        let client = Client::tracked(
            rocket::build()
                .manage(Navigation::default())
                .mount("/", routes![home, fallback]),
        )
        .await
        .unwrap();
        for path in ["/", "/?q=Rust"] {
            let response = client.get(path).dispatch().await;
            assert_eq!(response.status(), Status::Ok);
            assert!(
                response
                    .into_string()
                    .await
                    .unwrap()
                    .starts_with("all articles:")
            );
        }
    }

    #[rocket::async_test]
    async fn home_redirects_to_selected_section_or_fixed_page() {
        for (selected, location) in [("blog", "/sections/blog"), ("profile", "/about")] {
            let mut navigation = Navigation::default();
            navigation.tabs.push(Tab::Page {
                id: "profile".into(),
                label: "About".into(),
                slug: "about".into(),
            });
            navigation.home = selected.into();
            let client =
                Client::tracked(rocket::build().manage(navigation).mount("/", routes![home]))
                    .await
                    .unwrap();
            let response = client.get("/").dispatch().await;
            assert_eq!(response.status(), Status::SeeOther);
            assert_eq!(response.headers().get_one("Location"), Some(location));
        }
    }
}
