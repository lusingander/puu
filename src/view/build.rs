use serde_json::Value;

use crate::{
    schema::{
        ArrayConstraints, Reference, ReferenceKind, ReferenceTarget, Schema, SchemaChildRole,
        SchemaKind,
    },
    view::model::{SchemaNodeRole, ViewDetail, ViewDocument, ViewNode, ViewNodeRole, ViewOptions},
};

pub fn from_schema(schema: &Schema, options: &ViewOptions) -> ViewDocument {
    let mut roots = vec![schema_node(schema, "root", false, true, options)];
    let mut definitions = Vec::new();
    collect_definitions(schema, &mut definitions, options);
    if !definitions.is_empty() {
        roots.push(ViewNode {
            role: ViewNodeRole::Section,
            key: None,
            value: "Definitions".to_owned(),
            details: Vec::new(),
            children: definitions,
        });
    }
    ViewDocument { roots }
}

fn collect_definitions(schema: &Schema, output: &mut Vec<ViewNode>, options: &ViewOptions) {
    schema.for_each_child(|role, child| {
        if let SchemaChildRole::Definition(name) = role {
            output.push(definition_node(
                child,
                schema.definition_label(name, child),
                options,
            ));
        }
        collect_definitions(child, output, options);
    });
}

fn definition_node(schema: &Schema, name: &str, options: &ViewOptions) -> ViewNode {
    schema_node_with_role(
        schema,
        name,
        false,
        false,
        SchemaNodeRole::Definition,
        options,
    )
}

pub fn schema_node(
    schema: &Schema,
    name: &str,
    required: bool,
    root: bool,
    options: &ViewOptions,
) -> ViewNode {
    schema_node_with_role(
        schema,
        name,
        required,
        root,
        SchemaNodeRole::Instance,
        options,
    )
}

pub fn pattern_property_node(schema: &Schema, pattern: &str, options: &ViewOptions) -> ViewNode {
    schema_node_with_role(
        schema,
        &format!("<properties matching {pattern:?}>"),
        false,
        false,
        SchemaNodeRole::PatternProperty,
        options,
    )
}

pub fn property_names_node(schema: &Schema, options: &ViewOptions) -> ViewNode {
    schema_node_with_role(
        schema,
        "<property names>",
        false,
        false,
        SchemaNodeRole::PropertyName,
        options,
    )
}

pub fn contains_node(schema: &Schema, array: &ArrayConstraints, options: &ViewOptions) -> ViewNode {
    let mut node = schema_node_with_role(
        schema,
        "contains",
        false,
        false,
        SchemaNodeRole::Contains,
        options,
    );
    let count = match (&array.min_contains, &array.max_contains) {
        (Some(min), Some(max)) => format!("{min}..{max} matches"),
        (Some(min), None) => format!(">= {min} matches"),
        (None, Some(max)) => format!("1..{max} matches"),
        (None, None) => ">= 1 matches".to_owned(),
    };
    node.details.insert(0, ViewDetail::constraint(count));
    node
}

pub fn content_schema_node(
    schema: &Schema,
    has_media_type: bool,
    options: &ViewOptions,
) -> ViewNode {
    let mut node = schema_node_with_role(
        schema,
        "content schema",
        false,
        false,
        SchemaNodeRole::ContentSchema,
        options,
    );
    if !has_media_type {
        node.details
            .insert(0, ViewDetail::marker("ignored without contentMediaType"));
    }
    node
}

pub fn unevaluated_node(
    schema: &Schema,
    name: &str,
    role: SchemaNodeRole,
    evaluation_sources: Vec<&str>,
    contains_note: Option<&str>,
    options: &ViewOptions,
) -> ViewNode {
    let mut node = schema_node_with_role(schema, name, false, false, role, options);
    node.value = match schema.kind {
        SchemaKind::Any => "allowed".to_owned(),
        SchemaKind::Never => "forbidden".to_owned(),
        _ => node.value,
    };
    node.details
        .insert(0, ViewDetail::marker("evaluation-dependent"));
    if !evaluation_sources.is_empty() {
        node.details.insert(
            1,
            ViewDetail::marker(format!("evaluated by: {}", evaluation_sources.join(", "))),
        );
    }
    if let Some(note) = contains_note {
        node.details.push(ViewDetail::marker(note));
    }
    node
}

fn reference_node(reference: &Reference) -> ViewNode {
    let mut details = Vec::new();
    add_reference_details(reference, &mut details);
    ViewNode {
        role: ViewNodeRole::Schema(SchemaNodeRole::ReferenceApplicator),
        key: Some(reference.kind.keyword().to_owned()),
        value: reference_value(reference),
        details,
        children: Vec::new(),
    }
}

pub fn logical_section(keyword: &str, branches: &[Schema], options: &ViewOptions) -> ViewNode {
    ViewNode {
        role: ViewNodeRole::Section,
        key: None,
        value: keyword.to_owned(),
        details: Vec::new(),
        children: branches
            .iter()
            .enumerate()
            .map(|(index, branch)| {
                schema_node_with_role(
                    branch,
                    &format!("[{index}]"),
                    false,
                    false,
                    SchemaNodeRole::LogicalBranch,
                    options,
                )
            })
            .collect(),
    }
}

pub fn not_node(schema: &Schema, options: &ViewOptions) -> ViewNode {
    schema_node_with_role(
        schema,
        "not",
        false,
        false,
        SchemaNodeRole::LogicalBranch,
        options,
    )
}

pub fn dependent_schema_node(
    schema: &Schema,
    property_name: &str,
    options: &ViewOptions,
) -> ViewNode {
    schema_node_with_role(
        schema,
        &format!("when property {property_name:?} exists"),
        false,
        false,
        SchemaNodeRole::DependentSchema,
        options,
    )
}

pub fn conditional_node(
    schema: &Schema,
    keyword: &str,
    role: SchemaNodeRole,
    ignored_without_if: bool,
    options: &ViewOptions,
) -> ViewNode {
    let mut node = schema_node_with_role(schema, keyword, false, false, role, options);
    if ignored_without_if {
        node.details
            .insert(0, ViewDetail::marker("ignored without if"));
    }
    node
}

fn schema_node_with_role(
    schema: &Schema,
    name: &str,
    required: bool,
    root: bool,
    role: SchemaNodeRole,
    options: &ViewOptions,
) -> ViewNode {
    let key = match (&schema.kind, root) {
        (SchemaKind::Any | SchemaKind::Never, true) => None,
        _ => Some(name.to_owned()),
    };
    let primary_reference = primary_reference(schema);
    let value = schema_value(schema, primary_reference);
    let details = crate::view::details::from_schema(schema, required, primary_reference, options);
    let children = crate::view::children::from_schema(schema, options);

    ViewNode {
        role: ViewNodeRole::Schema(role),
        key,
        value,
        details,
        children,
    }
}

fn schema_value(schema: &Schema, primary_reference: Option<&Reference>) -> String {
    if let Some(reference) = primary_reference {
        return reference_value(reference);
    }

    let mut value = match &schema.kind {
        SchemaKind::Any => "any".to_owned(),
        SchemaKind::Never => "never".to_owned(),
        SchemaKind::Typed(kinds) => kinds.join(" | "),
        SchemaKind::Unspecified => "<type unspecified>".to_owned(),
    };
    if matches!(&schema.kind, SchemaKind::Typed(kinds) if kinds == &["array"])
        && let Some(items) = &schema.array_constraints.items
        && schema.array_constraints.prefix_items.is_none()
        && let Some(item_type) = simple_item_type(items)
    {
        value = format!("{item_type}[]");
    }
    if matches!(schema.kind, SchemaKind::Unspecified) {
        if let Some(const_value) = &schema.value_constraints.const_value {
            value = format!("= {const_value}");
        } else if let Some(enum_values) = &schema.value_constraints.enum_values
            && inline_enum(enum_values)
        {
            value = format!(
                "enum {{{}}}",
                enum_values
                    .iter()
                    .map(Value::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    value
}

pub fn inline_enum(values: &[serde_json::Value]) -> bool {
    values.len() <= 3
        && values.iter().all(|value| {
            value.is_null() || value.is_boolean() || value.is_number() || value.is_string()
        })
}

fn primary_reference(schema: &Schema) -> Option<&Reference> {
    schema.references().next()
}

pub fn additional_reference_nodes(schema: &Schema) -> Vec<ViewNode> {
    schema.references().skip(1).map(reference_node).collect()
}

fn reference_value(reference: &Reference) -> String {
    match &reference.target {
        ReferenceTarget::Local {
            location,
            display_name,
            ..
        } => format!(
            "-> {}",
            if display_name.is_empty() {
                location
            } else {
                display_name
            }
        ),
        ReferenceTarget::External => {
            let uri = if reference.kind == ReferenceKind::Static {
                &reference.uri
            } else {
                reference.resolved_uri.as_ref().unwrap_or(&reference.uri)
            };
            format_reference_uri(uri)
        }
        ReferenceTarget::Unresolved => format_reference_uri(&reference.uri),
    }
}

fn format_reference_uri(uri: &str) -> String {
    if uri.is_empty() {
        "-> \"\"".to_owned()
    } else {
        format!("-> {uri}")
    }
}

pub fn add_reference_details(reference: &Reference, details: &mut Vec<ViewDetail>) {
    match &reference.target {
        ReferenceTarget::Local {
            recursive: true, ..
        } => details.push(ViewDetail::marker("recursive")),
        ReferenceTarget::External => details.push(ViewDetail::marker("external")),
        ReferenceTarget::Unresolved => details.push(ViewDetail::marker("unresolved")),
        ReferenceTarget::Local { .. } => {}
    }
    match (reference.kind, reference.dynamic_start) {
        (ReferenceKind::Static, _) => {}
        (ReferenceKind::Recursive, true) => {
            details.push(ViewDetail::marker("recursive ref start"));
        }
        (ReferenceKind::Recursive, false)
            if matches!(&reference.target, ReferenceTarget::Local { .. }) =>
        {
            details.push(ViewDetail::marker("recursive ref: static start"));
        }
        (ReferenceKind::Recursive, false) => details.push(ViewDetail::marker("recursive ref")),
        (ReferenceKind::Dynamic, true) => {
            details.push(ViewDetail::marker("dynamic ref start"));
        }
        (ReferenceKind::Dynamic, false)
            if matches!(&reference.target, ReferenceTarget::Local { .. }) =>
        {
            details.push(ViewDetail::marker("dynamic ref: static start"));
        }
        (ReferenceKind::Dynamic, false) => details.push(ViewDetail::marker("dynamic ref")),
    }
}

pub fn object_evaluation_sources(schema: &Schema) -> Vec<&'static str> {
    let mut sources = Vec::new();
    let object = &schema.object_constraints;
    if !schema.properties.is_empty() {
        sources.push("properties");
    }
    if !object.pattern_properties.is_empty() {
        sources.push("patternProperties");
    }
    if object.additional_properties.is_some() {
        sources.push("additionalProperties");
    }
    if !object.dependent_schemas.is_empty() {
        sources.push("dependentSchemas");
    }
    add_in_place_sources(schema, &mut sources);
    sources
}

pub fn array_evaluation_sources(schema: &Schema) -> Vec<&'static str> {
    let mut sources = Vec::new();
    let array = &schema.array_constraints;
    if array.prefix_items.is_some() {
        sources.push(if schema.dialect.contains_marks_evaluated_items() {
            "prefixItems"
        } else {
            "items tuple"
        });
    }
    if array.items.is_some() {
        sources.push("items");
    }
    if array.contains.is_some() && schema.dialect.contains_marks_evaluated_items() {
        sources.push("contains");
    }
    add_in_place_sources(schema, &mut sources);
    sources
}

fn add_in_place_sources(schema: &Schema, sources: &mut Vec<&'static str>) {
    for reference in schema.references() {
        sources.push(reference.kind.keyword());
    }
    let logical = &schema.logical_applicators;
    if !logical.all_of.is_empty() {
        sources.push("allOf");
    }
    if !logical.any_of.is_empty() {
        sources.push("anyOf");
    }
    if !logical.one_of.is_empty() {
        sources.push("oneOf");
    }
    let conditional = &schema.conditional_applicators;
    if conditional.condition.is_some() {
        sources.push("if");
        if conditional.then_branch.is_some() {
            sources.push("then");
        }
        if conditional.else_branch.is_some() {
            sources.push("else");
        }
    }
}

pub fn add_annotation_details(schema: &Schema, verbose: bool, details: &mut Vec<ViewDetail>) {
    let annotations = &schema.annotations;
    if let Some(title) = &annotations.title {
        details.push(ViewDetail::annotation(format!(
            "title: {}",
            quoted_text(title, if verbose { 160 } else { 60 })
        )));
    }
    if let Some(description) = &annotations.description {
        details.push(ViewDetail::annotation(format!(
            "description: {}",
            quoted_text(description, if verbose { 240 } else { 60 })
        )));
    }
    if let Some(default_value) = &annotations.default_value {
        details.push(ViewDetail::annotation(format!(
            "default: {}",
            compact_json(default_value, if verbose { 240 } else { 80 })
        )));
    }
    if let Some(examples) = &annotations.examples {
        let text = if verbose {
            let serialized =
                serde_json::to_string(examples).expect("JSON values should always serialize");
            truncate(&serialized, 240)
        } else {
            format!("{} values", examples.len())
        };
        details.push(ViewDetail::annotation(format!("examples: {text}")));
    }
    add_boolean_annotation(details, "deprecated", annotations.deprecated, verbose);
    add_boolean_annotation(details, "read only", annotations.read_only, verbose);
    add_boolean_annotation(details, "write only", annotations.write_only, verbose);
    if let Some(format) = &annotations.format {
        details.push(ViewDetail::annotation(format!("format: {format}")));
    }
    if let Some(encoding) = &annotations.content_encoding {
        details.push(ViewDetail::annotation(format!(
            "content encoding: {}",
            compact_text(encoding, if verbose { 160 } else { 60 })
        )));
    }
    if let Some(media_type) = &annotations.content_media_type {
        details.push(ViewDetail::annotation(format!(
            "content media type: {}",
            compact_text(media_type, if verbose { 160 } else { 60 })
        )));
    }
    if verbose {
        if let Some(comment) = &annotations.comment {
            details.push(ViewDetail::annotation(format!(
                "$comment: {}",
                quoted_text(comment, 240)
            )));
        }
        if !annotations.vocabulary.is_empty() {
            let vocabulary = annotations
                .vocabulary
                .iter()
                .map(|(uri, required)| {
                    format!(
                        "{uri} ({})",
                        if *required { "required" } else { "optional" }
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            details.push(ViewDetail::metadata(format!(
                "$vocabulary: {}",
                compact_text(&vocabulary, 240)
            )));
        }
    }
}

fn add_boolean_annotation(
    details: &mut Vec<ViewDetail>,
    label: &str,
    value: Option<bool>,
    verbose: bool,
) {
    match value {
        Some(true) => details.push(ViewDetail::annotation(label)),
        Some(false) if verbose => {
            details.push(ViewDetail::annotation(format!("{label}: false")));
        }
        Some(false) | None => {}
    }
}

fn quoted_text(value: &str, max_chars: usize) -> String {
    format!("{:?}", compact_text(value, max_chars))
}

fn compact_text(value: &str, max_chars: usize) -> String {
    truncate(
        &value.split_whitespace().collect::<Vec<_>>().join(" "),
        max_chars,
    )
}

pub fn compact_json(value: &Value, max_chars: usize) -> String {
    truncate(&value.to_string(), max_chars)
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    let mut truncated = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

pub fn simple_item_type(schema: &Schema) -> Option<&str> {
    let SchemaKind::Typed(kinds) = &schema.kind else {
        return None;
    };
    let [kind] = kinds.as_slice() else {
        return None;
    };
    if !matches!(
        kind.as_str(),
        "string" | "number" | "integer" | "boolean" | "null"
    ) {
        return None;
    }
    let object = &schema.object_constraints;
    let array = &schema.array_constraints;
    let string = &schema.string_constraints;
    let number = &schema.number_constraints;
    let values = &schema.value_constraints;
    if !schema.properties.is_empty()
        || !schema.definitions.is_empty()
        || schema.references().next().is_some()
        || !schema.required_names.is_empty()
        || object.min_properties.is_some()
        || object.max_properties.is_some()
        || !object.pattern_properties.is_empty()
        || object.property_names.is_some()
        || !object.dependent_required.is_empty()
        || !object.dependent_schemas.is_empty()
        || object.additional_properties.is_some()
        || object.unevaluated_properties.is_some()
        || array.items.is_some()
        || array.prefix_items.is_some()
        || array.contains.is_some()
        || array.min_contains.is_some()
        || array.max_contains.is_some()
        || array.min_items.is_some()
        || array.max_items.is_some()
        || array.unique_items.is_some()
        || array.unevaluated_items.is_some()
        || !schema.logical_applicators.all_of.is_empty()
        || !schema.logical_applicators.any_of.is_empty()
        || !schema.logical_applicators.one_of.is_empty()
        || schema.logical_applicators.not.is_some()
        || schema.conditional_applicators.condition.is_some()
        || schema.conditional_applicators.then_branch.is_some()
        || schema.conditional_applicators.else_branch.is_some()
        || string.min_length.is_some()
        || string.max_length.is_some()
        || string.pattern.is_some()
        || number.minimum.is_some()
        || number.maximum.is_some()
        || number.exclusive_minimum.is_some()
        || number.exclusive_maximum.is_some()
        || number.multiple_of.is_some()
        || values.const_value.is_some()
        || values.enum_values.is_some()
        || !schema.annotations.is_empty()
        || !schema.uninterpreted_keywords.is_empty()
        || !schema.ignored_keywords.is_empty()
    {
        return None;
    }
    Some(kind)
}
