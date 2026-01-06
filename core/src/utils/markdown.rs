pub mod to_text;
use pulldown_cmark::{Event, Options, Parser, html};
use to_text::{end_tag, is_strikethrough, start_tag};

pub fn markdown_to_html(input: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(input, options);

    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    html_output
}

pub fn markdown_to_text(markdown: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);

    let parser = Parser::new_ext(&markdown, options);
    let mut tags_stack = Vec::new();
    let mut buffer = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => {
                start_tag(&tag, &mut buffer, &mut tags_stack);
                tags_stack.push(tag);
            }
            Event::End(tag) => {
                tags_stack.pop();
                end_tag(&tag, &mut buffer, &tags_stack);
            }
            Event::Text(content) => {
                if !tags_stack.iter().any(is_strikethrough) {
                    buffer.push_str(&content)
                }
            }
            Event::Code(content) => buffer.push_str(&content),
            Event::SoftBreak => buffer.push(' '),
            _ => (),
        }
    }
    buffer.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::markdown_to_text;

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

    // TODO Fix
    #[ignore]
    #[test]
    fn link_with_itself() {
        let markdown = "Go to [https://www.google.com].";
        let expected = "Go to https://www.google.com.";
        assert_eq!(markdown_to_text(markdown), expected)
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
}
