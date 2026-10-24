use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct FrontMatter {
    pub title: String,
    pub slug: String,
    #[serde(default)]
    pub deleted: bool,
    #[serde(default)]
    pub table_of_contents: bool,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default, alias = "date")]
    pub created_at: Option<String>,
    pub excerpt: Option<String>,
    pub icatch_path: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub categories: Vec<String>,
}

impl FrontMatter {
    #[allow(dead_code)]
    #[allow(
        clippy::too_many_arguments,
        reason = "Constructor mirrors the front matter fields"
    )]
    pub fn new(
        title: String,
        slug: String,
        deleted: bool,
        created_at: Option<String>,
        excerpt: Option<String>,
        icatch_path: Option<String>,
        tags: Vec<String>,
        categories: Vec<String>,
    ) -> Self {
        Self {
            title,
            slug,
            deleted,
            table_of_contents: false,
            deprecated: false,
            created_at,
            excerpt,
            icatch_path,
            tags,
            categories,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FrontMatter;

    #[test]
    fn article_flags_are_opt_in() {
        let base = "title: Test\nslug: test\ntags: []\ncategories: []\n";
        for field in ["table_of_contents", "deprecated"] {
            for (setting, expected) in [
                (String::new(), false),
                (format!("{field}: false\n"), false),
                (format!("{field}: true\n"), true),
            ] {
                let matter: FrontMatter =
                    serde_yaml::from_str(&format!("{base}{setting}")).unwrap();
                assert_eq!(
                    if field == "deprecated" {
                        matter.deprecated
                    } else {
                        matter.table_of_contents
                    },
                    expected
                );
            }
            assert!(
                serde_yaml::from_str::<FrontMatter>(&format!("{base}{field}: invalid")).is_err()
            );
        }
    }
}
