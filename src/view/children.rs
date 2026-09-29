use crate::{
    schema::{Schema, SchemaKind},
    view::{
        build::{
            additional_reference_nodes, array_evaluation_sources, conditional_node, contains_node,
            content_schema_node, dependent_schema_node, logical_section, not_node,
            object_evaluation_sources, pattern_property_node, property_names_node, schema_node,
            simple_item_type, unevaluated_node,
        },
        model::{SchemaNodeRole, ViewNode, ViewNodeRole, ViewOptions},
    },
};

pub fn from_schema(schema: &Schema, options: &ViewOptions) -> Vec<ViewNode> {
    let mut children = additional_reference_nodes(schema);
    add_object_children(schema, options, &mut children);
    add_array_children(schema, options, &mut children);
    add_supplemental_children(schema, options, &mut children);
    add_applicator_children(schema, options, &mut children);
    children
}

fn add_object_children(schema: &Schema, options: &ViewOptions, children: &mut Vec<ViewNode>) {
    children.extend(
        schema
            .properties
            .iter()
            .map(|(property_name, property_schema)| {
                let display_name = if property_name.chars().any(char::is_control) {
                    format!("{property_name:?}")
                } else {
                    property_name.clone()
                };
                schema_node(
                    property_schema,
                    &display_name,
                    schema.required_names.contains(property_name),
                    false,
                    options,
                )
            }),
    );
    let object = &schema.object_constraints;
    children.extend(
        object
            .pattern_properties
            .iter()
            .map(|(pattern, schema)| pattern_property_node(schema, pattern, options)),
    );
    if let Some(property_names) = &object.property_names {
        children.push(property_names_node(property_names, options));
    }
    if let Some(additional) = &object.additional_properties
        && !matches!(additional.kind, SchemaKind::Any | SchemaKind::Never)
    {
        children.push(schema_node(
            additional,
            "<other properties>",
            false,
            false,
            options,
        ));
    }
    children.extend(
        object
            .dependent_schemas
            .iter()
            .map(|(name, schema)| dependent_schema_node(schema, name, options)),
    );
    if let Some(unevaluated) = &object.unevaluated_properties {
        children.push(unevaluated_node(
            unevaluated,
            "unevaluated properties",
            SchemaNodeRole::UnevaluatedProperties,
            object_evaluation_sources(schema),
            None,
            options,
        ));
    }
}

fn add_array_children(schema: &Schema, options: &ViewOptions, children: &mut Vec<ViewNode>) {
    let array = &schema.array_constraints;
    if let Some(prefix_items) = &array.prefix_items {
        for (index, item) in prefix_items.iter().enumerate() {
            children.push(schema_node(
                item,
                &format!("[{index}]"),
                false,
                false,
                options,
            ));
        }
    }
    if let Some(items) = &array.items {
        match (&array.prefix_items, &items.kind) {
            (Some(_), SchemaKind::Any) => {}
            (Some(_), SchemaKind::Never) => children.push(ViewNode {
                role: ViewNodeRole::Constraint,
                key: Some("additional items".to_owned()),
                value: "forbidden".to_owned(),
                details: Vec::new(),
                children: Vec::new(),
            }),
            (Some(prefix_items), _) => children.push(schema_node(
                items,
                &format!("[{}..]", prefix_items.len()),
                false,
                false,
                options,
            )),
            (None, SchemaKind::Any | SchemaKind::Never) => {}
            (None, _)
                if matches!(&schema.kind, SchemaKind::Typed(kinds) if kinds == &["array"])
                    && simple_item_type(items).is_some() => {}
            (None, _) => children.push(schema_node(items, "items", false, false, options)),
        }
    }
    if let Some(contains) = &array.contains {
        children.push(contains_node(contains, array, options));
    }
    if let Some(unevaluated) = &array.unevaluated_items {
        let contains_note = array.contains.as_ref().map(|_| {
            if schema.dialect.contains_marks_evaluated_items() {
                "contains marks matching items evaluated"
            } else {
                "contains does not mark items evaluated"
            }
        });
        children.push(unevaluated_node(
            unevaluated,
            "unevaluated items",
            SchemaNodeRole::UnevaluatedItems,
            array_evaluation_sources(schema),
            contains_note,
            options,
        ));
    }
}

fn add_supplemental_children(schema: &Schema, options: &ViewOptions, children: &mut Vec<ViewNode>) {
    if let Some(content_schema) = &schema.annotations.content_schema {
        children.push(content_schema_node(
            content_schema,
            schema.annotations.content_media_type.is_some(),
            options,
        ));
    }
    if let Some(pattern) = &schema.string_constraints.pattern
        && pattern.chars().count() > 48
    {
        children.push(ViewNode {
            role: ViewNodeRole::Constraint,
            key: Some("pattern".to_owned()),
            value: format!("{pattern:?}"),
            details: Vec::new(),
            children: Vec::new(),
        });
    }
}

fn add_applicator_children(schema: &Schema, options: &ViewOptions, children: &mut Vec<ViewNode>) {
    let logical = &schema.logical_applicators;
    if !logical.all_of.is_empty() {
        children.push(logical_section("allOf", &logical.all_of, options));
    }
    if !logical.any_of.is_empty() {
        children.push(logical_section("anyOf", &logical.any_of, options));
    }
    if !logical.one_of.is_empty() {
        children.push(logical_section("oneOf", &logical.one_of, options));
    }
    if let Some(not) = &logical.not {
        children.push(not_node(not, options));
    }
    let conditional = &schema.conditional_applicators;
    if let Some(condition) = &conditional.condition {
        children.push(conditional_node(
            condition,
            "if",
            SchemaNodeRole::Condition,
            false,
            options,
        ));
    }
    let ignored_without_if = conditional.condition.is_none();
    if let Some(then_branch) = &conditional.then_branch {
        children.push(conditional_node(
            then_branch,
            "then",
            SchemaNodeRole::ThenBranch,
            ignored_without_if,
            options,
        ));
    }
    if let Some(else_branch) = &conditional.else_branch {
        children.push(conditional_node(
            else_branch,
            "else",
            SchemaNodeRole::ElseBranch,
            ignored_without_if,
            options,
        ));
    }
}
