use crate::{entity::article, utils::markdown::markdown_to_text};

#[derive(Debug, serde::Serialize)]
pub struct Fragment {
    pub text: String,
    pub matched: bool,
}

pub fn fragments(text: &str, query: &str, excerpt: bool) -> Vec<Fragment> {
    let chars: Vec<char> = text.chars().collect();
    let folded = text.to_lowercase();
    let mut origins = Vec::with_capacity(folded.len());
    // Contextual default casing (Σ → ς/σ) preserves UTF-8 lengths, so
    // per-character lengths can map whole-string lowercase back to its source.
    for (index, ch) in chars.iter().enumerate() {
        let lower_len = ch.to_lowercase().map(char::len_utf8).sum();
        origins.extend(std::iter::repeat_n(index, lower_len));
    }
    let mut matches = vec![false; chars.len()];
    for term in terms(query) {
        for (start, value) in folded.match_indices(&term) {
            let first = origins[start];
            let last = origins[start + value.len() - 1];
            matches[first..=last].fill(true);
        }
    }
    let start = if excerpt {
        matches
            .iter()
            .position(|value| *value)
            .unwrap_or(0)
            .saturating_sub(40)
    } else {
        0
    };
    let end = if excerpt {
        (start + 160).min(chars.len())
    } else {
        chars.len()
    };
    let mut output: Vec<Fragment> = Vec::new();
    if start > 0 {
        output.push(Fragment {
            text: "…".into(),
            matched: false,
        });
    }
    for index in start..end {
        if let Some(last) = output.last_mut()
            && last.matched == matches[index]
        {
            last.text.push(chars[index]);
        } else {
            output.push(Fragment {
                text: chars[index].to_string(),
                matched: matches[index],
            });
        }
    }
    if end < chars.len() {
        output.push(Fragment {
            text: "…".into(),
            matched: false,
        });
    }
    output
}

pub fn excerpt_fragments(body: &str, fallback: &str, query: &str) -> Vec<Fragment> {
    let body = markdown_to_text(body);
    let highlighted = fragments(&body, query, true);
    if highlighted.iter().any(|part| part.matched) {
        highlighted
    } else {
        fragments(fallback, query, true)
    }
}

pub fn terms(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

pub fn article_text(article: &article::Model) -> String {
    format!(
        "{}\n{}\n{}",
        article.title,
        markdown_to_text(article.excerpt.as_deref().unwrap_or_default()),
        markdown_to_text(&article.content),
    )
    .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragments_preserve_contextual_lowercase_and_original_character_offsets() {
        for (text, query, expected) in [
            ("ΟΣ", "ΟΣ", vec![("ΟΣ", true)]),
            (
                "İ 😀 ΟΣ ΟΣΑ ΟΣ\u{301}",
                "ος",
                vec![
                    ("İ 😀 ", false),
                    ("ΟΣ", true),
                    (" ΟΣΑ ", false),
                    ("ΟΣ", true),
                    ("\u{301}", false),
                ],
            ),
            (
                "İ 😀 ΟΣ",
                "i\u{307} ος",
                vec![("İ", true), (" 😀 ", false), ("ΟΣ", true)],
            ),
            (
                "Σ ΟΣΑ",
                "σ",
                vec![("Σ", true), (" Ο", false), ("Σ", true), ("Α", false)],
            ),
        ] {
            let parts = fragments(text, query, false);
            assert_eq!(
                parts
                    .iter()
                    .map(|part| (part.text.as_str(), part.matched))
                    .collect::<Vec<_>>(),
                expected,
                "text: {text}, query: {query}",
            );
        }
    }

    #[test]
    fn excerpts_use_contextual_body_matches_and_highlight_fallbacks() {
        let body = format!("{} İ 😀 **ΟΣ** after", "前文".repeat(150));
        let parts = excerpt_fragments(&body, "unrelated excerpt", "ΟΣ");
        let text = parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<String>();
        assert!(text.starts_with('…'));
        assert!(text.contains("İ 😀 ΟΣ after"));
        assert!(parts.iter().any(|part| part.matched && part.text == "ΟΣ"));

        let fallback = excerpt_fragments("no body match", "İ 😀 ΟΣ", "i\u{307} ος");
        assert_eq!(
            fallback
                .iter()
                .map(|part| (part.text.as_str(), part.matched))
                .collect::<Vec<_>>(),
            vec![("İ", true), (" 😀 ", false), ("ΟΣ", true)],
        );
    }

    #[test]
    fn excerpts_find_late_matches_and_preserve_unicode_and_html_as_text() {
        let body = format!("{} **東京** <literal> İSTANBUL 😀", "前文".repeat(150));
        let parts = fragments(&body, "東京 istanbul literal", true);
        let text = parts
            .iter()
            .map(|part| part.text.as_str())
            .collect::<String>();
        assert!(text.starts_with('…') && text.contains("東京") && text.contains('😀'));
        assert!(parts.iter().any(|part| part.matched && part.text == "東京"));
        assert!(parts.iter().any(|part| part.text.contains('<')));
        let fallback = excerpt_fragments("no body match", "Title-only fallback", "title");
        assert_eq!(fallback[0].text, "Title");
        assert!(fallback[0].matched);
    }
}
