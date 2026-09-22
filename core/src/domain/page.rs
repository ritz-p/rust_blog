use crate::domain::query::PagingQuery;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PageLink {
    pub number: u64,
    pub url: String,
    pub current: bool,
}

#[derive(Debug, Serialize)]
pub struct Pagination {
    pub latest_url: String,
    pub oldest_url: String,
    pub pages: Vec<PageLink>,
}

#[derive(Debug, Clone, Copy)]
pub struct Page {
    pub number: u64,
    pub per: u64,
}

impl Page {
    pub fn normalize(self, max: u64) -> Self {
        let number = self.number.max(1);
        let per = self.per.clamp(1, max);
        Self { number, per }
    }

    pub fn new_from_query<T>(query: &T, default_per: u64) -> Self
    where
        T: PagingQuery,
    {
        Self {
            number: query.page().unwrap_or(1),
            per: query.per().unwrap_or(default_per),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PageInfo {
    pub current_page: u64,
    pub per: u64,
    pub total_pages: u64,
    pub has_prev: bool,
    pub has_next: bool,
    pub prev_page: u64,
    pub next_page: u64,
}

impl PageInfo {
    pub fn navigation(&self, url: impl Fn(u64) -> String) -> Pagination {
        let start = self.current_page.saturating_sub(3).max(1);
        let end = self.current_page.saturating_add(3).min(self.total_pages);
        Pagination {
            latest_url: url(1),
            oldest_url: url(self.total_pages),
            pages: (start..=end)
                .map(|number| PageLink {
                    number,
                    url: url(number),
                    current: number == self.current_page,
                })
                .collect(),
        }
    }

    pub fn get_page_url(&self, number: u64, base_path: &str, sort_key: Option<&String>) -> String {
        let mut url = format!("{base_path}?page={number}&per={}", self.per);
        if let Some(key) = sort_key {
            url.push_str("&sort_key=");
            url.push_str(&crate::utils::url_segment(key));
        }
        url
    }

    pub fn new(page: Page, total: u64) -> Self {
        let total_pages = total.div_ceil(page.per).max(1);
        let current_page = page.number.clamp(1, total_pages);

        let has_prev = current_page > 1;
        let has_next = current_page < total_pages;

        Self {
            current_page,
            per: page.per,
            total_pages,
            has_prev,
            has_next,
            prev_page: if has_prev { current_page - 1 } else { 1 },
            next_page: if has_next {
                current_page + 1
            } else {
                total_pages
            },
        }
    }
    pub fn get_prev_url(&self, base_path: &str, sort_key: Option<&String>) -> String {
        if self.has_prev {
            format!(
                "{}?page={}&per={}{}",
                base_path,
                self.prev_page,
                self.per,
                if let Some(key) = sort_key {
                    "&sort_key=".to_owned() + key
                } else {
                    "".to_owned()
                }
            )
        } else {
            String::new()
        }
    }
    pub fn get_next_url(&self, base_path: &str, sort_key: Option<&String>) -> String {
        if self.has_next {
            format!(
                "{}?page={}&per={}{}",
                base_path,
                self.next_page,
                self.per,
                if let Some(key) = sort_key {
                    "&sort_key=".to_owned() + key
                } else {
                    "".to_owned()
                }
            )
        } else {
            String::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Page, PageInfo};
    use crate::domain::query::PagingQuery;
    use crate::repository::SQLITE_MAX;

    #[test]
    fn navigation_limits_numbers_to_three_pages_on_each_side() {
        for (current, total, expected) in [
            (1, 0, vec![1]),
            (1, 10, vec![1, 2, 3, 4]),
            (5, 10, vec![2, 3, 4, 5, 6, 7, 8]),
            (10, 10, vec![7, 8, 9, 10]),
        ] {
            let info = PageInfo::new(
                Page {
                    number: current,
                    per: 1,
                },
                total,
            );
            let navigation = info.navigation(|number| format!("/page/{number}"));
            assert_eq!(
                navigation
                    .pages
                    .iter()
                    .map(|page| page.number)
                    .collect::<Vec<_>>(),
                expected
            );
            assert_eq!(
                navigation.pages.iter().filter(|page| page.current).count(),
                1
            );
            assert_eq!(navigation.latest_url, "/page/1");
            assert_eq!(navigation.oldest_url, format!("/page/{}", total.max(1)));
        }
        let info = PageInfo::new(
            Page {
                number: u64::MAX,
                per: 1,
            },
            u64::MAX,
        );
        assert_eq!(info.navigation(|page| page.to_string()).pages.len(), 4);
    }

    #[test]
    fn numbered_urls_keep_page_size_and_sort_order() {
        let info = PageInfo::new(Page { number: 5, per: 3 }, 30);
        let sort = "updated_at".to_string();
        let links = info.navigation(|number| info.get_page_url(number, "/tag/rust", Some(&sort)));
        assert_eq!(
            links.latest_url,
            "/tag/rust?page=1&per=3&sort_key=updated_at"
        );
        assert_eq!(
            links.oldest_url,
            "/tag/rust?page=10&per=3&sort_key=updated_at"
        );
        assert_eq!(
            links.pages[0].url,
            "/tag/rust?page=2&per=3&sort_key=updated_at"
        );
    }

    #[derive(Clone, Copy)]
    struct MockQuery {
        page: Option<u64>,
        per: Option<u64>,
    }

    impl PagingQuery for MockQuery {
        fn new() -> Self {
            Self {
                page: None,
                per: None,
            }
        }

        fn page(&self) -> Option<u64> {
            self.page
        }

        fn per(&self) -> Option<u64> {
            self.per
        }
    }

    #[test]
    fn page_new_from_query_uses_defaults() {
        let query = MockQuery::new();
        let page = Page::new_from_query(&query, 10);
        assert_eq!(page.number, 1);
        assert_eq!(page.per, 10);
    }

    #[test]
    fn configured_page_size_has_no_fifty_article_cap() {
        let query = MockQuery::new();
        let page = Page::new_from_query(&query, 75).normalize(SQLITE_MAX);
        assert_eq!(page.per, 75);
        let info = PageInfo::new(page, 151);
        assert_eq!(info.total_pages, 3);
        assert_eq!(info.get_next_url("/", None), "/?page=2&per=75");
    }

    #[test]
    fn page_new_from_query_uses_query_values() {
        let query = MockQuery {
            page: Some(3),
            per: Some(25),
        };
        let page = Page::new_from_query(&query, 10);
        assert_eq!(page.number, 3);
        assert_eq!(page.per, 25);
    }

    #[test]
    fn page_normalize_clamps_values() {
        let page = Page {
            number: 0,
            per: 100,
        }
        .normalize(30);
        assert_eq!(page.number, 1);
        assert_eq!(page.per, 30);
    }

    #[test]
    fn page_info_new_sets_bounds_and_navigation_flags() {
        let info = PageInfo::new(
            Page {
                number: 99,
                per: 10,
            },
            95,
        );
        assert_eq!(info.current_page, 10);
        assert_eq!(info.total_pages, 10);
        assert!(info.has_prev);
        assert!(!info.has_next);
        assert_eq!(info.prev_page, 9);
        assert_eq!(info.next_page, 10);
    }

    #[test]
    fn page_info_new_handles_first_page_and_empty_total() {
        let info = PageInfo::new(Page { number: 1, per: 10 }, 0);
        assert_eq!(info.current_page, 1);
        assert_eq!(info.total_pages, 1);
        assert!(!info.has_prev);
        assert!(!info.has_next);
        assert_eq!(info.prev_page, 1);
        assert_eq!(info.next_page, 1);
    }

    #[test]
    fn page_info_prev_next_url_include_sort_key() {
        let info = PageInfo::new(Page { number: 2, per: 10 }, 50);
        let sort_key = "updated_at".to_string();
        assert_eq!(
            info.get_prev_url("/tags/rust", Some(&sort_key)),
            "/tags/rust?page=1&per=10&sort_key=updated_at"
        );
        assert_eq!(
            info.get_next_url("/tags/rust", Some(&sort_key)),
            "/tags/rust?page=3&per=10&sort_key=updated_at"
        );
    }

    #[test]
    fn page_info_prev_next_url_return_empty_when_no_navigation() {
        let info = PageInfo::new(Page { number: 1, per: 10 }, 5);
        assert_eq!(info.get_prev_url("/articles", None), "");
        assert_eq!(info.get_next_url("/articles", None), "");
    }
}
