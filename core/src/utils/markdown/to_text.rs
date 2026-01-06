use pulldown_cmark::Tag;

pub fn start_tag(tag: &Tag, buffer: &mut String, tags_stack: &mut Vec<Tag>) {
    match tag {
        Tag::Link(_, _, title) | Tag::Image(_, _, title) => buffer.push_str(&title),
        Tag::Item => {
            buffer.push('\n');
            let mut lists_stack = tags_stack
                .iter_mut()
                .filter_map(|tag| match tag {
                    Tag::List(nb) => Some(nb),
                    _ => None,
                })
                .collect::<Vec<_>>();
            let prefix_tabs_count = lists_stack.len() - 1;
            for _ in 0..prefix_tabs_count {
                buffer.push('\t')
            }
            if let Some(Some(nb)) = lists_stack.last_mut() {
                buffer.push_str(&nb.to_string());
                buffer.push_str(". ");
                *nb += 1;
            } else {
                buffer.push_str("• ");
            }
        }
        Tag::Paragraph | Tag::CodeBlock(_) | Tag::Heading(_, _, _) => buffer.push('\n'),
        _ => (),
    }
}

pub fn end_tag(tag: &Tag, buffer: &mut String, tags_stack: &[Tag]) {
    match tag {
        Tag::Paragraph | Tag::Heading(_, _, _) => buffer.push('\n'),
        Tag::CodeBlock(_) => {
            if buffer.chars().last() != Some('\n') {
                buffer.push('\n');
            }
        }
        Tag::List(_) => {
            let is_sublist = tags_stack.iter().any(|tag| match tag {
                Tag::List(_) => true,
                _ => false,
            });
            if !is_sublist {
                buffer.push('\n')
            }
        }
        _ => (),
    }
}

pub fn is_strikethrough(tag: &Tag) -> bool {
    match tag {
        Tag::Strikethrough => true,
        _ => false,
    }
}
