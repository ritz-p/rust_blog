use std::cell::RefCell;

use lol_html::{RewriteStrSettings, element, rewrite_str, text};

/// Adds a table of contents to sanitized HTML produced by `markdown_to_html`.
/// The original HTML is retained when there are no headings or rewriting fails.
pub fn toc(input: &str) -> String {
    let headings = RefCell::new(Vec::<(u8, String, String)>::new());
    let selector = "h1, h2, h3, h4, h5, h6";
    let result = rewrite_str(
        input,
        RewriteStrSettings::new()
            .append_element_content_handler(element!(selector, |el| {
                let mut headings = headings.borrow_mut();
                let id = format!("toc-heading-{}", headings.len() + 1);
                el.set_attribute("id", &id)?;
                let level = el.tag_name().as_bytes()[1] - b'0';
                headings.push((level, id, String::new()));
                Ok(())
            }))
            .append_element_content_handler(text!(selector, |text| {
                if let Some(heading) = headings.borrow_mut().last_mut() {
                    heading.2.push_str(text.as_str());
                }
                Ok(())
            }))
            .append_element_content_handler(element!(
                "h1 br, h2 br, h3 br, h4 br, h5 br, h6 br",
                |_| {
                    if let Some(heading) = headings.borrow_mut().last_mut() {
                        heading.2.push(' ');
                    }
                    Ok(())
                }
            ))
            .append_element_content_handler(element!(
                "h1 img, h2 img, h3 img, h4 img, h5 img, h6 img",
                |el| {
                    if let Some(alt) = el.get_attribute("alt") {
                        if let Some(heading) = headings.borrow_mut().last_mut() {
                            heading.2.push_str(&alt);
                        }
                    }
                    Ok(())
                }
            )),
    );
    let Ok(content) = result else {
        return input.to_owned();
    };
    let mut headings = headings.into_inner();
    if headings.is_empty() {
        return input.to_owned();
    }
    for (_, _, label) in &mut headings {
        *label = html_escape::decode_html_entities(label).into_owned();
    }
    let mut output = format!(
        "<nav class=\"table-of-contents\" aria-label=\"目次\"><div class=\"toc-header\"><span class=\"toc-title\">目次</span><span class=\"toc-count\">{}項目</span></div>",
        headings.len()
    );
    render_toc(&headings, &mut 0, 0, &mut output);
    output.push_str("</nav>\n");
    output.push_str(&content);
    output
}

fn render_toc(headings: &[(u8, String, String)], index: &mut usize, parent: u8, html: &mut String) {
    html.push_str("<ol>");
    while *index < headings.len() && headings[*index].0 > parent {
        let (level, id, text) = &headings[*index];
        let escaped = html_escape::encode_safe(text);
        html.push_str(&format!("<li><a href=\"#{id}\">{escaped}</a>"));
        *index += 1;
        if *index < headings.len() && headings[*index].0 > *level {
            render_toc(headings, index, *level, html);
        }
        html.push_str("</li>");
    }
    html.push_str("</ol>");
}

#[cfg(test)]
mod tests {
    use super::toc;
    use crate::utils::markdown::markdown_to_html;

    #[test]
    fn toc_preserves_rendered_body_except_heading_ids() {
        let rendered = markdown_to_html(
            "## A & B\n\n```rust\nfn main() {}\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |\n\n<div><h3 title=\"a > b\">HTML &amp; heading</h3></div>",
        );
        let output = toc(&rendered);
        let (nav, body) = output.split_once("</nav>\n").unwrap();
        assert!(nav.contains(">A &amp; B</a>"));
        assert!(nav.contains(">HTML &amp; heading</a>"));
        assert!(!nav.contains("&amp;amp;"));
        assert_eq!(
            body.replace(" id=\"toc-heading-1\"", "")
                .replace(" id=\"toc-heading-2\"", ""),
            rendered
        );
    }
}
