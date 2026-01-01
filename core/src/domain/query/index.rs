use crate::domain::query::PagingQuery;

#[derive(FromForm, Debug, Clone, Copy)]
pub struct IndexQuery {
    pub page: Option<u64>,
    pub per: Option<u64>,
}

impl PagingQuery for IndexQuery {
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
