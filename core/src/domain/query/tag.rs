use crate::domain::query::PagingQuery;

#[derive(FromForm, Debug, Clone)]
pub struct TagQuery {
    pub page: Option<u64>,
    pub per: Option<u64>,
    pub sort_key: Option<String>,
}

impl PagingQuery for TagQuery {
    fn new() -> Self {
        Self {
            page: None,
            per: None,
            sort_key: None,
        }
    }
    fn page(&self) -> Option<u64> {
        self.page
    }
    fn per(&self) -> Option<u64> {
        self.per
    }
}
