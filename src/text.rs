use std::{borrow::Cow, fmt::Write};

use console::Style;
use unicode_segmentation::UnicodeSegmentation;

use crate::view::{SchemaNodeRole, ViewDetailKind, ViewDocument, ViewNode, ViewNodeRole};

#[derive(clap::ValueEnum, Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Layout {
    #[default]
    Sections,
    Tree,
}

#[derive(Default)]
pub struct RenderOptions {
    pub layout: Layout,
    pub max_width: Option<usize>,
}

pub struct ColorTheme {
    pub key: Style,
    pub value: Style,
    pub connector: Style,
    pub constraint: Style,
    pub annotation: Style,
    pub metadata: Style,
    pub marker: Style,
}

impl Default for ColorTheme {
    fn default() -> Self {
        Self {
            key: Style::new().cyan(),
            value: Style::new().yellow(),
            connector: Style::new().blue(),
            constraint: Style::new().green(),
            annotation: Style::new().magenta(),
            metadata: Style::new().magenta(),
            marker: Style::new().red(),
        }
    }
}

pub fn render(document: &ViewDocument, theme: &ColorTheme, options: &RenderOptions) -> String {
    let mut output = String::new();
    match options.layout {
        Layout::Sections => {
            for (index, root) in document.roots.iter().enumerate() {
                if index > 0 {
                    output.push('\n');
                }
                render_node(root, "", "", theme, options, &mut output);
            }
        }
        Layout::Tree => {
            write!(output, "{}", theme.value.apply_to("Schema")).expect("writing to String");
            finish_line(&mut output, 0, options.max_width);
            render_children(&document.roots, "", theme, options, &mut output);
        }
    }
    output
}

fn render_node(
    node: &ViewNode,
    prefix: &str,
    connector: &str,
    theme: &ColorTheme,
    options: &RenderOptions,
    output: &mut String,
) {
    let line_start = output.len();
    if !prefix.is_empty() {
        write!(output, "{}", theme.connector.apply_to(prefix)).expect("writing to String");
    }
    if !connector.is_empty() {
        write!(output, "{}", theme.connector.apply_to(connector)).expect("writing to String");
    }
    let (key_style, value_style) = match node.role {
        ViewNodeRole::Schema(
            SchemaNodeRole::Instance
            | SchemaNodeRole::Definition
            | SchemaNodeRole::PatternProperty
            | SchemaNodeRole::PropertyName
            | SchemaNodeRole::Contains
            | SchemaNodeRole::ContentSchema
            | SchemaNodeRole::UnevaluatedProperties
            | SchemaNodeRole::UnevaluatedItems
            | SchemaNodeRole::ReferenceApplicator
            | SchemaNodeRole::ExpandedReference
            | SchemaNodeRole::LogicalBranch
            | SchemaNodeRole::DependentSchema
            | SchemaNodeRole::Condition
            | SchemaNodeRole::ThenBranch
            | SchemaNodeRole::ElseBranch,
        ) => (&theme.key, &theme.value),
        ViewNodeRole::Constraint => (&theme.constraint, &theme.constraint),
        ViewNodeRole::Section => (&theme.value, &theme.value),
        ViewNodeRole::Omission => (&theme.marker, &theme.marker),
    };
    if let Some(key) = &node.key {
        write!(
            output,
            "{} ",
            key_style.apply_to(escape_control_characters(key))
        )
        .expect("writing to String");
    }
    write!(
        output,
        "{}",
        value_style.apply_to(escape_control_characters(&node.value))
    )
    .expect("writing to String");
    for detail in &node.details {
        let style = match detail.kind {
            ViewDetailKind::Constraint => &theme.constraint,
            ViewDetailKind::Annotation => &theme.annotation,
            ViewDetailKind::Metadata => &theme.metadata,
            ViewDetailKind::Marker => &theme.marker,
        };
        let text = escape_control_characters(&detail.text);
        write!(output, " {}", style.apply_to(format!("[{text}]"))).expect("writing to String");
    }
    finish_line(output, line_start, options.max_width);

    let child_prefix = if connector == "├─ " {
        format!("{prefix}│  ")
    } else if connector == "└─ " {
        format!("{prefix}   ")
    } else {
        prefix.to_owned()
    };
    render_children(&node.children, &child_prefix, theme, options, output);
}

fn render_children(
    children: &[ViewNode],
    prefix: &str,
    theme: &ColorTheme,
    options: &RenderOptions,
    output: &mut String,
) {
    for (index, child) in children.iter().enumerate() {
        let connector = if index + 1 == children.len() {
            "└─ "
        } else {
            "├─ "
        };
        render_node(child, prefix, connector, theme, options, output);
    }
}

fn finish_line(output: &mut String, start: usize, max_width: Option<usize>) {
    if let Some(max_width) = max_width
        && let Cow::Owned(truncated) = truncate_line(&output[start..], max_width)
    {
        output.truncate(start);
        output.push_str(&truncated);
    }
    output.push('\n');
}

fn truncate_line(line: &str, max_width: usize) -> Cow<'_, str> {
    let plain = console::strip_ansi_codes(line);
    if console::measure_text_width(&plain) <= max_width {
        return Cow::Borrowed(line);
    }

    let tail = &".."[..max_width.min(2)];
    let budget = max_width - tail.len();
    let mut columns = 0;
    let mut remaining = 0;
    for grapheme in plain.graphemes(true) {
        let width = console::measure_text_width(grapheme);
        if columns + width > budget {
            break;
        }
        columns += width;
        remaining += grapheme.len();
    }

    let mut truncated = String::new();
    let mut tail_written = false;
    for (part, is_ansi) in console::AnsiCodeIterator::new(line) {
        if is_ansi {
            // Keep closing style codes even when their text has been omitted.
            truncated.push_str(part);
        } else {
            let keep = remaining.min(part.len());
            truncated.push_str(&part[..keep]);
            remaining -= keep;
            if keep < part.len() && !tail_written {
                truncated.push_str(tail);
                tail_written = true;
            }
        }
    }
    Cow::Owned(truncated)
}

fn escape_control_characters(value: &str) -> Cow<'_, str> {
    if !value.chars().any(char::is_control) {
        return Cow::Borrowed(value);
    }

    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() {
            escaped.extend(character.escape_default());
        } else {
            escaped.push(character);
        }
    }
    Cow::Owned(escaped)
}

#[cfg(test)]
mod tests {
    use console::Style;

    use crate::view::{SchemaNodeRole, ViewDocument, ViewNode, ViewNodeRole};

    use super::{ColorTheme, Layout, RenderOptions, render, truncate_line};

    #[test]
    fn escapes_control_characters_in_rendered_content() {
        let document = ViewDocument {
            roots: vec![ViewNode {
                role: ViewNodeRole::Schema(SchemaNodeRole::Instance),
                key: Some("key\n".to_owned()),
                value: "value\u{1b}".to_owned(),
                details: Vec::new(),
                children: Vec::new(),
            }],
        };
        let plain = Style::new();
        let theme = ColorTheme {
            key: plain.clone(),
            value: plain.clone(),
            connector: plain.clone(),
            constraint: plain.clone(),
            annotation: plain.clone(),
            metadata: plain.clone(),
            marker: plain,
        };

        assert_eq!(
            render(&document, &theme, &RenderOptions::default()),
            "key\\n value\\u{1b}\n"
        );
    }

    #[test]
    fn tree_layout_includes_every_top_level_block() {
        let document = ViewDocument {
            roots: ["root", "Definitions", "Another block"]
                .map(|value| ViewNode {
                    role: ViewNodeRole::Section,
                    key: None,
                    value: value.to_owned(),
                    details: Vec::new(),
                    children: Vec::new(),
                })
                .into(),
        };

        let output = render(
            &document,
            &ColorTheme::default(),
            &RenderOptions {
                layout: Layout::Tree,
                ..RenderOptions::default()
            },
        );
        assert_eq!(
            console::strip_ansi_codes(&output),
            "Schema\n├─ root\n├─ Definitions\n└─ Another block\n"
        );
    }

    #[rstest::rstest]
    #[case("abcdef", 6, "abcdef")]
    #[case("abcdef", 5, "abc..")]
    #[case("abcdef", 2, "..")]
    #[case("日本語abc", 6, "日本..")]
    #[case("日本語abc", 5, "日..")]
    #[case("e\u{301}abcdef", 3, "e\u{301}..")]
    #[case("a\u{301}\u{308}bcdef", 3, "a\u{301}\u{308}..")]
    #[case("👨‍👩‍👧‍👦abcdef", 4, "👨‍👩‍👧‍👦..")]
    #[case("👨‍👩‍👧‍👦abcdef", 3, "..")]
    #[case("🇯🇵abcdef", 4, "🇯🇵..")]
    #[case("👍🏽abcdef", 4, "👍🏽..")]
    fn truncates_at_grapheme_boundaries(
        #[case] line: &str,
        #[case] width: usize,
        #[case] expected: &str,
    ) {
        let truncated = truncate_line(line, width);
        assert_eq!(truncated, expected);
        assert!(console::measure_text_width(&truncated) <= width);
    }

    #[test]
    fn preserves_ansi_codes_and_uses_the_same_cut_as_plain_text() {
        let line = "\x1b[36m日本\x1b[0m \x1b[33mstring\x1b[0m \x1b[31m[required]\x1b[0m";

        for width in 2..=25 {
            let truncated = truncate_line(line, width);
            assert_eq!(
                console::strip_ansi_codes(&truncated),
                truncate_line(&console::strip_ansi_codes(line), width)
            );
            assert!(console::measure_text_width(&truncated) <= width);
            assert!(truncated.ends_with("\x1b[0m"));
        }
    }
}
