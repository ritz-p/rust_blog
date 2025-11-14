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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::markdown::markdown_to_html;

    #[test]
    fn test_cut_out_string_basic() {
        let input = "Rust is awesome!";
        let output = cut_out_string(input, 4);
        assert_eq!(output, "Rust");
    }

    #[test]
    fn test_cut_out_string_length_longer_than_input() {
        let input = "Hi";
        let output = cut_out_string(input, 10);
        assert_eq!(output, "Hi");
    }

    #[test]
    fn test_utc_to_jst_format() {
        use chrono::{TimeZone, Utc};
        let utc_time = Utc.ymd(2025, 1, 1).and_hms(0, 0, 0);
        let jst_string = utc_to_jst(utc_time);
        assert!(
            jst_string.starts_with("2025-01-01 09:00:00"),
            "JST time should be 9 hours ahead of UTC"
        );
    }

    #[test]
    fn test_markdown_to_html_basic() {
        let md = "# Title\nHello **Rust**!";
        let html = markdown_to_html(md);

        assert!(
            html.contains("<h1>Title</h1>"),
            "Header should be converted to <h1>"
        );
        assert!(
            html.contains("<strong>Rust</strong>"),
            "Bold text should be <strong>"
        );
    }
}
