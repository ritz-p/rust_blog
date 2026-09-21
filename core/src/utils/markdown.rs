pub mod to_text;
mod toc;
use ammonia::Builder;
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, html};
use std::sync::LazyLock;
use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator},
    parsing::{SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};
pub use toc::toc;

use to_text::{end_tag, is_strikethrough, start_tag};

static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
static EXTRA_SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(two_face::syntax::extra_newlines);

pub fn markdown_to_html(input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let mut parser = Parser::new_ext(input, options);
    let mut events = Vec::new();
    while let Some(event) = parser.next() {
        if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(info))) = &event {
            let syntax = info.split_whitespace().next().and_then(|language| {
                let language = match language {
                    "bash" => "sh",
                    _ => language,
                };
                SYNTAX_SET
                    .find_syntax_by_token(language)
                    .map(|syntax| (syntax, &*SYNTAX_SET))
                    .or_else(|| {
                        EXTRA_SYNTAX_SET
                            .find_syntax_by_token(language)
                            .map(|syntax| (syntax, &*EXTRA_SYNTAX_SET))
                    })
            });
            if let Some((syntax, syntax_set)) = syntax {
                let mut code = String::new();
                for event in parser.by_ref() {
                    match event {
                        Event::Text(text) => code.push_str(&text),
                        Event::End(Tag::CodeBlock(_)) => break,
                        _ => (),
                    }
                }
                if let Some(highlighted) = highlight_code(&code, syntax, syntax_set) {
                    events.push(Event::Html(highlighted.into()));
                } else {
                    events.push(event.clone());
                    events.push(Event::Text(code.into()));
                    events.push(Event::End(Tag::CodeBlock(CodeBlockKind::Fenced(
                        info.clone(),
                    ))));
                }
                continue;
            }
        }
        events.push(event);
    }

    let mut html_output = String::new();
    html::push_html(&mut html_output, events.into_iter());

    sanitize_html(&html_output)
}

fn highlight_code(code: &str, syntax: &SyntaxReference, syntax_set: &SyntaxSet) -> Option<String> {
    let mut generator = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        syntax_set,
        ClassStyle::SpacedPrefixed { prefix: "syntax-" },
    );
    for line in LinesWithEndings::from(code) {
        generator
            .parse_html_for_line_which_includes_newline(line)
            .ok()?;
    }
    Some(format!(
        "<pre><code>{}</code></pre>\n",
        generator.finalize()
    ))
}

fn sanitize_html(html: &str) -> String {
    Builder::default()
        .add_tag_attributes("span", &["class"])
        .clean(html)
        .to_string()
}

pub fn markdown_to_text(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let mut broken_link = |link: pulldown_cmark::BrokenLink<'_>| {
        let reference = link.reference.as_ref();
        if reference.starts_with("https://") || reference.starts_with("http://") {
            Some((reference.to_owned().into(), "".into()))
        } else {
            None
        }
    };
    let parser = Parser::new_with_broken_link_callback(markdown, options, Some(&mut broken_link));
    let mut tags_stack = Vec::new();
    let mut buffer = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => {
                if !tags_stack.iter().any(is_strikethrough) {
                    start_tag(&tag, &mut buffer, &mut tags_stack);
                }
                tags_stack.push(tag);
            }
            Event::End(tag) => {
                tags_stack.pop();
                if !tags_stack.iter().any(is_strikethrough) {
                    end_tag(&tag, &mut buffer, &tags_stack);
                }
            }
            Event::Text(content) => {
                if !tags_stack.iter().any(is_strikethrough) {
                    buffer.push_str(&content)
                }
            }
            Event::Code(content) if !tags_stack.iter().any(is_strikethrough) => {
                buffer.push_str(&content)
            }
            Event::SoftBreak => buffer.push(' '),
            _ => (),
        }
    }
    buffer.trim().to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn toc_is_opt_in_and_omitted_without_headings() {
        let input = "## 見出し\n\nbody";
        assert!(!super::markdown_to_html(input).contains("table-of-contents"));
        assert_eq!(
            super::toc(&super::markdown_to_html("body")),
            super::markdown_to_html("body")
        );
    }

    #[test]
    fn toc_links_are_unique_and_follow_nested_heading_order() {
        let input = "## 同名\n\n#### **深い** `code` [link](https://example.com)\n\n## 同名\n\n末尾\n====\n\n```bash\n# not a heading\n```";
        let html = super::toc(&super::markdown_to_html(input));
        let toc = html.split("</nav>").next().unwrap();
        assert!(toc.contains("<a href=\"#toc-heading-1\">同名</a><ol><li><a href=\"#toc-heading-2\">深い code link</a>"));
        for i in 1..=4 {
            assert_eq!(html.matches(&format!("id=\"toc-heading-{i}\"")).count(), 1);
            assert_eq!(
                toc.matches(&format!("href=\"#toc-heading-{i}\"")).count(),
                1
            );
        }
        assert!(!toc.contains("not a heading"));
        assert_eq!(toc.matches("<ol>").count(), toc.matches("</ol>").count());
        assert_eq!(toc.matches("<li>").count(), toc.matches("</li>").count());
    }

    #[test]
    fn toc_escapes_labels_and_preserves_content_sanitization() {
        let html = super::toc(&super::markdown_to_html(
            "# `<img src=x onerror=alert(1)>` & text\n\n<script>alert(1)</script>\n\n[bad](javascript:alert(1))",
        ));
        assert!(html.contains("&lt;img src=x onerror=alert(1)&gt;"));
        assert!(!html.contains("<img"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("javascript:"));
    }

    use super::{markdown_to_html, markdown_to_text};

    fn without_spans(html: &str) -> String {
        ammonia::Builder::default()
            .rm_tags(&["span"])
            .clean(html)
            .to_string()
    }

    #[test]
    fn highlights_known_languages_and_aliases() {
        for (language, code) in [
            ("rust", "fn main() { let s = \"hello\"; } // comment\n"),
            ("rs", "fn main() { let s = \"hello\"; } // comment\n"),
            (
                "javascript",
                "function greet() { return \"hello\"; } // comment\n",
            ),
            ("js", "function greet() { return \"hello\"; } // comment\n"),
        ] {
            let html = markdown_to_html(&format!("```{language}\n{code}```\n"));
            assert!(html.starts_with("<pre><code><span"), "{language}: {html}");
            for scope in ["syntax-keyword", "syntax-string", "syntax-comment"] {
                assert!(
                    html.split('"')
                        .skip(1)
                        .step_by(2)
                        .any(|classes| { classes.split_whitespace().any(|class| class == scope) }),
                    "missing {scope} for {language}: {html}"
                );
            }
            assert!(!html.contains("style="));
            assert_eq!(
                without_spans(&html),
                markdown_to_html(&format!("```\n{code}```\n"))
            );
        }
    }

    #[test]
    fn highlights_kotlin_and_bash_with_aliases() {
        for (languages, code) in [
            (
                &["kotlin", "kt", "kts"][..],
                "fun main() { val text = \"<hello> & world\"; println(text) } // comment\n",
            ),
            (
                &["bash", "sh"][..],
                "# comment\nif true; then echo \"<hello> & world\"; fi\n",
            ),
        ] {
            let expected = markdown_to_html(&format!("```\n{code}```\n"));
            for language in languages {
                let html = markdown_to_html(&format!("```{language}\n{code}```\n"));
                for scope in ["syntax-string", "syntax-comment"] {
                    assert!(
                        html.contains(scope),
                        "missing {scope} for {language}: {html}"
                    );
                }
                assert!(
                    html.contains("syntax-keyword") || html.contains("syntax-storage"),
                    "{language}: {html}"
                );
                assert_eq!(without_spans(&html), expected, "{language}");
            }
        }
    }

    #[test]
    fn syntax_classes_are_namespaced_to_avoid_bulma_collisions() {
        let code = "println!(\"Hello,World\");\nlet n = 42;\n";
        let html = markdown_to_html(&format!("```rust\n{code}```\n"));

        let classes: Vec<_> = html
            .split("class=\"")
            .skip(1)
            .flat_map(|attribute| attribute.split('"').next().unwrap().split_whitespace())
            .collect();
        assert!(!classes.is_empty());
        assert!(
            classes.iter().all(|class| class.starts_with("syntax-")),
            "{html}"
        );
        // Bulma's .section otherwise adds padding to the macro's parentheses.
        assert!(
            html.contains("class=\"syntax-punctuation syntax-section syntax-group syntax-begin syntax-rust\">("),
            "{html}"
        );
        assert!(
            html.contains(
                "class=\"syntax-punctuation syntax-section syntax-group syntax-end syntax-rust\">)"
            ),
            "{html}"
        );
        assert!(
            html.contains("class=\"syntax-constant syntax-numeric syntax-integer syntax-decimal syntax-rust\">42</span>"),
            "{html}"
        );
        assert_eq!(
            without_spans(&html),
            markdown_to_html(&format!("```\n{code}```\n"))
        );
    }

    #[test]
    fn highlights_multiple_blocks_independently() {
        let rust = "```rust extra-info\n/* open comment\n```\n";
        let js = "```js\nconst value = \"hello\";\n```\n";
        assert_eq!(
            markdown_to_html(&format!("{rust}\nBetween.\n\n{js}")),
            format!(
                "{}<p>Between.</p>\n{}",
                markdown_to_html(rust),
                markdown_to_html(js)
            )
        );
    }

    #[test]
    fn leaves_plain_code_blocks_unchanged() {
        for markdown in [
            "```not-a-language\n<x> & value\n```\n",
            "```\n<x> & value\n```\n",
            "    <x> & value\n",
        ] {
            assert_eq!(
                markdown_to_html(markdown),
                "<pre><code>&lt;x&gt; &amp; value\n</code></pre>\n"
            );
        }
    }

    #[test]
    fn highlighting_preserves_whitespace_and_escapes_html() {
        let code =
            "fn main() {\n\tlet s = \"<script>alert('x')</script> & <img onerror=bad>\";  \n\n}\n";
        let html = markdown_to_html(&format!("```rust\n{code}```\n"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("<img"));
        assert_eq!(
            without_spans(&html),
            markdown_to_html(&format!("```\n{code}```\n"))
        );
    }

    #[test]
    fn inline_html_code_is_unchanged() {
        assert_eq!(
            markdown_to_html("Use `let x = <value> & 1;`."),
            "<p>Use <code>let x = &lt;value&gt; &amp; 1;</code>.</p>\n"
        );
    }

    #[test]
    fn allows_span_classes_but_strips_unsafe_attributes() {
        let html = markdown_to_html(
            r#"<span class="keyword" style="color:red" onclick="alert(1)">safe</span><img src="x" onerror="alert(1)"><a href="javascript:alert(1)">link</a>"#,
        );
        assert!(html.contains(r#"<span class="keyword">safe</span>"#));
        for unsafe_attribute in ["style=", "onclick=", "onerror=", "javascript:"] {
            assert!(!html.contains(unsafe_attribute), "{html}");
        }
    }

    #[test]
    fn basic_inline_strong() {
        let markdown = r#"**Hello**"#;
        let expected = "Hello";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn basic_inline_emphasis() {
        let markdown = r#"_Hello_"#;
        let expected = "Hello";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn basic_header() {
        let markdown = r#"# Header

## Sub header

End paragraph."#;
        let expected = "Header

Sub header

End paragraph.";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn alt_header() {
        let markdown = r#"
Header
======

End paragraph."#;
        let expected = "Header

End paragraph.";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn strong_emphasis() {
        let markdown = r#"**asterisks and _underscores_**"#;
        let expected = "asterisks and underscores";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn strikethrough() {
        let markdown = r#"This was ~~erased~~ deleted."#;
        let expected = "This was  deleted.";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn mixed_list() {
        let markdown = r#"Start paragraph.

1. First ordered list item
2. Another item
1. Actual numbers don't matter, just that it's a number
  1. Ordered sub-list
4. And another item.

End paragraph."#;

        let expected = "Start paragraph.

1. First ordered list item
2. Another item
3. Actual numbers don't matter, just that it's a number
4. Ordered sub-list
5. And another item.

End paragraph.";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn nested_lists() {
        let markdown = r#"
* alpha
* beta
    * one
    * two
* gamma
"#;
        let expected = "• alpha
• beta
\t• one
\t• two
• gamma";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn list_with_header() {
        let markdown = r#"# Title
* alpha
* beta
"#;
        let expected = r#"Title

• alpha
• beta"#;
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn basic_link() {
        let markdown = "I'm an [inline-style link](https://www.google.com).";
        let expected = "I'm an inline-style link.";
        assert_eq!(markdown_to_text(markdown), expected)
    }

    #[test]
    fn plain_text_ignores_link_and_image_titles() {
        for (markdown, expected) in [
            (r#"[label](https://example.com "tooltip")"#, "label"),
            (
                r#"[**bold** `code`](https://example.com "tooltip")"#,
                "bold code",
            ),
            (r#"![alt text](image.png "tooltip")"#, "alt text"),
            (
                "[label][ref]\n\n[ref]: https://example.com \"tooltip\"",
                "label",
            ),
            (r#"[](https://example.com "tooltip")"#, ""),
        ] {
            assert_eq!(markdown_to_text(markdown), expected, "{markdown}");
        }
    }

    #[test]
    fn link_with_itself() {
        let markdown = "Go to [https://www.google.com].";
        let expected = "Go to https://www.google.com.";
        assert_eq!(markdown_to_text(markdown), expected)
    }

    #[test]
    fn deleted_nested_content_is_not_an_excerpt() {
        assert_eq!(
            markdown_to_text(
                "before ~~**text** `code` [link](https://example.com \"title\")~~ after"
            ),
            "before  after"
        );
        assert_eq!(
            markdown_to_text("[ordinary words] and `[https://example.com]`"),
            "[ordinary words] and [https://example.com]"
        );
    }

    #[test]
    fn basic_image() {
        let markdown = "As displayed in ![img alt text](https://github.com/adam-p/markdown-here/raw/master/src/common/images/icon48.png).";
        let expected = "As displayed in img alt text.";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn inline_code() {
        let markdown = "This is `inline code`.";
        let expected = "This is inline code.";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn code_block() {
        let markdown = r#"Start paragraph.
```javascript
var s = "JavaScript syntax highlighting";
alert(s);
```
End paragraph."#;
        let expected = r#"Start paragraph.

var s = "JavaScript syntax highlighting";
alert(s);

End paragraph."#;
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn block_quote() {
        let markdown = r#"Start paragraph.

> Blockquotes are very handy in email to emulate reply text.
> This line is part of the same quote.

End paragraph."#;
        let expected = "Start paragraph.

Blockquotes are very handy in email to emulate reply text. This line is part of the same quote.

End paragraph.";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn paragraphs() {
        let markdown = r#"Paragraph 1.

Paragraph 2."#;
        let expected = "Paragraph 1.

Paragraph 2.";
        assert_eq!(markdown_to_text(markdown), expected);
    }

    #[test]
    fn strips_script_tag() {
        let markdown = r#"Hello<script>alert("xss")</script>World"#;
        let html = markdown_to_html(markdown);
        assert!(!html.contains("<script>"));
        assert!(html.contains("Hello"));
        assert!(html.contains("World"));
    }

    #[test]
    fn strips_javascript_link() {
        let markdown = r#"[click](javascript:alert(1))"#;
        let html = markdown_to_html(markdown);
        assert!(!html.contains("javascript:"));
    }
}
