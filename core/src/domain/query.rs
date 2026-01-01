pub mod category;
pub mod index;
pub mod tag;

pub trait PagingQuery {
    fn new() -> Self;
    fn page(&self) -> Option<u64>;
    fn per(&self) -> Option<u64>;
}
