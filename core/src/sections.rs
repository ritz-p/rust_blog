use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Tab {
    All {
        id: String,
        label: String,
    },
    Section {
        id: String,
        label: String,
    },
    Page {
        id: String,
        label: String,
        slug: String,
    },
}

impl Tab {
    pub fn id(&self) -> &str {
        match self {
            Self::All { id, .. } | Self::Section { id, .. } | Self::Page { id, .. } => id,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::All { label, .. } | Self::Section { label, .. } | Self::Page { label, .. } => {
                label
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Navigation {
    #[serde(default = "default_home")]
    pub home: String,
    pub default_section: String,
    pub tabs: Vec<Tab>,
}

fn default_home() -> String {
    "all".into()
}

impl Default for Navigation {
    fn default() -> Self {
        Self {
            home: default_home(),
            default_section: "blog".into(),
            tabs: vec![
                Tab::All {
                    id: "all".into(),
                    label: "すべて".into(),
                },
                Tab::Section {
                    id: "blog".into(),
                    label: "ブログ".into(),
                },
            ],
        }
    }
}

impl Navigation {
    pub fn from_config(map: &std::collections::HashMap<String, String>) -> Result<Self> {
        let navigation = match map.get("navigation") {
            Some(value) => toml::from_str(value)?,
            None => Self::default(),
        };
        navigation.validate()?;
        Ok(navigation)
    }

    pub fn validate(&self) -> Result<()> {
        ensure!(
            (2..=10).contains(&self.tabs.len()),
            "navigation requires 2 to 10 tabs"
        );
        let mut ids = HashSet::new();
        let mut all_count = 0;
        for tab in &self.tabs {
            crate::slug::validate(tab.id(), 100)?;
            ensure!(
                tab.id().bytes().all(|byte| byte.is_ascii_lowercase()
                    || byte.is_ascii_digit()
                    || b"-_".contains(&byte)),
                "tab id must use lowercase ASCII letters, digits, - or _"
            );
            ensure!(
                ids.insert(crate::slug::collision_key(tab.id())),
                "duplicate tab id: {}",
                tab.id()
            );
            ensure!(
                !tab.label().trim().is_empty(),
                "tab label must not be blank"
            );
            match tab {
                Tab::All { id, .. } => {
                    ensure!(id == "all", "the all-articles tab id must be all");
                    all_count += 1;
                }
                Tab::Page { slug, .. } => crate::slug::validate_fixed(slug)?,
                Tab::Section { .. } => {}
            }
        }
        ensure!(all_count == 1, "exactly one all-articles tab is required");
        ensure!(
            self.tabs.iter().any(|tab| tab.id() == self.home),
            "unknown home tab: {}",
            self.home
        );
        self.validate_section(&self.default_section)
    }

    pub fn validate_section(&self, section: &str) -> Result<()> {
        ensure!(
            self.tabs
                .iter()
                .any(|tab| matches!(tab, Tab::Section { id, .. } if id == section)),
            "unknown article section: {section}"
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_pages_and_all_are_not_article_sections() {
        let mut navigation = Navigation::default();
        navigation.tabs.push(Tab::Page {
            id: "about".into(),
            label: "About".into(),
            slug: "about".into(),
        });
        navigation.home = "about".into();
        navigation.validate().unwrap();
        assert!(navigation.validate_section("all").is_err());
        assert!(navigation.validate_section("about").is_err());
        assert!(navigation.validate_section("blog").is_ok());
    }

    #[test]
    fn rejects_ambiguous_tabs_and_invalid_defaults() {
        let mut navigation = Navigation::default();
        navigation.tabs.push(Tab::Section {
            id: "BLOG".into(),
            label: "Duplicate".into(),
        });
        assert!(navigation.validate().is_err());
        navigation.tabs.pop();
        navigation.home = "missing".into();
        assert!(navigation.validate().is_err());
        navigation.home = "all".into();
        navigation.default_section = "all".into();
        assert!(navigation.validate().is_err());
    }

    #[test]
    fn reads_toml_and_defaults_home_to_all() {
        let mut map = std::collections::HashMap::new();
        map.insert(
            "navigation".into(),
            r#"
default_section = "tech"
[[tabs]]
kind = "all"
id = "all"
label = "すべて"
[[tabs]]
kind = "section"
id = "tech"
label = "技術ブログ"
[[tabs]]
kind = "page"
id = "about"
label = "About"
slug = "about"
"#
            .into(),
        );
        let navigation = Navigation::from_config(&map).unwrap();
        assert_eq!(navigation.home, "all");
        assert_eq!(navigation.default_section, "tech");
        assert_eq!(navigation.tabs.len(), 3);
        map.insert("navigation".into(), "invalid".into());
        assert!(Navigation::from_config(&map).is_err());
    }
}

pub fn taxonomy_name(section: &str, name: &str) -> String {
    if section == "blog" {
        name.to_owned()
    } else {
        format!("[{section}] {name}")
    }
}

pub fn taxonomy_label<'a>(section: &str, name: &'a str) -> &'a str {
    if section == "blog" {
        name
    } else {
        name.strip_prefix(&format!("[{section}] ")).unwrap_or(name)
    }
}

impl Navigation {
    pub fn load() -> Result<Self> {
        let path =
            std::env::var("RUST_BLOG_CONFIG_PATH").unwrap_or_else(|_| "blog_config.toml".into());
        #[derive(Deserialize)]
        struct Config {
            #[serde(default)]
            navigation: Navigation,
        }
        let contents = match std::fs::read_to_string(path) {
            Ok(contents) => contents,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && std::env::var_os("RUST_BLOG_CONFIG_PATH").is_none() =>
            {
                return Ok(Self::default());
            }
            Err(error) => return Err(error.into()),
        };
        let config: Config = toml::from_str(&contents)?;
        config.navigation.validate()?;
        Ok(config.navigation)
    }

    pub fn links(&self, static_mode: bool) -> serde_json::Value {
        serde_json::json!(
            self.tabs
                .iter()
                .map(|tab| serde_json::json!({
                    "id": tab.id(), "label": tab.label(), "url": self.url(tab, static_mode)
                }))
                .collect::<Vec<_>>()
        )
    }

    pub fn url(&self, tab: &Tab, static_mode: bool) -> String {
        let suffix = if static_mode { "/" } else { "" };
        match tab {
            Tab::All { .. } => format!("/all{suffix}"),
            Tab::Section { id, .. } => {
                format!("/sections/{}{suffix}", crate::utils::url_segment(id))
            }
            Tab::Page { slug, .. } => format!("/{}{suffix}", crate::utils::url_segment(slug)),
        }
    }
}

pub async fn context(
    db: &sea_orm::DatabaseConnection,
    navigation: &Navigation,
    section: Option<&str>,
    number: u64,
    per: u64,
    static_mode: bool,
) -> Result<tera::Context> {
    use crate::entity::{article, category, tag};
    use sea_orm::{ColumnTrait, EntityTrait, ModelTrait, QueryFilter, QueryOrder};
    use std::collections::BTreeMap;
    if let Some(section) = section {
        navigation.validate_section(section)?;
    }
    let mut query =
        article::Entity::find().filter(article::Column::CreatedAt.lte(chrono::Utc::now()));
    if let Some(section) = section {
        query = query.filter(article::Column::Section.eq(section));
    }
    let models = query
        .order_by_desc(article::Column::CreatedAt)
        .order_by_desc(article::Column::Id)
        .all(db)
        .await?;
    let page = crate::domain::page::PageInfo::new(
        crate::domain::page::Page {
            number,
            per: per.max(1),
        },
        models.len() as u64,
    );
    let mut tags = BTreeMap::new();
    let mut categories = BTreeMap::new();
    let mut articles = Vec::new();
    let suffix = if static_mode { "/" } else { "" };
    for (index, model) in models.iter().enumerate() {
        for value in model.find_related(tag::Entity).all(db).await? {
            tags.insert(value.slug.clone(), serde_json::json!({"name": value.name, "url": format!("/tag/{}{suffix}", crate::utils::url_segment(&value.slug))}));
        }
        for value in model.find_related(category::Entity).all(db).await? {
            categories.insert(value.slug.clone(), serde_json::json!({"name": value.name, "url": format!("/category/{}{suffix}", crate::utils::url_segment(&value.slug))}));
        }
        if index as u64 >= (page.current_page - 1) * page.per
            && (index as u64) < page.current_page * page.per
        {
            articles.push(serde_json::json!({
                "title": model.title,
                "url": format!("/posts/{}{suffix}", crate::utils::url_segment(&model.slug)),
                "excerpt": crate::utils::cut_out_string(&crate::utils::markdown::markdown_to_text(model.excerpt.as_deref().unwrap_or(&model.content)), 100),
                "icatch_path": model.icatch_path,
                "created_at": crate::utils::utc_to_jst(model.created_at),
                "updated_at": crate::utils::utc_to_jst(model.updated_at),
            }));
        }
    }
    let tab = navigation
        .tabs
        .iter()
        .find(|tab| match (section, tab) {
            (None, Tab::All { .. }) => true,
            (Some(section), Tab::Section { id, .. }) => section == id,
            _ => false,
        })
        .ok_or_else(|| anyhow::anyhow!("missing tab"))?;
    let base = navigation.url(tab, static_mode);
    let url = |number| {
        if static_mode {
            if number == 1 {
                base.clone()
            } else {
                format!("{base}page/{number}/")
            }
        } else {
            format!("{base}?page={number}")
        }
    };
    let mut ctx = tera::Context::new();
    ctx.insert("tab_title", tab.label());
    ctx.insert("active_tab", tab.id());
    ctx.insert("tags_url", &format!("{base}#section-tags"));
    ctx.insert("categories_url", &format!("{base}#section-categories"));
    ctx.insert("navigation_tabs", &navigation.links(static_mode));
    ctx.insert("articles", &articles);
    ctx.insert("section_tags", &tags.values().collect::<Vec<_>>());
    ctx.insert(
        "section_categories",
        &categories.values().collect::<Vec<_>>(),
    );
    ctx.insert("pagination", &page.navigation(url));
    ctx.insert("page", &page.current_page);
    ctx.insert("total_pages", &page.total_pages);
    ctx.insert("has_prev", &page.has_prev);
    ctx.insert("has_next", &page.has_next);
    ctx.insert("prev_url", &url(page.prev_page));
    ctx.insert("next_url", &url(page.next_page));
    Ok(ctx)
}

pub fn install_navigation(
    tera: &mut tera::Tera,
    navigation: &Navigation,
    static_mode: bool,
) -> Result<()> {
    let tabs = navigation.links(static_mode);
    tera.register_function(
        "site_tabs",
        move |_: &std::collections::HashMap<String, serde_json::Value>| Ok(tabs.clone()),
    );
    tera.add_raw_template("partial/navigation", "{% set tabs = site_tabs() %}<nav class=\"tabs\" aria-label=\"記事区分\"><ul>{% for tab in tabs %}<li{% if active_tab is defined and active_tab == tab.id %} class=\"is-active\"{% endif %}><a href=\"{{ tab.url | escape | safe }}\"{% if active_tab is defined and active_tab == tab.id %} aria-current=\"page\"{% endif %}>{{ tab.label | escape | safe }}</a></li>{% endfor %}</ul></nav>")?;
    Ok(())
}

pub async fn export(
    tera: &tera::Tera,
    db: &sea_orm::DatabaseConnection,
    navigation: &Navigation,
    config: &crate::utils::config::CommonConfig,
    out: &std::path::Path,
) -> Result<()> {
    for tab in &navigation.tabs {
        let section = match tab {
            Tab::Section { id, .. } => Some(id.as_str()),
            Tab::All { .. } => None,
            Tab::Page { .. } => continue,
        };
        let base = navigation.url(tab, true);
        let mut number = 1;
        loop {
            let mut context = context(
                db,
                navigation,
                section,
                number,
                config.articles_per_page,
                true,
            )
            .await?;
            context.insert("site_name", &config.site_name);
            context.insert("favicon_path", &config.favicon_path);
            let directory = if number == 1 {
                out.join(base.trim_matches('/'))
            } else {
                out.join(base.trim_matches('/'))
                    .join(format!("page/{number}"))
            };
            std::fs::create_dir_all(&directory)?;
            std::fs::write(
                directory.join("index.html"),
                tera.render("sections", &context)?,
            )?;
            if number
                >= context
                    .get("total_pages")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(1)
            {
                break;
            }
            number += 1;
        }
    }
    if navigation.home != "all" {
        let tab = navigation
            .tabs
            .iter()
            .find(|tab| tab.id() == navigation.home)
            .ok_or_else(|| anyhow::anyhow!("unknown home"))?;
        let source = out
            .join(navigation.url(tab, true).trim_matches('/'))
            .join("index.html");
        std::fs::copy(source, out.join("index.html"))?;
    }
    Ok(())
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};

    #[rocket::async_test]
    async fn sections_keep_same_named_taxonomy_separate_and_exclude_fixed_pages() {
        let db = Database::connect("sqlite::memory:").await.unwrap();
        for sql in [
            "CREATE TABLE article (id INTEGER PRIMARY KEY, title TEXT NOT NULL, slug TEXT UNIQUE NOT NULL, excerpt TEXT, content TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, icatch_path TEXT, table_of_contents BOOLEAN NOT NULL DEFAULT false, deprecated BOOLEAN NOT NULL DEFAULT false, section TEXT NOT NULL DEFAULT 'blog')",
            "CREATE TABLE tag (id INTEGER PRIMARY KEY, name TEXT UNIQUE, slug TEXT UNIQUE)",
            "CREATE TABLE category (id INTEGER PRIMARY KEY, name TEXT UNIQUE, slug TEXT UNIQUE)",
            "CREATE TABLE article_tag (article_id INTEGER, tag_id INTEGER, PRIMARY KEY(article_id, tag_id))",
            "CREATE TABLE article_category (article_id INTEGER, category_id INTEGER, PRIMARY KEY(article_id, category_id))",
            "CREATE TABLE fixed_content (id INTEGER PRIMARY KEY, title TEXT, slug TEXT, content TEXT)",
            "INSERT INTO fixed_content VALUES (1, 'About', 'about', 'fixed')",
        ] {
            db.execute(Statement::from_string(DbBackend::Sqlite, sql))
                .await
                .unwrap();
        }
        let mut navigation = Navigation::default();
        navigation.tabs.push(Tab::Section {
            id: "tech".into(),
            label: "技術".into(),
        });
        for (slug, section) in [("first", "blog"), ("second", "tech")] {
            let mut matter = crate::utils::front_matter::FrontMatter::new(
                slug.into(),
                slug.into(),
                false,
                Some("2020-01-01".into()),
                None,
                None,
                vec!["Rust".into()],
                vec!["Review".into()],
            );
            matter.section = Some(section.into());
            let id = crate::seed::article::seed_article(&db, &matter, "body")
                .await
                .unwrap();
            crate::seed::article::seed_tag(&db, &matter, id)
                .await
                .unwrap();
            crate::seed::article::seed_category(&db, &matter, id)
                .await
                .unwrap();
        }
        let all = context(&db, &navigation, None, 1, 10, false)
            .await
            .unwrap()
            .into_json();
        assert_eq!(all["articles"].as_array().unwrap().len(), 2);
        assert_eq!(all["section_tags"].as_array().unwrap().len(), 2);
        assert_eq!(all["section_categories"].as_array().unwrap().len(), 2);
        let tech = context(&db, &navigation, Some("tech"), 1, 10, true)
            .await
            .unwrap()
            .into_json();
        assert_eq!(tech["articles"].as_array().unwrap().len(), 1);
        assert_eq!(tech["articles"][0]["title"], "second");
        assert_eq!(tech["articles"][0]["url"], "/posts/second/");
        assert_eq!(tech["section_tags"].as_array().unwrap().len(), 1);
        let (articles, _) = crate::repository::article::get_articles_by_tag_slug(
            &db,
            crate::domain::page::Page { number: 1, per: 10 },
            "[tech] rust",
            "created_at",
        )
        .await
        .unwrap();
        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].slug, "second");
    }
}

impl Navigation {
    pub async fn validate_database(&self, db: &sea_orm::DatabaseConnection) -> Result<()> {
        use crate::entity::{article, fixed_content};
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};
        let sections: Vec<String> = article::Entity::find()
            .select_only()
            .column(article::Column::Section)
            .distinct()
            .into_tuple()
            .all(db)
            .await?;
        for section in sections {
            self.validate_section(&section)?;
        }
        for tab in &self.tabs {
            if let Tab::Page { slug, .. } = tab {
                ensure!(
                    fixed_content::Entity::find()
                        .filter(fixed_content::Column::Slug.eq(slug))
                        .one(db)
                        .await?
                        .is_some(),
                    "fixed page does not exist: {slug}"
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod navigation_render_tests {
    use super::*;

    #[test]
    fn links_and_labels_are_escaped_once_with_or_without_autoescape() {
        for autoescape in [true, false] {
            let mut tera = tera::Tera::default();
            if autoescape {
                tera.autoescape_on(vec!["navigation"]);
            }
            let mut navigation = Navigation::default();
            if let Tab::All { label, .. } = &mut navigation.tabs[0] {
                *label = "<All & articles>".into();
            }
            install_navigation(&mut tera, &navigation, false).unwrap();
            let html = tera
                .render("partial/navigation", &tera::Context::new())
                .unwrap();
            assert!(html.contains("href=\"&#x2F;all\""));
            assert!(html.contains("&lt;All &amp; articles&gt;"));
            assert!(!html.contains("&amp;#"));
        }
    }
}
