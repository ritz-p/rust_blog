use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context as AnyhowContext, Result};
use chrono::{Datelike, Utc};
use sea_orm::DatabaseConnection;
use serde_json::json;
use tera::{Context, Tera};
use walkdir::WalkDir;

use crate::{
    domain::page::Page,
    repository::{
        article::{
            ArticlePeriod, get_all_articles, get_all_published_articles, get_article_periods,
            get_articles_by_tag_slug, get_article_by_category_slug, get_latest_articles,
        },
        category::{get_all_categories, get_categories_by_article},
        fixed_content::get_all_fixed_contents,
        tag::{get_all_tags, get_tags_by_article},
    },
    utils::{
        config::CommonConfig, cut_out_string,
        markdown::{markdown_to_html, markdown_to_text},
        utc_to_jst,
    },
};

const PAGE_SIZE: u64 = 10;
const BULMA_CSS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/bulma.min.css"));
const SITE_CSS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/site.css"));
const NAV_JS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/nav.js"));

pub async fn export_site(
    db: &DatabaseConnection,
    config_map: &HashMap<String, String>,
    out_dir: impl AsRef<Path>,
) -> Result<()> {
    let out_dir = out_dir.as_ref();
    reset_output_dir(out_dir)?;
    write_static_assets(out_dir)?;

    let config = CommonConfig {
        site_name: config_map.get("site_name").cloned(),
        default_icatch_path: config_map.get("default_icatch_path").cloned(),
        favicon_path: config_map.get("favicon_path").cloned(),
    };
    let tera = load_templates(Path::new("templates"))?;

    export_index_pages(&tera, db, &config, out_dir).await?;
    export_article_pages(&tera, db, &config, out_dir).await?;
    export_fixed_content_pages(&tera, db, &config, out_dir).await?;
    export_tag_pages(&tera, db, &config, out_dir).await?;
    export_category_pages(&tera, db, &config, out_dir).await?;
    export_error_page(&tera, &config, out_dir, "404", "404.html")?;
    write_cloudflare_support_files(out_dir)?;

    Ok(())
}

async fn export_index_pages(
    tera: &Tera,
    db: &DatabaseConnection,
    config: &CommonConfig,
    out_dir: &Path,
) -> Result<()> {
    let periods = get_article_periods(db, None).await?;
    export_index_variant(tera, db, config, out_dir, None, &periods).await?;
    for period in &periods {
        export_index_variant(tera, db, config, out_dir, Some(*period), &periods).await?;
    }
    Ok(())
}

async fn export_index_variant(
    tera: &Tera,
    db: &DatabaseConnection,
    config: &CommonConfig,
    out_dir: &Path,
    period: Option<ArticlePeriod>,
    all_periods: &[ArticlePeriod],
) -> Result<()> {
    let (_, first_page_info) = get_all_articles(
        db,
        Page {
            number: 1,
            per: PAGE_SIZE,
        },
        period,
    )
    .await?;

    for page_number in 1..=first_page_info.total_pages {
        let page = Page {
            number: page_number,
            per: PAGE_SIZE,
        };
        let (models, page_info) = get_all_articles(db, page, period).await?;
        let default_icatch_path = config.default_icatch_path.clone().unwrap_or_default();
        let period_links: Vec<_> = all_periods
            .iter()
            .map(|p| {
                json!({
                    "label": format!("{}/{:02}", p.year, p.month),
                    "href": static_index_url(1, Some(*p)),
                    "is_selected": period == Some(*p),
                })
            })
            .collect();
        let articles: Vec<_> = models
            .into_iter()
            .map(|m| {
                let excerpt = match m.excerpt.as_ref() {
                    Some(value) => value.clone(),
                    None => cut_out_string(&markdown_to_text(&m.content), 100),
                };
                let icatch_path = m
                    .icatch_path
                    .clone()
                    .unwrap_or_else(|| default_icatch_path.clone());
                json!({
                    "title": m.title,
                    "slug": m.slug,
                    "excerpt": excerpt,
                    "icatch_path": icatch_path,
                    "created_at": utc_to_jst(m.created_at),
                    "updated_at": utc_to_jst(m.updated_at),
                })
            })
            .collect();

        let mut ctx = base_context(config);
        ctx.insert("articles", &articles);
        ctx.insert("page", &page_info.current_page);
        ctx.insert("per", &page_info.per);
        ctx.insert("total_pages", &page_info.total_pages);
        ctx.insert("has_prev", &page_info.has_prev);
        ctx.insert("has_next", &page_info.has_next);
        ctx.insert("prev_page", &page_info.prev_page);
        ctx.insert("next_page", &page_info.next_page);
        ctx.insert("prev_url", &static_index_url(page_info.prev_page, period));
        ctx.insert("next_url", &static_index_url(page_info.next_page, period));
        ctx.insert(
            "selected_period",
            &period.map(|p| format!("{}/{:02}", p.year, p.month)),
        );
        ctx.insert("period_links", &period_links);

        render_to_path(
            tera,
            "index",
            &ctx,
            &out_dir.join(static_index_output_path(page_info.current_page, period)),
        )?;
    }

    Ok(())
}

async fn export_article_pages(
    tera: &Tera,
    db: &DatabaseConnection,
    config: &CommonConfig,
    out_dir: &Path,
) -> Result<()> {
    let latest_articles = latest_articles_json(db).await?;
    let articles = get_all_published_articles(db).await?;
    for article in articles {
        let tags: Vec<_> = get_tags_by_article(db, &article)
            .await?
            .into_iter()
            .map(|tag| json!({ "name": tag.name, "slug": tag.slug }))
            .collect();
        let categories: Vec<_> = get_categories_by_article(db, &article)
            .await?
            .into_iter()
            .map(|category| json!({ "name": category.name, "slug": category.slug }))
            .collect();

        let mut ctx = base_context(config);
        ctx.insert("title", &article.title);
        ctx.insert("content_html", &markdown_to_html(&article.content));
        ctx.insert("created_at", &utc_to_jst(article.created_at));
        ctx.insert("updated_at", &utc_to_jst(article.updated_at));
        ctx.insert("tags", &tags);
        ctx.insert("categories", &categories);
        ctx.insert("latest_articles", &latest_articles);

        render_to_path(
            tera,
            "article_detail",
            &ctx,
            &out_dir.join(format!("posts/{}/index.html", article.slug)),
        )?;
    }
    Ok(())
}

async fn export_fixed_content_pages(
    tera: &Tera,
    db: &DatabaseConnection,
    config: &CommonConfig,
    out_dir: &Path,
) -> Result<()> {
    let latest_articles = latest_articles_json(db).await?;
    let fixed_contents = get_all_fixed_contents(db).await?;
    for fixed_content in fixed_contents {
        let excerpt = match fixed_content.excerpt.as_ref() {
            Some(value) => value.clone(),
            None => cut_out_string(&markdown_to_text(&fixed_content.content), 100),
        };

        let mut ctx = base_context(config);
        ctx.insert("title", &fixed_content.title);
        ctx.insert("excerpt", &excerpt);
        ctx.insert("content_html", &markdown_to_html(&fixed_content.content));
        ctx.insert("created_at", &utc_to_jst(fixed_content.created_at));
        ctx.insert("updated_at", &utc_to_jst(fixed_content.updated_at));
        ctx.insert("latest_articles", &latest_articles);

        render_to_path(
            tera,
            "about",
            &ctx,
            &out_dir.join(format!("{}/index.html", fixed_content.slug)),
        )?;
    }
    Ok(())
}

async fn export_tag_pages(
    tera: &Tera,
    db: &DatabaseConnection,
    config: &CommonConfig,
    out_dir: &Path,
) -> Result<()> {
    let tags = get_all_tags(db).await?;
    let tag_items: Vec<_> = tags
        .iter()
        .map(|tag| json!({ "name": tag.name, "slug": tag.slug }))
        .collect();

    let mut list_ctx = base_context(config);
    list_ctx.insert("tags", &tag_items);
    render_to_path(tera, "tags", &list_ctx, &out_dir.join("tags/index.html"))?;

    for tag in tags {
        for sort_key in ["created_at", "updated_at"] {
            export_tag_variant(tera, db, config, out_dir, &tag.slug, sort_key).await?;
        }
    }

    Ok(())
}

async fn export_tag_variant(
    tera: &Tera,
    db: &DatabaseConnection,
    config: &CommonConfig,
    out_dir: &Path,
    slug: &str,
    sort_key: &str,
) -> Result<()> {
    let (_, first_page_info) = get_articles_by_tag_slug(
        db,
        Page {
            number: 1,
            per: PAGE_SIZE,
        },
        slug,
        sort_key,
    )
    .await?;

    for page_number in 1..=first_page_info.total_pages {
        let (articles, page_info) = get_articles_by_tag_slug(
            db,
            Page {
                number: page_number,
                per: PAGE_SIZE,
            },
            slug,
            sort_key,
        )
        .await?;
        let default_icatch_path = config.default_icatch_path.clone().unwrap_or_default();

        let article_items: Vec<_> = articles
            .iter()
            .map(|article| {
                let icatch_path = article
                    .icatch_path
                    .clone()
                    .unwrap_or_else(|| default_icatch_path.clone());
                let excerpt = match article.excerpt.as_ref() {
                    Some(value) => value.clone(),
                    None => cut_out_string(&markdown_to_text(&article.content), 100),
                };
                json!({
                    "title": article.title.clone(),
                    "slug": article.slug,
                    "icatch_path": icatch_path,
                    "excerpt": excerpt,
                    "created_at": utc_to_jst(article.created_at),
                })
            })
            .collect();

        let mut ctx = base_context(config);
        ctx.insert("tag_slug", &slug);
        ctx.insert("sort_key", &sort_key);
        ctx.insert("sort_created_url", &static_tag_url(slug, "created_at", 1));
        ctx.insert("sort_updated_url", &static_tag_url(slug, "updated_at", 1));
        ctx.insert("articles", &article_items);
        ctx.insert("page", &page_info.current_page);
        ctx.insert("per", &page_info.per);
        ctx.insert("total_pages", &page_info.total_pages);
        ctx.insert("has_prev", &page_info.has_prev);
        ctx.insert("has_next", &page_info.has_next);
        ctx.insert("prev_page", &page_info.prev_page);
        ctx.insert("next_page", &page_info.next_page);
        ctx.insert(
            "prev_url",
            &static_tag_url(slug, sort_key, page_info.prev_page),
        );
        ctx.insert(
            "next_url",
            &static_tag_url(slug, sort_key, page_info.next_page),
        );

        render_to_path(
            tera,
            "tag",
            &ctx,
            &out_dir.join(static_tag_output_path(slug, sort_key, page_info.current_page)),
        )?;
    }
    Ok(())
}

async fn export_category_pages(
    tera: &Tera,
    db: &DatabaseConnection,
    config: &CommonConfig,
    out_dir: &Path,
) -> Result<()> {
    let categories = get_all_categories(db).await?;
    let category_items: Vec<_> = categories
        .iter()
        .map(|category| json!({ "name": category.name, "slug": category.slug }))
        .collect();

    let mut list_ctx = base_context(config);
    list_ctx.insert("categories", &category_items);
    render_to_path(
        tera,
        "categories",
        &list_ctx,
        &out_dir.join("categories/index.html"),
    )?;

    for category in categories {
        for sort_key in ["created_at", "updated_at"] {
            export_category_variant(tera, db, config, out_dir, &category.slug, sort_key).await?;
        }
    }

    Ok(())
}

async fn export_category_variant(
    tera: &Tera,
    db: &DatabaseConnection,
    config: &CommonConfig,
    out_dir: &Path,
    slug: &str,
    sort_key: &str,
) -> Result<()> {
    let (_, first_page_info) = get_article_by_category_slug(
        db,
        Page {
            number: 1,
            per: PAGE_SIZE,
        },
        slug,
        sort_key,
    )
    .await?;

    for page_number in 1..=first_page_info.total_pages {
        let (articles, page_info) = get_article_by_category_slug(
            db,
            Page {
                number: page_number,
                per: PAGE_SIZE,
            },
            slug,
            sort_key,
        )
        .await?;
        let default_icatch_path = config.default_icatch_path.clone().unwrap_or_default();

        let article_items: Vec<_> = articles
            .iter()
            .map(|article| {
                let icatch_path = article
                    .icatch_path
                    .clone()
                    .unwrap_or_else(|| default_icatch_path.clone());
                let excerpt = match article.excerpt.as_ref() {
                    Some(value) => value.clone(),
                    None => cut_out_string(&markdown_to_text(&article.content), 100),
                };
                json!({
                    "title": article.title.clone(),
                    "slug": article.slug,
                    "icatch_path": icatch_path,
                    "excerpt": excerpt,
                    "created_at": utc_to_jst(article.created_at),
                })
            })
            .collect();

        let mut ctx = base_context(config);
        ctx.insert("category_slug", &slug);
        ctx.insert("sort_key", &sort_key);
        ctx.insert("sort_created_url", &static_category_url(slug, "created_at", 1));
        ctx.insert("sort_updated_url", &static_category_url(slug, "updated_at", 1));
        ctx.insert("articles", &article_items);
        ctx.insert("page", &page_info.current_page);
        ctx.insert("per", &page_info.per);
        ctx.insert("total_pages", &page_info.total_pages);
        ctx.insert("has_prev", &page_info.has_prev);
        ctx.insert("has_next", &page_info.has_next);
        ctx.insert("prev_page", &page_info.prev_page);
        ctx.insert("next_page", &page_info.next_page);
        ctx.insert(
            "prev_url",
            &static_category_url(slug, sort_key, page_info.prev_page),
        );
        ctx.insert(
            "next_url",
            &static_category_url(slug, sort_key, page_info.next_page),
        );

        render_to_path(
            tera,
            "category",
            &ctx,
            &out_dir.join(static_category_output_path(slug, sort_key, page_info.current_page)),
        )?;
    }
    Ok(())
}

fn export_error_page(
    tera: &Tera,
    config: &CommonConfig,
    out_dir: &Path,
    template_name: &str,
    output_name: &str,
) -> Result<()> {
    let ctx = base_context(config);
    render_to_path(tera, template_name, &ctx, &out_dir.join(output_name))
}

async fn latest_articles_json(db: &DatabaseConnection) -> Result<Vec<serde_json::Value>> {
    Ok(get_latest_articles(db, 5)
        .await?
        .into_iter()
        .map(|model| {
            json!({
                "title": model.title,
                "slug": model.slug,
            })
        })
        .collect())
}

fn base_context(config: &CommonConfig) -> Context {
    let mut ctx = Context::new();
    ctx.insert("site_name", &config.site_name);
    ctx.insert("favicon_path", &config.favicon_path);
    ctx.insert("year", &Utc::now().year());
    ctx
}

fn render_to_path(tera: &Tera, template: &str, ctx: &Context, output: &Path) -> Result<()> {
    let rendered = tera
        .render(template, ctx)
        .with_context(|| format!("failed to render template {template}"))?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, rendered).with_context(|| format!("failed to write {:?}", output))?;
    Ok(())
}

fn load_templates(root: &Path) -> Result<Tera> {
    let mut templates = Vec::new();
    for entry in WalkDir::new(root) {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let relative = path
            .strip_prefix(root)
            .with_context(|| format!("failed to strip prefix for {:?}", path))?;
        let name = normalize_template_name(relative);
        let contents =
            fs::read_to_string(path).with_context(|| format!("failed to read {:?}", path))?;
        templates.push((name, contents));
    }
    let mut tera = Tera::default();
    tera.add_raw_templates(templates)
        .context("failed to register templates")?;
    Ok(tera)
}

fn normalize_template_name(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    raw.strip_suffix(".html.tera")
        .or_else(|| raw.strip_suffix(".tera"))
        .unwrap_or(&raw)
        .to_string()
}

fn reset_output_dir(out_dir: &Path) -> Result<()> {
    if out_dir.exists() {
        fs::remove_dir_all(out_dir)
            .with_context(|| format!("failed to clear output dir {:?}", out_dir))?;
    }
    fs::create_dir_all(out_dir)?;
    Ok(())
}

fn write_static_assets(out_dir: &Path) -> Result<()> {
    write_embedded_asset_file(out_dir.join("css/bulma.min.css"), BULMA_CSS)?;
    write_embedded_asset_file(out_dir.join("css/site.css"), SITE_CSS)?;
    write_embedded_asset_file(out_dir.join("js/nav.js"), NAV_JS)?;
    copy_dir_recursive(Path::new("content/image"), &out_dir.join("image"))?;
    copy_dir_recursive(Path::new("content/icon"), &out_dir.join("icon"))?;
    Ok(())
}

fn write_cloudflare_support_files(out_dir: &Path) -> Result<()> {
    fs::write(out_dir.join("_headers"), build_headers_file())
        .with_context(|| format!("failed to write {:?}", out_dir.join("_headers")))?;
    fs::write(out_dir.join("_redirects"), build_redirects_file(out_dir)?)
        .with_context(|| format!("failed to write {:?}", out_dir.join("_redirects")))?;
    Ok(())
}

fn build_headers_file() -> String {
    [
        "/*",
        "  Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' https: data:; object-src 'none'; base-uri 'self'; frame-ancestors 'none'; form-action 'self'",
        "  Referrer-Policy: strict-origin-when-cross-origin",
        "  X-Content-Type-Options: nosniff",
        "  X-Frame-Options: DENY",
        "  Permissions-Policy: geolocation=(), microphone=(), camera=()",
        "  Strict-Transport-Security: max-age=31536000; includeSubDomains",
        "",
        "/css/*",
        "  Cache-Control: public, max-age=31556952, immutable",
        "",
        "/js/*",
        "  Cache-Control: public, max-age=31556952, immutable",
        "",
        "/image/*",
        "  Cache-Control: public, max-age=31556952, immutable",
        "",
        "/icon/*",
        "  Cache-Control: public, max-age=31556952, immutable",
        "",
    ]
    .join("\n")
}

fn build_redirects_file(out_dir: &Path) -> Result<String> {
    let mut redirects = Vec::new();
    for entry in WalkDir::new(out_dir).min_depth(1) {
        let entry = entry?;
        if !entry.file_type().is_dir() {
            continue;
        }
        let index_html = entry.path().join("index.html");
        if !index_html.exists() {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(out_dir)
            .with_context(|| format!("failed to strip prefix for {:?}", entry.path()))?;
        let rel = relative.to_string_lossy().replace('\\', "/");
        if rel.is_empty() {
            continue;
        }
        redirects.push(format!("/{rel} /{rel}/ 308"));
    }
    redirects.sort();
    redirects.dedup();
    redirects.push(String::new());
    Ok(redirects.join("\n"))
}

fn write_embedded_asset_file(target: PathBuf, contents: &[u8]) -> Result<()> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&target, contents).with_context(|| format!("failed to write {:?}", target))?;
    Ok(())
}

fn copy_dir_recursive(source: &Path, target: &Path) -> Result<()> {
    if !source.exists() {
        return Ok(());
    }
    for entry in WalkDir::new(source) {
        let entry = entry?;
        let relative = entry.path().strip_prefix(source)?;
        let destination = target.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&destination)?;
        } else {
            if let Some(parent) = destination.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(entry.path(), &destination).with_context(|| {
                format!(
                    "failed to copy static asset from {:?} to {:?}",
                    entry.path(),
                    destination
                )
            })?;
        }
    }
    Ok(())
}

fn static_index_output_path(page: u64, period: Option<ArticlePeriod>) -> PathBuf {
    match period {
        None if page <= 1 => PathBuf::from("index.html"),
        None => PathBuf::from(format!("page/{page}/index.html")),
        Some(period) if page <= 1 => {
            PathBuf::from(format!("archive/{}/{:02}/index.html", period.year, period.month))
        }
        Some(period) => PathBuf::from(format!(
            "archive/{}/{:02}/page/{page}/index.html",
            period.year, period.month
        )),
    }
}

fn static_index_url(page: u64, period: Option<ArticlePeriod>) -> String {
    match period {
        None if page <= 1 => "/".to_string(),
        None => format!("/page/{page}/"),
        Some(period) if page <= 1 => format!("/archive/{}/{:02}/", period.year, period.month),
        Some(period) => format!("/archive/{}/{:02}/page/{page}/", period.year, period.month),
    }
}

fn static_tag_output_path(slug: &str, sort_key: &str, page: u64) -> PathBuf {
    let sort_segment = if sort_key == "updated_at" {
        "updated/"
    } else {
        ""
    };
    if page <= 1 {
        PathBuf::from(format!("tag/{slug}/{sort_segment}index.html"))
    } else {
        PathBuf::from(format!("tag/{slug}/{sort_segment}page/{page}/index.html"))
    }
}

fn static_tag_url(slug: &str, sort_key: &str, page: u64) -> String {
    let sort_segment = if sort_key == "updated_at" {
        "updated/"
    } else {
        ""
    };
    if page <= 1 {
        format!("/tag/{slug}/{sort_segment}")
    } else {
        format!("/tag/{slug}/{sort_segment}page/{page}/")
    }
}

fn static_category_output_path(slug: &str, sort_key: &str, page: u64) -> PathBuf {
    let sort_segment = if sort_key == "updated_at" {
        "updated/"
    } else {
        ""
    };
    if page <= 1 {
        PathBuf::from(format!("category/{slug}/{sort_segment}index.html"))
    } else {
        PathBuf::from(format!(
            "category/{slug}/{sort_segment}page/{page}/index.html"
        ))
    }
}

fn static_category_url(slug: &str, sort_key: &str, page: u64) -> String {
    let sort_segment = if sort_key == "updated_at" {
        "updated/"
    } else {
        ""
    };
    if page <= 1 {
        format!("/category/{slug}/{sort_segment}")
    } else {
        format!("/category/{slug}/{sort_segment}page/{page}/")
    }
}
