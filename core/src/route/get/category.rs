use rocket::{State, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::{DatabaseConnection, DbErr};
use serde_json::json;

use crate::{
    domain::{
        page::{Page, PageInfo},
        query::{PagingQuery, category::CategoryQuery},
    },
    repository::{article::get_article_by_category_slug, category::get_all_categories},
    utils::config::CommonConfig,
};

#[get("/categories")]
pub async fn category_list(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
) -> Result<Template, Status> {
    let models = get_all_categories(db)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let categories = models
        .iter()
        .map(|category| {
            json!({
                "name": category.name.clone(),"slug": category.slug.clone()
            })
        })
        .collect::<Vec<_>>();
    Ok(Template::render(
        "categories",
        context! {
            site_name: &config.site_name,
            categories
        },
    ))
}

#[get("/category/<slug>?<query..>")]
pub async fn category_detail(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    query: Option<CategoryQuery>,
    slug: &str,
) -> Result<Template, Status> {
    let query = query.unwrap_or(CategoryQuery::new());
    let page = Page::new_from_query(&query);
    println!("{:?}", page);
    let sort_key = query.sort_key.unwrap_or_else(|| "created_at".to_string());
    match get_article_by_category_slug(db.inner(), page, slug, &sort_key).await {
        Ok((articles, page_info)) => {
            let base_path = "/category/".to_owned() + slug;
            let prev_url = PageInfo::get_prev_url(&page_info, &base_path, Some(&sort_key));
            let next_url = PageInfo::get_next_url(&page_info, &base_path, Some(&sort_key));
            println!(
                "pageinfo={:?} prev={} next={}",
                page_info, prev_url, next_url
            );

            Ok(Template::render(
                "category",
                context! {
                    site_name: &config.site_name,
                    category_slug: slug,
                    sort_key: sort_key,
                    articles: articles.iter().map(|article| {
                        json!({
                            "title": article.title.clone(),
                            "slug": article.slug,
                            "created_at": article.created_at.to_string(),
                        })
                    }).collect::<Vec<_>>(),
                    page: page_info.count,
                    per: page_info.per,
                    total_pages: page_info.total_pages,
                    has_prev: page_info.has_prev,
                    has_next: page_info.has_next,
                    prev_page: page_info.prev_page,
                    next_page: page_info.next_page,
                    prev_url: prev_url,
                    next_url: next_url,
                },
            ))
        }
        Err(DbErr::RecordNotFound(_)) => Err(Status::NotFound),
        Err(e) => {
            error!("category_detail error for {}: {}", slug, e);
            Err(Status::InternalServerError)
        }
    }
}
