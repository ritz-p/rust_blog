use crate::domain::query::PagingQuery;

#[derive(Debug, Clone, Copy)]
pub struct Page {
    pub count: u64,
    pub per: u64,
}

impl Page {
    pub fn normalize(self, max: u64) -> Self {
        let count = self.count.max(1);
        let per = self.per.clamp(1, max);
        Self { count, per }
    }

    pub fn new_from_query<T>(query: &T) -> Self
    where
        T: PagingQuery,
    {
        Self {
            count: query.page().unwrap_or(1),
            per: query.per().unwrap_or(10),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PageInfo {
    pub count: u64,
    pub per: u64,
    pub total_pages: u64,
    pub has_prev: bool,
    pub has_next: bool,
    pub prev_page: u64,
    pub next_page: u64,
}

impl PageInfo {
    pub fn new(page: Page, total: u64) -> Self {
        let total_pages = ((total + page.per - 1) / page.per).max(1);
        let count = page.count.clamp(1, total_pages);

        let has_prev = count > 1;
        let has_next = count < total_pages;

        Self {
            count: count,
            per: page.per,
            total_pages,
            has_prev,
            has_next,
            prev_page: if has_prev { count - 1 } else { 1 },
            next_page: if has_next { count + 1 } else { total_pages },
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
                    "&sort_key=".to_owned() + &key
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
                    "&sort_key=".to_owned() + &key
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
        let page = Page::new_from_query(&query);
        assert_eq!(page.count, 1);
        assert_eq!(page.per, 10);
    }

    #[test]
    fn page_new_from_query_uses_query_values() {
        let query = MockQuery {
            page: Some(3),
            per: Some(25),
        };
        let page = Page::new_from_query(&query);
        assert_eq!(page.count, 3);
        assert_eq!(page.per, 25);
    }

    #[test]
    fn page_normalize_clamps_values() {
        let page = Page { count: 0, per: 100 }.normalize(30);
        assert_eq!(page.count, 1);
        assert_eq!(page.per, 30);
    }

    #[test]
    fn page_info_new_sets_bounds_and_navigation_flags() {
        let info = PageInfo::new(Page { count: 99, per: 10 }, 95);
        assert_eq!(info.count, 10);
        assert_eq!(info.total_pages, 10);
        assert!(info.has_prev);
        assert!(!info.has_next);
        assert_eq!(info.prev_page, 9);
        assert_eq!(info.next_page, 10);
    }

    #[test]
    fn page_info_new_handles_first_page_and_empty_total() {
        let info = PageInfo::new(Page { count: 1, per: 10 }, 0);
        assert_eq!(info.count, 1);
        assert_eq!(info.total_pages, 1);
        assert!(!info.has_prev);
        assert!(!info.has_next);
        assert_eq!(info.prev_page, 1);
        assert_eq!(info.next_page, 1);
    }

    #[test]
    fn page_info_prev_next_url_include_sort_key() {
        let info = PageInfo::new(Page { count: 2, per: 10 }, 50);
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
        let info = PageInfo::new(Page { count: 1, per: 10 }, 5);
        assert_eq!(info.get_prev_url("/articles", None), "");
        assert_eq!(info.get_next_url("/articles", None), "");
    }
}
