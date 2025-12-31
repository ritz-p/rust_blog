use rocket::{State, http::Status};
use rocket_dyn_templates::{Template, context};
use sea_orm::{DatabaseConnection, DbErr};
use serde_json::json;

use crate::{
    domain::{
        page::{Page, PageInfo},
        query::{PagingQuery, tag::TagQuery},
    },
    repository::{article::get_articles_by_tag_slug, tag::get_all_tags},
    utils::{config::CommonConfig, cut_out_string},
};

#[get("/tags")]
pub async fn tag_list(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
) -> Result<Template, Status> {
    let models = get_all_tags(db)
        .await
        .map_err(|_| Status::InternalServerError)?;
    let tags = models
        .iter()
        .map(|tag| {
            json!({
                "name": tag.name.clone(),"slug": tag.slug.clone()
            })
        })
        .collect::<Vec<_>>();
    Ok(Template::render(
        "tags",
        context! {
            site_name: &config.site_name,
            tags
        },
    ))
}

#[get("/tag/<slug>?<query..>")]
pub async fn tag_detail(
    config: &State<CommonConfig>,
    db: &State<DatabaseConnection>,
    slug: &str,
    query: Option<TagQuery>,
) -> Result<Template, Status> {
    let query = query.unwrap_or(TagQuery::new());
    let page = Page::new_from_query(&query);
    let sort_key = query.sort_key.unwrap_or_else(|| "created_at".to_string());
    match get_articles_by_tag_slug(&db, page, slug, &sort_key).await {
        Ok((articles, page_info)) => {
            let base_path = "/tag/".to_owned() + slug;
            let prev_url = PageInfo::get_prev_url(&page_info, &base_path, Some(&sort_key));
            let next_url = PageInfo::get_next_url(&page_info, &base_path, Some(&sort_key));
            let default_icatch_path = config
                .default_icatch_path
                .clone()
                .unwrap_or_default();
            Ok(Template::render(
                "tag",
                context! {
                    site_name: &config.site_name,
                    tag_slug: slug,
                    sort_key: sort_key,
                    articles: articles.iter().map(|article| {
                        let icatch_path = article
                            .icatch_path
                            .clone()
                            .unwrap_or_else(|| default_icatch_path.clone());
                        let excerpt = match article.excerpt.as_ref() {
                            Some(value) => value.clone(),
                            None => cut_out_string(&article.content, 100),
                        };
                        json!({
                            "title": article.title.clone(),
                            "slug": article.slug,
                            "icatch_path": icatch_path,
                            "excerpt": excerpt,
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
            error!("tag_detail error for {}: {}", slug, e);
            Err(Status::InternalServerError)
        }
    }
}
