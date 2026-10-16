use std::{borrow::Cow, fmt::Write};

use console::Style;

use crate::view::{SchemaNodeRole, ViewDetailKind, ViewDocument, ViewNode, ViewNodeRole};

#[derive(clap::ValueEnum, Clone, Copy, Debug, Eq, PartialEq)]
pub enum Layout {
    Sections,
    Tree,
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

pub fn render(document: &ViewDocument, theme: &ColorTheme, layout: Layout) -> String {
    let mut output = String::new();
    match layout {
        Layout::Sections => {
            for (index, root) in document.roots.iter().enumerate() {
                if index > 0 {
                    output.push('\n');
                }
                render_node(root, "", "", theme, &mut output);
            }
        }
        Layout::Tree => {
            writeln!(output, "{}", theme.value.apply_to("Schema")).expect("writing to String");
            render_children(&document.roots, "", theme, &mut output);
        }
    }
    output
}

fn render_node(
    node: &ViewNode,
    prefix: &str,
    connector: &str,
    theme: &ColorTheme,
    output: &mut String,
) {
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
    output.push('\n');

    let child_prefix = if connector == "├─ " {
        format!("{prefix}│  ")
    } else if connector == "└─ " {
        format!("{prefix}   ")
    } else {
        prefix.to_owned()
    };
    render_children(&node.children, &child_prefix, theme, output);
}

fn render_children(children: &[ViewNode], prefix: &str, theme: &ColorTheme, output: &mut String) {
    for (index, child) in children.iter().enumerate() {
        let connector = if index + 1 == children.len() {
            "└─ "
        } else {
            "├─ "
        };
        render_node(child, prefix, connector, theme, output);
    }
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

    use super::{ColorTheme, Layout, render};

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
            render(&document, &theme, Layout::Sections),
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

        let output = render(&document, &ColorTheme::default(), Layout::Tree);
        assert_eq!(
            console::strip_ansi_codes(&output),
            "Schema\n├─ root\n├─ Definitions\n└─ Another block\n"
        );
    }
}
