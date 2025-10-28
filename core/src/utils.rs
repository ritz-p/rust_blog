pub mod fixed_content_matter;
pub mod front_matter;
pub mod markdown;

pub fn cut_out_string(base: &str, limit: usize) -> String {
    let l = base.len().min(limit);
    base[0..l].to_string()
}
