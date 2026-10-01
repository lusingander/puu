use crate::{
    schema::{Schema, SchemaKind},
    view::{
        build::{
            additional_reference_nodes, array_evaluation_sources, conditional_node,
            constraint_node, contains_node, content_schema_node, dependent_schema_node,
            logical_section, not_node, object_evaluation_sources, pattern_property_node,
            property_names_node, schema_node, simple_item_type, unevaluated_node,
        },
        context::BuildContext,
        model::{SchemaNodeRole, ViewNode},
    },
};

pub fn from_schema(schema: &Schema, context: &mut BuildContext<'_, '_>) -> Vec<ViewNode> {
    let mut children = additional_reference_nodes(schema, context);
    add_object_children(schema, context, &mut children);
    add_array_children(schema, context, &mut children);
    add_supplemental_children(schema, context, &mut children);
    add_applicator_children(schema, context, &mut children);
    children
}

fn add_object_children(
    schema: &Schema,
    context: &mut BuildContext<'_, '_>,
    children: &mut Vec<ViewNode>,
) {
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
                    context,
                )
            }),
    );
    let object = &schema.object_constraints;
    children.extend(
        object
            .pattern_properties
            .iter()
            .map(|(pattern, schema)| pattern_property_node(schema, pattern, context)),
    );
    if let Some(property_names) = &object.property_names {
        children.push(property_names_node(property_names, context));
    }
    if let Some(additional) = &object.additional_properties
        && !matches!(additional.kind, SchemaKind::Any | SchemaKind::Never)
    {
        children.push(schema_node(
            additional,
            "<other properties>",
            false,
            false,
            context,
        ));
    }
    children.extend(
        object
            .dependent_schemas
            .iter()
            .map(|(name, schema)| dependent_schema_node(schema, name, context)),
    );
    if let Some(unevaluated) = &object.unevaluated_properties {
        children.push(unevaluated_node(
            unevaluated,
            "unevaluated properties",
            SchemaNodeRole::UnevaluatedProperties,
            object_evaluation_sources(schema),
            None,
            context,
        ));
    }
}

fn add_array_children(
    schema: &Schema,
    context: &mut BuildContext<'_, '_>,
    children: &mut Vec<ViewNode>,
) {
    let array = &schema.array_constraints;
    if let Some(prefix_items) = &array.prefix_items {
        for (index, item) in prefix_items.iter().enumerate() {
            children.push(schema_node(
                item,
                &format!("[{index}]"),
                false,
                false,
                context,
            ));
        }
    }
    if let Some(items) = &array.items {
        match (&array.prefix_items, &items.kind) {
            (Some(_), SchemaKind::Any) => {}
            (Some(_), SchemaKind::Never) => children.push(constraint_node(
                "additional items",
                "forbidden".to_owned(),
                context,
            )),
            (Some(prefix_items), _) => children.push(schema_node(
                items,
                &format!("[{}..]", prefix_items.len()),
                false,
                false,
                context,
            )),
            (None, SchemaKind::Any | SchemaKind::Never) => {}
            (None, _)
                if matches!(&schema.kind, SchemaKind::Typed(kinds) if kinds == &["array"])
                    && simple_item_type(items).is_some() => {}
            (None, _) => children.push(schema_node(items, "items", false, false, context)),
        }
    }
    if let Some(contains) = &array.contains {
        children.push(contains_node(contains, array, context));
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
            context,
        ));
    }
}

fn add_supplemental_children(
    schema: &Schema,
    context: &mut BuildContext<'_, '_>,
    children: &mut Vec<ViewNode>,
) {
    if let Some(content_schema) = &schema.annotations.content_schema {
        children.push(content_schema_node(
            content_schema,
            schema.annotations.content_media_type.is_some(),
            context,
        ));
    }
    if let Some(pattern) = &schema.string_constraints.pattern
        && pattern.chars().count() > 48
    {
        children.push(constraint_node("pattern", format!("{pattern:?}"), context));
    }
}

fn add_applicator_children(
    schema: &Schema,
    context: &mut BuildContext<'_, '_>,
    children: &mut Vec<ViewNode>,
) {
    let logical = &schema.logical_applicators;
    if !logical.all_of.is_empty() {
        children.push(logical_section("allOf", &logical.all_of, context));
    }
    if !logical.any_of.is_empty() {
        children.push(logical_section("anyOf", &logical.any_of, context));
    }
    if !logical.one_of.is_empty() {
        children.push(logical_section("oneOf", &logical.one_of, context));
    }
    if let Some(not) = &logical.not {
        children.push(not_node(not, context));
    }
    let conditional = &schema.conditional_applicators;
    if let Some(condition) = &conditional.condition {
        children.push(conditional_node(
            condition,
            "if",
            SchemaNodeRole::Condition,
            false,
            context,
        ));
    }
    let ignored_without_if = conditional.condition.is_none();
    if let Some(then_branch) = &conditional.then_branch {
        children.push(conditional_node(
            then_branch,
            "then",
            SchemaNodeRole::ThenBranch,
            ignored_without_if,
            context,
        ));
    }
    if let Some(else_branch) = &conditional.else_branch {
        children.push(conditional_node(
            else_branch,
            "else",
            SchemaNodeRole::ElseBranch,
            ignored_without_if,
            context,
        ));
    }
}
