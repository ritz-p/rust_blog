use rocket::FromForm;

use crate::domain::query::PagingQuery;

#[derive(FromForm, Debug, Clone)]
pub struct IndexQuery {
    pub page: Option<u64>,
    pub per: Option<u64>,
    pub year: Option<i32>,
    pub month: Option<u32>,
    pub q: Option<String>,
}

impl PagingQuery for IndexQuery {
    fn new() -> Self {
        Self {
            page: None,
            per: None,
            year: None,
            month: None,
            q: None,
        }
    }
    fn page(&self) -> Option<u64> {
        self.page
    }
    fn per(&self) -> Option<u64> {
        self.per
    }
}
