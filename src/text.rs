use std::{borrow::Cow, fmt::Write};

use console::Style;

use crate::view::{SchemaNodeRole, ViewDetailKind, ViewDocument, ViewNode, ViewNodeRole};

pub struct ColorTheme {
    pub key: Style,
    pub value: Style,
    pub connector: Style,
    pub constraint: Style,
    pub annotation: Style,
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
            marker: Style::new().red(),
        }
    }
}

pub fn render(document: &ViewDocument, theme: &ColorTheme) -> String {
    let mut output = String::new();
    for (index, root) in document.roots.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        render_node(root, "", "", theme, &mut output);
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
            | SchemaNodeRole::LogicalBranch
            | SchemaNodeRole::DependentSchema
            | SchemaNodeRole::Condition
            | SchemaNodeRole::ThenBranch
            | SchemaNodeRole::ElseBranch,
        ) => (&theme.key, &theme.value),
        ViewNodeRole::Constraint => (&theme.constraint, &theme.constraint),
        ViewNodeRole::Section => (&theme.value, &theme.value),
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
    for (index, child) in node.children.iter().enumerate() {
        let connector = if index + 1 == node.children.len() {
            "└─ "
        } else {
            "├─ "
        };
        render_node(child, &child_prefix, connector, theme, output);
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

    use super::{ColorTheme, render};

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
            marker: plain,
        };

        assert_eq!(render(&document, &theme), "key\\n value\\u{1b}\n");
    }
}
