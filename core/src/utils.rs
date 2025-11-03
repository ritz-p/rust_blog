use chrono::{DateTime, FixedOffset, Utc};

pub mod fixed_content_matter;
pub mod front_matter;
pub mod markdown;

pub fn cut_out_string(base: &str, limit: usize) -> String {
    let l = base.len().min(limit);
    base.chars().take(l).collect()
}

pub fn utc_to_jst(utc: DateTime<Utc>) -> String {
    let jst_offset = FixedOffset::east_opt(9 * 3600).unwrap_or(FixedOffset::east(0));
    utc.with_timezone(&jst_offset)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string()
}
