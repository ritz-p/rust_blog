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

    pub fn offset(self) -> u64 {
        (self.count - 1) * self.per
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
    pub total: u64,
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
            total,
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
