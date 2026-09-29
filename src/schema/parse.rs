use std::collections::HashSet;

use serde_json::{Map, Number, Value};

use crate::{
    dialect::{ArrayItemsSyntax, Dialect, DynamicReferenceSyntax, ObjectDependenciesSyntax},
    schema::{
        error::SchemaError,
        model::{
            Annotations, ArrayConstraints, ConditionalApplicators, LogicalApplicators,
            NumberConstraints, ObjectConstraints, ParsedDocument, Reference, ReferenceKind,
            ReferenceTarget, Schema, SchemaIdentity, SchemaKind, StringConstraints,
            ValueConstraints,
        },
        resource,
    },
};

struct ParsedObjectDependencies {
    dependent_required: Vec<(String, Vec<String>)>,
    dependent_schemas: Vec<(String, Schema)>,
}

struct ParsedArrayItems {
    items: Option<Box<Schema>>,
    prefix_items: Option<Vec<Schema>>,
}

#[derive(Clone, Copy)]
struct ParseContext {
    inherited_dialect: Dialect,
    forced_dialect: Option<Dialect>,
}

impl ParseContext {
    const fn root(forced_dialect: Option<Dialect>) -> Self {
        Self {
            inherited_dialect: Dialect::latest_supported(),
            forced_dialect,
        }
    }

    const fn effective_dialect(self) -> Dialect {
        match self.forced_dialect {
            Some(dialect) => dialect,
            None => self.inherited_dialect,
        }
    }

    const fn with_inherited_dialect(self, inherited_dialect: Dialect) -> Self {
        Self {
            inherited_dialect,
            ..self
        }
    }
}

pub fn parse(input: &str, forced_dialect: Option<Dialect>) -> Result<ParsedDocument, SchemaError> {
    let value: Value = serde_json::from_str(input).map_err(SchemaError::InvalidJson)?;
    let value_locations = collect_value_locations(&value);
    let mut schema = parse_value(value, "#", ParseContext::root(forced_dialect))?;
    resource::prepare(&mut schema)?;
    Ok(ParsedDocument {
        root: schema,
        value_locations,
    })
}

fn collect_value_locations(value: &Value) -> HashSet<String> {
    fn collect(value: &Value, location: String, locations: &mut HashSet<String>) {
        locations.insert(location.clone());
        match value {
            Value::Object(entries) => {
                for (name, child) in entries {
                    collect(child, append_location(&location, name), locations);
                }
            }
            Value::Array(values) => {
                for (index, child) in values.iter().enumerate() {
                    collect(child, format!("{location}/{index}"), locations);
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }

    let mut locations = HashSet::new();
    collect(value, "#".to_owned(), &mut locations);
    locations
}

fn parse_value(value: Value, location: &str, context: ParseContext) -> Result<Schema, SchemaError> {
    parse_value_at(value, location, context).map_err(|error| error.at(location))
}

fn parse_value_at(
    value: Value,
    location: &str,
    context: ParseContext,
) -> Result<Schema, SchemaError> {
    match value {
        Value::Bool(true) => Ok(Schema {
            location: location.to_owned(),
            identity: SchemaIdentity::default(),
            dialect: context.effective_dialect(),
            overridden_dialect: None,
            kind: SchemaKind::Any,
            reference: None,
            recursive_reference: None,
            dynamic_reference: None,
            definitions: Vec::new(),
            properties: Vec::new(),
            required_names: Vec::new(),
            object_constraints: ObjectConstraints::default(),
            array_constraints: ArrayConstraints::default(),
            logical_applicators: LogicalApplicators::default(),
            conditional_applicators: ConditionalApplicators::default(),
            string_constraints: StringConstraints::default(),
            number_constraints: NumberConstraints::default(),
            value_constraints: ValueConstraints::default(),
            annotations: Annotations::default(),
            ignored_keywords: Vec::new(),
            uninterpreted_keywords: Vec::new(),
        }),
        Value::Bool(false) => Ok(Schema {
            location: location.to_owned(),
            identity: SchemaIdentity::default(),
            dialect: context.effective_dialect(),
            overridden_dialect: None,
            kind: SchemaKind::Never,
            reference: None,
            recursive_reference: None,
            dynamic_reference: None,
            definitions: Vec::new(),
            properties: Vec::new(),
            required_names: Vec::new(),
            object_constraints: ObjectConstraints::default(),
            array_constraints: ArrayConstraints::default(),
            logical_applicators: LogicalApplicators::default(),
            conditional_applicators: ConditionalApplicators::default(),
            string_constraints: StringConstraints::default(),
            number_constraints: NumberConstraints::default(),
            value_constraints: ValueConstraints::default(),
            annotations: Annotations::default(),
            ignored_keywords: Vec::new(),
            uninterpreted_keywords: Vec::new(),
        }),
        Value::Object(mut object) => {
            let (dialect, overridden_dialect) = select_dialect(&mut object, location, context)?;
            let child_context = context.with_inherited_dialect(dialect);
            let identifier = take_string(&mut object, "$id")?;
            let anchor = if dialect.supports_anchor() {
                take_string(&mut object, "$anchor")?
            } else {
                None
            };
            let (recursive_anchor, dynamic_anchor) = match dialect.dynamic_reference_syntax() {
                DynamicReferenceSyntax::Recursive => {
                    (take_bool(&mut object, "$recursiveAnchor")?, None)
                }
                DynamicReferenceSyntax::Dynamic => {
                    (None, take_string(&mut object, "$dynamicAnchor")?)
                }
                DynamicReferenceSyntax::None => (None, None),
            };
            let reference = take_string(&mut object, "$ref")?.map(|uri| Reference {
                kind: ReferenceKind::Static,
                uri,
                resolved_uri: None,
                target: ReferenceTarget::Unresolved,
                dynamic_start: false,
            });
            let recursive_reference =
                if dialect.dynamic_reference_syntax() == DynamicReferenceSyntax::Recursive {
                    take_string(&mut object, "$recursiveRef")?
                        .map(|uri| {
                            if uri != "#" {
                                return Err(SchemaError::InvalidKeywordValue("$recursiveRef"));
                            }
                            Ok(Reference {
                                kind: ReferenceKind::Recursive,
                                uri,
                                resolved_uri: None,
                                target: ReferenceTarget::Unresolved,
                                dynamic_start: false,
                            })
                        })
                        .transpose()?
                } else {
                    None
                };
            let dynamic_reference =
                if dialect.dynamic_reference_syntax() == DynamicReferenceSyntax::Dynamic {
                    take_string(&mut object, "$dynamicRef")?.map(|uri| Reference {
                        kind: ReferenceKind::Dynamic,
                        uri,
                        resolved_uri: None,
                        target: ReferenceTarget::Unresolved,
                        dynamic_start: false,
                    })
                } else {
                    None
                };
            let definitions = parse_definitions(&mut object, location, child_context)?;
            if !dialect.allows_ref_siblings() && reference.is_some() {
                return Ok(Schema {
                    location: location.to_owned(),
                    identity: SchemaIdentity {
                        identifier,
                        anchor,
                        recursive_anchor,
                        dynamic_anchor,
                        ..SchemaIdentity::default()
                    },
                    dialect,
                    overridden_dialect,
                    kind: SchemaKind::Unspecified,
                    reference,
                    recursive_reference,
                    dynamic_reference,
                    definitions,
                    properties: Vec::new(),
                    required_names: Vec::new(),
                    object_constraints: ObjectConstraints::default(),
                    array_constraints: ArrayConstraints::default(),
                    logical_applicators: LogicalApplicators::default(),
                    conditional_applicators: ConditionalApplicators::default(),
                    string_constraints: StringConstraints::default(),
                    number_constraints: NumberConstraints::default(),
                    value_constraints: ValueConstraints::default(),
                    annotations: Annotations::default(),
                    ignored_keywords: object.into_iter().collect(),
                    uninterpreted_keywords: Vec::new(),
                });
            }
            let kind = match object.shift_remove("type") {
                Some(Value::String(kind)) => {
                    if !valid_type(&kind) {
                        return Err(SchemaError::InvalidType(kind));
                    }
                    SchemaKind::Typed(vec![kind])
                }
                Some(Value::Array(types)) => {
                    let valid = !types.is_empty()
                        && types.iter().enumerate().all(|(index, value)| {
                            let Some(kind) = value.as_str() else {
                                return false;
                            };
                            valid_type(kind)
                                && !types[..index]
                                    .iter()
                                    .any(|previous| previous.as_str() == Some(kind))
                        });
                    if !valid {
                        return Err(SchemaError::InvalidType(Value::Array(types).to_string()));
                    }
                    let kinds = types
                        .into_iter()
                        .map(|value| {
                            let Value::String(kind) = value else {
                                unreachable!("schema types were validated as strings");
                            };
                            kind
                        })
                        .collect();
                    SchemaKind::Typed(kinds)
                }
                Some(other) => return Err(SchemaError::InvalidType(other.to_string())),
                None => SchemaKind::Unspecified,
            };
            let mut ignored_keywords = Vec::new();
            let mut uninterpreted_keywords = Vec::new();

            let properties = match object.shift_remove("properties") {
                Some(Value::Object(properties)) => properties
                    .into_iter()
                    .map(|(name, value)| {
                        let pointer = child_location(location, "properties", &name);
                        let schema =
                            parse_value(value, &pointer, child_context).map_err(|error| {
                                if error.is_invalid_root() {
                                    SchemaError::InvalidPropertySchema(name.clone()).at(&pointer)
                                } else {
                                    error
                                }
                            })?;
                        Ok((name, schema))
                    })
                    .collect::<Result<Vec<_>, SchemaError>>()?,
                Some(_) => return Err(SchemaError::InvalidProperties),
                None => Vec::new(),
            };
            let pattern_properties =
                parse_named_schemas(&mut object, "patternProperties", location, child_context)?;
            let property_names = match object.shift_remove("propertyNames") {
                Some(value) => Some(Box::new(parse_child_schema(
                    value,
                    &format!("{location}/propertyNames"),
                    "propertyNames",
                    child_context,
                )?)),
                None => None,
            };

            let required_names = match object.shift_remove("required") {
                Some(Value::Array(names)) => {
                    let mut required = Vec::with_capacity(names.len());
                    for value in names {
                        let Value::String(name) = value else {
                            return Err(SchemaError::InvalidRequired);
                        };
                        if required.contains(&name) {
                            return Err(SchemaError::InvalidRequired);
                        }
                        required.push(name);
                    }
                    required
                }
                Some(_) => return Err(SchemaError::InvalidRequired),
                None => Vec::new(),
            };
            let ParsedObjectDependencies {
                dependent_required,
                dependent_schemas,
            } = parse_object_dependencies(&mut object, location, child_context)?;

            let object_constraints = ObjectConstraints {
                min_properties: take_non_negative_integer(&mut object, "minProperties")?,
                max_properties: take_non_negative_integer(&mut object, "maxProperties")?,
                pattern_properties,
                property_names,
                dependent_required,
                dependent_schemas,
                additional_properties: match object.shift_remove("additionalProperties") {
                    Some(value @ (Value::Object(_) | Value::Bool(_))) => {
                        Some(Box::new(parse_value(
                            value,
                            &format!("{location}/additionalProperties"),
                            child_context,
                        )?))
                    }
                    Some(_) => {
                        return Err(SchemaError::InvalidKeywordValue("additionalProperties"));
                    }
                    None => None,
                },
                unevaluated_properties: if dialect.supports_unevaluated() {
                    parse_optional_schema(
                        &mut object,
                        "unevaluatedProperties",
                        location,
                        child_context,
                    )?
                } else {
                    None
                },
            };
            let ParsedArrayItems {
                items,
                prefix_items,
            } = parse_array_items(&mut object, location, child_context, &mut ignored_keywords)?;
            let contains = match object.shift_remove("contains") {
                Some(value) => Some(Box::new(parse_child_schema(
                    value,
                    &format!("{location}/contains"),
                    "contains",
                    child_context,
                )?)),
                None => None,
            };
            let (min_contains, max_contains) = parse_contains_counts(
                &mut object,
                dialect,
                contains.is_some(),
                &mut ignored_keywords,
            )?;
            let array_constraints = ArrayConstraints {
                items,
                prefix_items,
                contains,
                min_contains,
                max_contains,
                min_items: take_non_negative_integer(&mut object, "minItems")?,
                max_items: take_non_negative_integer(&mut object, "maxItems")?,
                unique_items: match object.shift_remove("uniqueItems") {
                    Some(Value::Bool(value)) => Some(value),
                    Some(_) => return Err(SchemaError::InvalidKeywordValue("uniqueItems")),
                    None => None,
                },
                unevaluated_items: if dialect.supports_unevaluated() {
                    parse_optional_schema(&mut object, "unevaluatedItems", location, child_context)?
                } else {
                    None
                },
            };
            let logical_applicators = LogicalApplicators {
                all_of: parse_schema_array(&mut object, "allOf", location, child_context)?,
                any_of: parse_schema_array(&mut object, "anyOf", location, child_context)?,
                one_of: parse_schema_array(&mut object, "oneOf", location, child_context)?,
                not: match object.shift_remove("not") {
                    Some(value) => Some(Box::new(parse_child_schema(
                        value,
                        &format!("{location}/not"),
                        "not",
                        child_context,
                    )?)),
                    None => None,
                },
            };
            let conditional_applicators = ConditionalApplicators {
                condition: parse_optional_schema(&mut object, "if", location, child_context)?,
                then_branch: parse_optional_schema(&mut object, "then", location, child_context)?,
                else_branch: parse_optional_schema(&mut object, "else", location, child_context)?,
            };

            let string_constraints = StringConstraints {
                min_length: take_non_negative_integer(&mut object, "minLength")?,
                max_length: take_non_negative_integer(&mut object, "maxLength")?,
                pattern: take_string(&mut object, "pattern")?,
            };
            let number_constraints = NumberConstraints {
                minimum: take_number(&mut object, "minimum")?,
                maximum: take_number(&mut object, "maximum")?,
                exclusive_minimum: take_number(&mut object, "exclusiveMinimum")?,
                exclusive_maximum: take_number(&mut object, "exclusiveMaximum")?,
                multiple_of: take_positive_number(&mut object, "multipleOf")?,
            };
            let value_constraints = ValueConstraints {
                const_value: object.shift_remove("const"),
                enum_values: match object.shift_remove("enum") {
                    Some(Value::Array(values)) if !values.is_empty() => Some(values),
                    Some(_) => return Err(SchemaError::InvalidKeywordValue("enum")),
                    None => None,
                },
            };
            let content_media_type = take_string(&mut object, "contentMediaType")?;
            let annotations = Annotations {
                title: take_string(&mut object, "title")?,
                description: take_string(&mut object, "description")?,
                default_value: object.shift_remove("default"),
                examples: match object.shift_remove("examples") {
                    Some(Value::Array(values)) => Some(values),
                    Some(_) => return Err(SchemaError::InvalidKeywordValue("examples")),
                    None => None,
                },
                deprecated: if dialect.supports_deprecated() {
                    take_bool(&mut object, "deprecated")?
                } else {
                    None
                },
                read_only: take_bool(&mut object, "readOnly")?,
                write_only: take_bool(&mut object, "writeOnly")?,
                format: take_string(&mut object, "format")?,
                content_encoding: take_string(&mut object, "contentEncoding")?,
                content_media_type,
                content_schema: if dialect.supports_content_schema() {
                    parse_optional_schema(&mut object, "contentSchema", location, child_context)?
                } else {
                    None
                },
                comment: take_string(&mut object, "$comment")?,
                vocabulary: parse_vocabulary(&mut object, dialect, location)?,
            };
            uninterpreted_keywords.extend(object);

            Ok(Schema {
                location: location.to_owned(),
                identity: SchemaIdentity {
                    identifier,
                    anchor,
                    recursive_anchor,
                    dynamic_anchor,
                    ..SchemaIdentity::default()
                },
                dialect,
                overridden_dialect,
                kind,
                reference,
                recursive_reference,
                dynamic_reference,
                definitions,
                properties,
                required_names,
                object_constraints,
                array_constraints,
                logical_applicators,
                conditional_applicators,
                string_constraints,
                number_constraints,
                value_constraints,
                annotations,
                ignored_keywords,
                uninterpreted_keywords,
            })
        }
        _ => Err(SchemaError::InvalidRoot),
    }
}

fn parse_array_items(
    object: &mut Map<String, Value>,
    location: &str,
    context: ParseContext,
    ignored_keywords: &mut Vec<(String, Value)>,
) -> Result<ParsedArrayItems, SchemaError> {
    if context.effective_dialect().array_items_syntax() == ArrayItemsSyntax::PrefixItems {
        let items = match object.shift_remove("items") {
            Some(value) => Some(Box::new(parse_child_schema(
                value,
                &format!("{location}/items"),
                "items",
                context,
            )?)),
            None => None,
        };
        let prefix_items = match object.shift_remove("prefixItems") {
            Some(Value::Array(values)) => {
                Some(parse_tuple_items(values, location, "prefixItems", context)?)
            }
            Some(_) => return Err(SchemaError::InvalidKeywordValue("prefixItems")),
            None => None,
        };
        return Ok(ParsedArrayItems {
            items,
            prefix_items,
        });
    }

    let (mut items, prefix_items) = match object.shift_remove("items") {
        Some(Value::Array(values)) => (
            None,
            Some(parse_tuple_items(values, location, "items", context)?),
        ),
        Some(value) => (
            Some(Box::new(parse_child_schema(
                value,
                &format!("{location}/items"),
                "items",
                context,
            )?)),
            None,
        ),
        None => (None, None),
    };
    if let Some(value) = object.shift_remove("additionalItems") {
        if prefix_items.is_some() {
            items = Some(Box::new(parse_child_schema(
                value,
                &format!("{location}/additionalItems"),
                "additionalItems",
                context,
            )?));
        } else {
            ignored_keywords.push(("additionalItems".to_owned(), value));
        }
    }
    Ok(ParsedArrayItems {
        items,
        prefix_items,
    })
}

fn parse_contains_counts(
    object: &mut Map<String, Value>,
    dialect: Dialect,
    has_contains: bool,
    ignored_keywords: &mut Vec<(String, Value)>,
) -> Result<(Option<Number>, Option<Number>), SchemaError> {
    if !dialect.supports_contains_counts() {
        return Ok((None, None));
    }
    if !has_contains {
        for keyword in ["minContains", "maxContains"] {
            if let Some(value) = object.shift_remove(keyword) {
                ignored_keywords.push((keyword.to_owned(), value));
            }
        }
        return Ok((None, None));
    }
    Ok((
        take_non_negative_integer(object, "minContains")?,
        take_non_negative_integer(object, "maxContains")?,
    ))
}

fn parse_child_schema(
    value: Value,
    location: &str,
    keyword: &'static str,
    context: ParseContext,
) -> Result<Schema, SchemaError> {
    match value {
        value @ (Value::Object(_) | Value::Bool(_)) => parse_value(value, location, context),
        _ => Err(SchemaError::InvalidKeywordValue(keyword)),
    }
}

fn parse_schema_array(
    object: &mut Map<String, Value>,
    keyword: &'static str,
    location: &str,
    context: ParseContext,
) -> Result<Vec<Schema>, SchemaError> {
    let Some(value) = object.shift_remove(keyword) else {
        return Ok(Vec::new());
    };
    let Value::Array(values) = value else {
        return Err(SchemaError::InvalidKeywordValue(keyword));
    };
    if values.is_empty() {
        return Err(SchemaError::InvalidKeywordValue(keyword));
    }
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            parse_child_schema(
                value,
                &format!("{location}/{keyword}/{index}"),
                keyword,
                context,
            )
        })
        .collect()
}

fn parse_optional_schema(
    object: &mut Map<String, Value>,
    keyword: &'static str,
    location: &str,
    context: ParseContext,
) -> Result<Option<Box<Schema>>, SchemaError> {
    match object.shift_remove(keyword) {
        Some(value) => Ok(Some(Box::new(parse_child_schema(
            value,
            &format!("{location}/{keyword}"),
            keyword,
            context,
        )?))),
        None => Ok(None),
    }
}

fn parse_vocabulary(
    object: &mut Map<String, Value>,
    dialect: Dialect,
    location: &str,
) -> Result<Vec<(String, bool)>, SchemaError> {
    if !dialect.supports_vocabulary() || location != "#" {
        return Ok(Vec::new());
    }
    let Some(value) = object.shift_remove("$vocabulary") else {
        return Ok(Vec::new());
    };
    let Value::Object(entries) = value else {
        return Err(SchemaError::InvalidKeywordValue("$vocabulary"));
    };
    entries
        .into_iter()
        .map(|(uri, value)| match value {
            Value::Bool(required) => Ok((uri, required)),
            _ => Err(SchemaError::InvalidKeywordValue("$vocabulary")),
        })
        .collect()
}

fn parse_named_schemas(
    object: &mut Map<String, Value>,
    keyword: &'static str,
    location: &str,
    context: ParseContext,
) -> Result<Vec<(String, Schema)>, SchemaError> {
    let Some(value) = object.shift_remove(keyword) else {
        return Ok(Vec::new());
    };
    let Value::Object(entries) = value else {
        return Err(SchemaError::InvalidKeywordValue(keyword));
    };
    entries
        .into_iter()
        .map(|(name, value)| {
            let schema = parse_child_schema(
                value,
                &child_location(location, keyword, &name),
                keyword,
                context,
            )?;
            Ok((name, schema))
        })
        .collect()
}

fn parse_object_dependencies(
    object: &mut Map<String, Value>,
    location: &str,
    context: ParseContext,
) -> Result<ParsedObjectDependencies, SchemaError> {
    if context.effective_dialect().object_dependencies_syntax() == ObjectDependenciesSyntax::Split {
        return Ok(ParsedObjectDependencies {
            dependent_required: parse_required_dependencies(object, "dependentRequired")?,
            dependent_schemas: parse_named_schemas(object, "dependentSchemas", location, context)?,
        });
    }

    let keyword = "dependencies";
    let Some(value) = object.shift_remove(keyword) else {
        return Ok(ParsedObjectDependencies {
            dependent_required: Vec::new(),
            dependent_schemas: Vec::new(),
        });
    };
    let Value::Object(entries) = value else {
        return Err(SchemaError::InvalidKeywordValue(keyword));
    };
    let mut dependent_required = Vec::new();
    let mut dependent_schemas = Vec::new();
    for (name, value) in entries {
        match value {
            Value::Array(values) => {
                dependent_required.push((name, parse_unique_strings(values, keyword)?));
            }
            value @ (Value::Object(_) | Value::Bool(_)) => {
                let schema = parse_child_schema(
                    value,
                    &child_location(location, keyword, &name),
                    keyword,
                    context,
                )?;
                dependent_schemas.push((name, schema));
            }
            _ => return Err(SchemaError::InvalidKeywordValue(keyword)),
        }
    }
    Ok(ParsedObjectDependencies {
        dependent_required,
        dependent_schemas,
    })
}

fn parse_required_dependencies(
    object: &mut Map<String, Value>,
    keyword: &'static str,
) -> Result<Vec<(String, Vec<String>)>, SchemaError> {
    let Some(value) = object.shift_remove(keyword) else {
        return Ok(Vec::new());
    };
    let Value::Object(entries) = value else {
        return Err(SchemaError::InvalidKeywordValue(keyword));
    };
    entries
        .into_iter()
        .map(|(name, value)| {
            let Value::Array(values) = value else {
                return Err(SchemaError::InvalidKeywordValue(keyword));
            };
            Ok((name, parse_unique_strings(values, keyword)?))
        })
        .collect()
}

fn parse_unique_strings(
    values: Vec<Value>,
    keyword: &'static str,
) -> Result<Vec<String>, SchemaError> {
    let mut names = Vec::with_capacity(values.len());
    for value in values {
        let Value::String(name) = value else {
            return Err(SchemaError::InvalidKeywordValue(keyword));
        };
        if names.contains(&name) {
            return Err(SchemaError::InvalidKeywordValue(keyword));
        }
        names.push(name);
    }
    Ok(names)
}

fn parse_tuple_items(
    values: Vec<Value>,
    location: &str,
    keyword: &'static str,
    context: ParseContext,
) -> Result<Vec<Schema>, SchemaError> {
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            parse_child_schema(
                value,
                &format!("{location}/{keyword}/{index}"),
                keyword,
                context,
            )
        })
        .collect()
}

fn parse_definitions(
    object: &mut Map<String, Value>,
    location: &str,
    context: ParseContext,
) -> Result<Vec<(String, Schema)>, SchemaError> {
    let dialect = context.effective_dialect();
    let keywords: Vec<_> = object
        .keys()
        .filter(|keyword| dialect.definition_keywords().contains(&keyword.as_str()))
        .cloned()
        .collect();
    let mut definitions = Vec::new();
    for keyword in keywords {
        let value = object
            .shift_remove(&keyword)
            .expect("keyword came from the same object");
        let Value::Object(entries) = value else {
            return Err(if keyword == "$defs" {
                SchemaError::InvalidDefinitions
            } else {
                SchemaError::InvalidKeywordValue("definitions")
            });
        };
        for (name, value) in entries {
            let pointer = child_location(location, &keyword, &name);
            let schema = parse_value(value, &pointer, context).map_err(|error| {
                if error.is_invalid_root() {
                    SchemaError::InvalidDefinitionSchema(name.clone()).at(&pointer)
                } else {
                    error
                }
            })?;
            definitions.push((name, schema));
        }
    }
    Ok(definitions)
}

fn select_dialect(
    object: &mut Map<String, Value>,
    location: &str,
    context: ParseContext,
) -> Result<(Dialect, Option<String>), SchemaError> {
    let declared_uri = match object.shift_remove("$schema") {
        Some(Value::String(uri)) => Some(uri),
        Some(_) => return Err(SchemaError::InvalidDialect),
        None => None,
    };
    if declared_uri.is_some() && location != "#" && !object.get("$id").is_some_and(Value::is_string)
    {
        return Err(SchemaError::InvalidDialectLocation);
    }
    if let Some(forced) = context.forced_dialect {
        let overridden = declared_uri.filter(|uri| Dialect::from_schema_uri(uri) != Some(forced));
        return Ok((forced, overridden));
    }
    let dialect = match declared_uri {
        Some(uri) => Dialect::from_schema_uri(&uri)
            .ok_or_else(|| SchemaError::UnsupportedDialect(uri.clone()))?,
        None => context.inherited_dialect,
    };
    Ok((dialect, None))
}

fn child_location(parent: &str, keyword: &str, name: &str) -> String {
    append_location(&format!("{parent}/{keyword}"), name)
}

fn append_location(parent: &str, token: &str) -> String {
    format!("{parent}/{}", token.replace('~', "~0").replace('/', "~1"))
}

fn valid_type(kind: &str) -> bool {
    matches!(
        kind,
        "null" | "boolean" | "object" | "array" | "number" | "integer" | "string"
    )
}

fn take_non_negative_integer(
    object: &mut Map<String, Value>,
    keyword: &'static str,
) -> Result<Option<Number>, SchemaError> {
    match object.shift_remove(keyword) {
        Some(Value::Number(number)) if is_non_negative_integer(&number) => Ok(Some(number)),
        Some(_) => Err(SchemaError::InvalidKeywordValue(keyword)),
        None => Ok(None),
    }
}

fn take_string(
    object: &mut Map<String, Value>,
    keyword: &'static str,
) -> Result<Option<String>, SchemaError> {
    match object.shift_remove(keyword) {
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => Err(SchemaError::InvalidKeywordValue(keyword)),
        None => Ok(None),
    }
}

fn take_bool(
    object: &mut Map<String, Value>,
    keyword: &'static str,
) -> Result<Option<bool>, SchemaError> {
    match object.shift_remove(keyword) {
        Some(Value::Bool(value)) => Ok(Some(value)),
        Some(_) => Err(SchemaError::InvalidKeywordValue(keyword)),
        None => Ok(None),
    }
}

fn take_number(
    object: &mut Map<String, Value>,
    keyword: &'static str,
) -> Result<Option<Number>, SchemaError> {
    match object.shift_remove(keyword) {
        Some(Value::Number(value)) => Ok(Some(value)),
        Some(_) => Err(SchemaError::InvalidKeywordValue(keyword)),
        None => Ok(None),
    }
}

fn take_positive_number(
    object: &mut Map<String, Value>,
    keyword: &'static str,
) -> Result<Option<Number>, SchemaError> {
    match take_number(object, keyword)? {
        Some(value) if is_positive_number(&value) => Ok(Some(value)),
        Some(_) => Err(SchemaError::InvalidKeywordValue(keyword)),
        None => Ok(None),
    }
}

fn is_non_negative_integer(number: &Number) -> bool {
    let text = number.to_string();
    let (mantissa, exponent) = text.split_once(['e', 'E']).unwrap_or((&text, "0"));
    if mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .all(|digit| digit == b'0')
    {
        return true;
    }
    if mantissa.starts_with('-') {
        return false;
    }

    let fractional_digits = mantissa.split_once('.').map_or(0, |(_, part)| part.len());
    let trailing_zeros = mantissa
        .bytes()
        .rev()
        .take_while(|digit| *digit == b'0')
        .count();
    match exponent.parse::<i128>() {
        Ok(exponent) => {
            exponent.saturating_add(trailing_zeros as i128) >= fractional_digits as i128
        }
        Err(_) => !exponent.starts_with('-'),
    }
}

fn is_positive_number(number: &Number) -> bool {
    let text = number.to_string();
    let mantissa = text
        .split_once(['e', 'E'])
        .map_or(text.as_str(), |(part, _)| part);
    !mantissa.starts_with('-') && mantissa.bytes().any(|digit| matches!(digit, b'1'..=b'9'))
}

#[cfg(test)]
mod tests {
    use crate::schema::parse;

    #[test]
    fn records_every_json_value_location() {
        let document = parse(
            r#"{
                "properties": {"a/b": {"type": "string"}},
                "default": {"~items": [true]}
            }"#,
            None,
        )
        .expect("schema should parse");

        for location in [
            "#",
            "#/properties",
            "#/properties/a~1b",
            "#/properties/a~1b/type",
            "#/default/~0items/0",
        ] {
            assert!(
                document.value_locations.contains(location),
                "missing {location}"
            );
        }
    }
}
