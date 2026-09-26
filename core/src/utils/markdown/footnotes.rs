use pulldown_cmark::{Event, Tag};
use std::collections::HashMap;

pub(super) fn prepare<'a>(
    events: Vec<Event<'a>>,
    input: &str,
) -> (Vec<Event<'a>>, Vec<(String, String)>) {
    let mut notes: HashMap<String, (usize, usize)> = HashMap::new();
    for event in &events {
        if let Event::FootnoteReference(label) = event {
            let next = notes.len() + 1;
            notes.entry(label.to_string()).or_insert((next, 0)).1 += 1;
        }
    }
    let mut prefix = "BLOGFOOTNOTEMARKER".to_owned();
    while input.contains(&prefix) {
        prefix.push('X');
    }
    let mut replacements = Vec::new();
    let mut marker = |html: String| {
        let key = format!("{prefix}{}END", replacements.len());
        replacements.push((key.clone(), html));
        Event::Text(key.into())
    };
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut output = Vec::new();
    for event in events {
        match event {
            Event::FootnoteReference(label) => {
                let (number, _) = notes[&label.to_string()];
                let reference = seen.entry(label.to_string()).or_default();
                *reference += 1;
                output.push(marker(format!("<sup id=\"footnote-ref-{number}-{reference}\"><a href=\"#footnote-{number}\" aria-label=\"脚注 {number}\">{number}</a></sup>")));
            }
            Event::Start(Tag::FootnoteDefinition(label)) => {
                let next = notes.len() + 1;
                let (number, _) = *notes.entry(label.to_string()).or_insert((next, 0));
                output.push(Event::Html("<div>".into()));
                output.push(marker(format!(
                    "<span id=\"footnote-{number}\" tabindex=\"-1\">{number}. </span>"
                )));
            }
            Event::End(Tag::FootnoteDefinition(label)) => {
                let (number, count) = notes[&label.to_string()];
                for reference in 1..=count {
                    output.push(marker(format!(" <a href=\"#footnote-ref-{number}-{reference}\" aria-label=\"脚注 {number} の参照 {reference} に戻る\">↩{reference}</a>")));
                }
                output.push(Event::Html("</div>".into()));
            }
            other => output.push(other),
        }
    }
    (output, replacements)
}

#[cfg(test)]
mod tests {
    use crate::utils::markdown::markdown_to_html;

    #[test]
    fn repeated_footnotes_have_distinct_backlinks_and_preserve_sanitization() {
        let html = markdown_to_html(
            "本文[^日本語] と再参照[^日本語]\n\n[^日本語]: **脚注** <script>alert(1)</script>\n\n<div id=\"footnote-1\">raw</div>\nBLOGFOOTNOTEMARKER0END",
        );
        assert_eq!(html.matches("id=\"footnote-1\"").count(), 1);
        assert_eq!(html.matches("href=\"#footnote-1\"").count(), 2);
        for reference in 1..=2 {
            assert_eq!(
                html.matches(&format!("id=\"footnote-ref-1-{reference}\""))
                    .count(),
                1
            );
            assert_eq!(
                html.matches(&format!("href=\"#footnote-ref-1-{reference}\""))
                    .count(),
                1
            );
        }
        assert!(html.contains("<strong>脚注</strong>"));
        assert!(!html.contains("<script"));
        assert!(html.contains("BLOGFOOTNOTEMARKER0END"));
    }
}
