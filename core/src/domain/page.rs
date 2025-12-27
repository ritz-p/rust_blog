#[derive(Debug, Clone, Copy)]
pub struct Page {
    pub page: u64,
    pub per: u64,
}

impl Page {
    pub fn normalize(self, max_per: u64) -> Self {
        let page = self.page.max(1);
        let per = self.per.clamp(1, max_per);
        Self { page, per }
    }

    pub fn offset(self) -> u64 {
        (self.page - 1) * self.per
    }
}

#[derive(Debug, Clone)]
pub struct PageInfo {
    pub page: u64,
    pub per: u64,
    pub total: u64,
    pub total_pages: u64,
    pub has_prev: bool,
    pub has_next: bool,
    pub prev_page: u64,
    pub next_page: u64,
}

impl PageInfo {
    pub fn new(page: u64, per: u64, total: u64) -> Self {
        let total_pages = ((total + per - 1) / per).max(1);
        let page = page.clamp(1, total_pages);

        let has_prev = page > 1;
        let has_next = page < total_pages;

        Self {
            page,
            per,
            total,
            total_pages,
            has_prev,
            has_next,
            prev_page: if has_prev { page - 1 } else { 1 },
            next_page: if has_next { page + 1 } else { total_pages },
        }
    }
}

#[derive(FromForm, Debug, Clone, Copy)]
pub struct PagingQuery {
    pub page: Option<u64>,
    pub per: Option<u64>,
}
