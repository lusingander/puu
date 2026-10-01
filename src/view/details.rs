use serde_json::Value;

use crate::{
    schema::{Reference, Schema, SchemaKind},
    view::{
        build::{
            add_annotation_details, add_metadata_details, add_reference_details, compact_json,
            inline_enum,
        },
        model::{ViewDetail, ViewOptions},
    },
};

pub fn from_schema(
    schema: &Schema,
    required: bool,
    primary_reference: Option<&Reference>,
    options: &ViewOptions,
) -> Vec<ViewDetail> {
    let mut details = Vec::new();
    add_identity(schema, required, primary_reference, &mut details);
    add_object_constraints(schema, &mut details);
    add_array_constraints(schema, &mut details);
    add_string_constraints(schema, &mut details);
    if options.annotations {
        add_annotation_details(schema, options.verbose, &mut details);
    }
    add_metadata_details(schema, options.verbose, &mut details);
    add_number_constraints(schema, &mut details);
    add_value_constraints(schema, primary_reference.is_some(), &mut details);
    add_keyword_markers(schema, options.verbose, &mut details);
    details
}

fn add_identity(
    schema: &Schema,
    required: bool,
    primary_reference: Option<&Reference>,
    details: &mut Vec<ViewDetail>,
) {
    if required {
        details.push(ViewDetail::marker("required"));
    }
    if let Some(uri) = &schema.overridden_dialect {
        details.push(ViewDetail::marker(format!(
            "$schema overridden: {uri} -> {}",
            schema.dialect.name()
        )));
    }
    if schema.identity.identifier.is_some() && schema.identity.is_resource_root {
        details.push(ViewDetail::marker(format!(
            "resource: {}",
            schema.identity.resource_uri
        )));
    }
    if let Some(anchor) = &schema.identity.anchor {
        details.push(ViewDetail::marker(format!("anchor: {anchor}")));
    }
    if schema.identity.recursive_anchor == Some(true) {
        details.push(ViewDetail::marker("recursive anchor"));
    }
    if let Some(anchor) = &schema.identity.dynamic_anchor {
        details.push(ViewDetail::marker(format!("dynamic anchor: {anchor}")));
    }
    if let Some(reference) = primary_reference {
        add_reference_details(reference, details);
        if let SchemaKind::Typed(kinds) = &schema.kind {
            details.push(ViewDetail::constraint(format!(
                "type: {}",
                kinds.join(" | ")
            )));
        }
    }
}

fn add_object_constraints(schema: &Schema, details: &mut Vec<ViewDetail>) {
    let unmatched_required: Vec<_> = schema
        .required_names
        .iter()
        .filter(|required_name| {
            !schema
                .properties
                .iter()
                .any(|(property_name, _)| property_name == *required_name)
        })
        .map(|name| format!("{name:?}"))
        .collect();
    if !unmatched_required.is_empty() {
        details.push(ViewDetail::constraint(format!(
            "required names: {}",
            unmatched_required.join(", ")
        )));
    }

    let object = &schema.object_constraints;
    match (&object.min_properties, &object.max_properties) {
        (Some(min), Some(max)) => {
            details.push(ViewDetail::constraint(format!("{min}..{max} properties")));
        }
        (Some(min), None) => details.push(ViewDetail::constraint(format!(">= {min} properties"))),
        (None, Some(max)) => details.push(ViewDetail::constraint(format!("<= {max} properties"))),
        (None, None) => {}
    }
    if let Some(additional) = &object.additional_properties
        && matches!(additional.kind, SchemaKind::Never)
    {
        let only_object = matches!(&schema.kind, SchemaKind::Typed(kinds) if kinds == &["object"]);
        details.push(ViewDetail::constraint(if only_object {
            "closed"
        } else {
            "additional properties: forbidden"
        }));
    }
    for (name, required_names) in &object.dependent_required {
        let required_names = if required_names.is_empty() {
            "nothing".to_owned()
        } else {
            required_names
                .iter()
                .map(|name| format!("{name:?}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        details.push(ViewDetail::constraint(format!(
            "if {name:?} exists, requires {required_names}"
        )));
    }
}

fn add_array_constraints(schema: &Schema, details: &mut Vec<ViewDetail>) {
    let array = &schema.array_constraints;
    match (&array.min_items, &array.max_items) {
        (Some(min), Some(max)) => {
            details.push(ViewDetail::constraint(format!("{min}..{max} items")))
        }
        (Some(min), None) => details.push(ViewDetail::constraint(format!(">= {min} items"))),
        (None, Some(max)) => details.push(ViewDetail::constraint(format!("<= {max} items"))),
        (None, None) => {}
    }
    if array.unique_items == Some(true) {
        details.push(ViewDetail::constraint("unique"));
    }
    if let Some(items) = &array.items
        && matches!(items.kind, SchemaKind::Never)
        && array.prefix_items.is_none()
    {
        details.push(ViewDetail::constraint("items: forbidden"));
    }
}

fn add_string_constraints(schema: &Schema, details: &mut Vec<ViewDetail>) {
    match (
        &schema.string_constraints.min_length,
        &schema.string_constraints.max_length,
    ) {
        (Some(min), Some(max)) => {
            details.push(ViewDetail::constraint(format!("{min}..{max} chars")))
        }
        (Some(min), None) => details.push(ViewDetail::constraint(format!(">= {min} chars"))),
        (None, Some(max)) => details.push(ViewDetail::constraint(format!("<= {max} chars"))),
        (None, None) => {}
    }
    if let Some(pattern) = &schema.string_constraints.pattern
        && pattern.chars().count() <= 48
    {
        details.push(ViewDetail::constraint(format!("pattern: {pattern:?}")));
    }
}

fn add_number_constraints(schema: &Schema, details: &mut Vec<ViewDetail>) {
    let number = &schema.number_constraints;
    if let Some(value) = &number.minimum {
        details.push(ViewDetail::constraint(format!(">= {value}")));
    }
    if let Some(value) = &number.exclusive_minimum {
        details.push(ViewDetail::constraint(format!("> {value}")));
    }
    if let Some(value) = &number.maximum {
        details.push(ViewDetail::constraint(format!("<= {value}")));
    }
    if let Some(value) = &number.exclusive_maximum {
        details.push(ViewDetail::constraint(format!("< {value}")));
    }
    if let Some(value) = &number.multiple_of {
        details.push(ViewDetail::constraint(format!("multiple of {value}")));
    }
}

fn add_value_constraints(
    schema: &Schema,
    has_primary_reference: bool,
    details: &mut Vec<ViewDetail>,
) {
    if let Some(const_value) = &schema.value_constraints.const_value
        && (!matches!(schema.kind, SchemaKind::Unspecified) || has_primary_reference)
    {
        details.push(ViewDetail::constraint(format!("const: {const_value}")));
    }
    if let Some(enum_values) = &schema.value_constraints.enum_values {
        if inline_enum(enum_values) {
            if !matches!(schema.kind, SchemaKind::Unspecified)
                || has_primary_reference
                || schema.value_constraints.const_value.is_some()
            {
                details.push(ViewDetail::constraint(format!(
                    "enum: {}",
                    enum_values
                        .iter()
                        .map(Value::to_string)
                        .collect::<Vec<_>>()
                        .join(" | ")
                )));
            }
        } else {
            details.push(ViewDetail::constraint(format!(
                "enum: {} values",
                enum_values.len()
            )));
        }
    }
}

fn add_keyword_markers(schema: &Schema, verbose: bool, details: &mut Vec<ViewDetail>) {
    if !schema.uninterpreted_keywords.is_empty() {
        details.push(ViewDetail::marker(format!(
            "uninterpreted: {}",
            schema
                .uninterpreted_keywords
                .iter()
                .map(|(keyword, value)| if verbose {
                    format!("{keyword}={}", compact_json(value, 160))
                } else {
                    keyword.clone()
                })
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    if !schema.ignored_keywords.is_empty() {
        let reason = if !schema.dialect.allows_ref_siblings() && schema.reference.is_some() {
            "ignored next to $ref"
        } else {
            "ignored"
        };
        details.push(ViewDetail::marker(format!(
            "{reason}: {}",
            schema
                .ignored_keywords
                .iter()
                .map(|(keyword, _)| keyword.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
}
