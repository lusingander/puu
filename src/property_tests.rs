use proptest::{collection::vec, prelude::*};
use serde_json::{Map, Number, Value};

use crate::{dialect::Dialect, schema, text, view};

const PROPERTY_CASES: u32 = 256;
const OUTPUT_PROPERTY_CASES: u32 = 64;

fn arbitrary_json() -> impl Strategy<Value = Value> {
    let leaf = prop_oneof![
        Just(Value::Null),
        any::<bool>().prop_map(Value::Bool),
        any::<i64>().prop_map(|value| Value::Number(Number::from(value))),
        any::<String>().prop_map(Value::String),
    ];

    leaf.prop_recursive(8, 128, 16, |inner| {
        prop_oneof![
            vec(inner.clone(), 0..16).prop_map(Value::Array),
            vec((any::<String>(), inner), 0..16).prop_map(|entries| {
                Value::Object(entries.into_iter().collect::<Map<String, Value>>())
            }),
        ]
    })
}

fn valid_uri_reference() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "",
        "#",
        "#anchor",
        "#/$defs/node",
        "../schema.json",
        "child.json#value",
        "https://example.test/schema",
    ])
    .prop_map(str::to_owned)
}

fn uri_reference() -> BoxedStrategy<String> {
    prop_oneof![8 => valid_uri_reference(), 1 => any::<String>()].boxed()
}

fn scalar_schema_field() -> BoxedStrategy<(String, Value)> {
    let schema_uri = prop::sample::select(vec![
        "https://json-schema.org/draft/2020-12/schema",
        "https://json-schema.org/draft/2019-09/schema",
        "http://json-schema.org/draft-07/schema#",
        "https://example.test/unsupported",
    ])
    .prop_map(|value| ("$schema".to_owned(), Value::String(value.to_owned())));
    let schema_type = prop::sample::select(vec![
        "null", "boolean", "object", "array", "number", "integer", "string",
    ])
    .prop_map(|value| ("type".to_owned(), Value::String(value.to_owned())));
    let schema_types = vec(
        prop::sample::select(vec![
            "null", "boolean", "object", "array", "number", "integer", "string",
        ]),
        1..5,
    )
    .prop_map(|values| {
        (
            "type".to_owned(),
            Value::Array(
                values
                    .into_iter()
                    .map(|value| Value::String(value.to_owned()))
                    .collect(),
            ),
        )
    });
    let identity_keyword = prop_oneof![
        (
            prop::sample::select(vec!["$id", "$ref", "$dynamicRef"]),
            uri_reference()
        )
            .prop_map(|(keyword, value)| (keyword.to_owned(), Value::String(value))),
        (
            prop::sample::select(vec!["$anchor", "$dynamicAnchor"]),
            "[A-Za-z_][A-Za-z0-9_.-]{0,12}",
        )
            .prop_map(|(keyword, value)| (keyword.to_owned(), Value::String(value))),
        Just(("$recursiveRef".to_owned(), Value::String("#".to_owned()))),
    ];
    let annotation_keyword = (
        prop::sample::select(vec![
            "title",
            "description",
            "pattern",
            "format",
            "contentEncoding",
            "contentMediaType",
            "$comment",
        ]),
        any::<String>(),
    )
        .prop_map(|(keyword, value)| (keyword.to_owned(), Value::String(value)));
    let integer_keyword = (
        prop::sample::select(vec![
            "minProperties",
            "maxProperties",
            "minItems",
            "maxItems",
            "minContains",
            "maxContains",
            "minLength",
            "maxLength",
        ]),
        0_u64..10_000,
    )
        .prop_map(|(keyword, value)| (keyword.to_owned(), Value::Number(Number::from(value))));
    let number_keyword = (
        prop::sample::select(vec![
            "minimum",
            "maximum",
            "exclusiveMinimum",
            "exclusiveMaximum",
            "multipleOf",
        ]),
        1_i64..10_000,
    )
        .prop_map(|(keyword, value)| (keyword.to_owned(), Value::Number(Number::from(value))));
    let boolean_keyword = (
        prop::sample::select(vec![
            "uniqueItems",
            "deprecated",
            "readOnly",
            "writeOnly",
            "$recursiveAnchor",
        ]),
        any::<bool>(),
    )
        .prop_map(|(keyword, value)| (keyword.to_owned(), Value::Bool(value)));
    let required = vec(any::<String>(), 0..8).prop_map(|names| {
        (
            "required".to_owned(),
            Value::Array(names.into_iter().map(Value::String).collect()),
        )
    });
    let dependent_required =
        vec((any::<String>(), vec(any::<String>(), 0..6)), 0..6).prop_map(|dependencies| {
            (
                "dependentRequired".to_owned(),
                Value::Object(
                    dependencies
                        .into_iter()
                        .map(|(name, required)| {
                            (
                                name,
                                Value::Array(required.into_iter().map(Value::String).collect()),
                            )
                        })
                        .collect(),
                ),
            )
        });
    let vocabulary = vec((uri_reference(), any::<bool>()), 0..6).prop_map(|entries| {
        (
            "$vocabulary".to_owned(),
            Value::Object(
                entries
                    .into_iter()
                    .map(|(uri, required)| (uri, Value::Bool(required)))
                    .collect(),
            ),
        )
    });
    let values = (
        prop::sample::select(vec!["enum", "examples"]),
        vec(arbitrary_json(), 0..6),
    )
        .prop_map(|(keyword, values)| (keyword.to_owned(), Value::Array(values)));
    let arbitrary_known = (
        prop::sample::select(vec![
            "type",
            "required",
            "items",
            "properties",
            "$id",
            "$ref",
            "minimum",
            "uniqueItems",
            "allOf",
            "format",
        ]),
        arbitrary_json(),
    )
        .prop_map(|(keyword, value)| (keyword.to_owned(), value));
    let unknown = (any::<String>(), arbitrary_json());

    prop_oneof![
        2 => schema_uri,
        4 => schema_type,
        2 => schema_types,
        4 => identity_keyword,
        6 => annotation_keyword,
        4 => integer_keyword,
        3 => number_keyword,
        3 => boolean_keyword,
        3 => required,
        2 => dependent_required,
        1 => vocabulary,
        2 => values,
        2 => arbitrary_known,
        1 => unknown,
    ]
    .boxed()
}

fn nested_schema_field<S>(inner: S) -> BoxedStrategy<(String, Value)>
where
    S: Strategy<Value = Value> + Clone + 'static,
{
    let direct = (
        prop::sample::select(vec![
            "items",
            "additionalItems",
            "contains",
            "additionalProperties",
            "propertyNames",
            "unevaluatedProperties",
            "unevaluatedItems",
            "not",
            "if",
            "then",
            "else",
            "contentSchema",
        ]),
        inner.clone(),
    )
        .prop_map(|(keyword, schema)| (keyword.to_owned(), schema));
    let array = (
        prop::sample::select(vec!["prefixItems", "allOf", "anyOf", "oneOf"]),
        vec(inner.clone(), 0..6),
    )
        .prop_map(|(keyword, schemas)| (keyword.to_owned(), Value::Array(schemas)));
    let named = (
        prop::sample::select(vec![
            "properties",
            "patternProperties",
            "$defs",
            "definitions",
            "dependentSchemas",
            "dependencies",
        ]),
        vec((any::<String>(), inner), 0..6),
    )
        .prop_map(|(keyword, entries)| {
            (
                keyword.to_owned(),
                Value::Object(entries.into_iter().collect::<Map<String, Value>>()),
            )
        });

    prop_oneof![direct, array, named].boxed()
}

fn schema_like_json() -> BoxedStrategy<Value> {
    let leaf = prop_oneof![
        any::<bool>().prop_map(Value::Bool),
        vec(scalar_schema_field(), 0..8).prop_map(|entries| {
            Value::Object(entries.into_iter().collect::<Map<String, Value>>())
        }),
    ];

    leaf.prop_recursive(8, 256, 16, |inner| {
        let field = prop_oneof![scalar_schema_field(), nested_schema_field(inner)];
        prop_oneof![
            any::<bool>().prop_map(Value::Bool),
            vec(field, 0..12).prop_map(|entries| {
                Value::Object(entries.into_iter().collect::<Map<String, Value>>())
            }),
        ]
    })
    .boxed()
}

fn reference_schema() -> impl Strategy<Value = Value> {
    ("[A-Za-z][A-Za-z0-9_-]{0,12}", schema_like_json()).prop_map(|(name, definition)| {
        let mut definitions = Map::new();
        definitions.insert(name.clone(), definition);
        let mut root = Map::new();
        root.insert("$defs".to_owned(), Value::Object(definitions));
        root.insert("$ref".to_owned(), Value::String(format!("#/$defs/{name}")));
        Value::Object(root)
    })
}

fn schema_input() -> BoxedStrategy<Value> {
    prop_oneof![4 => schema_like_json(), 1 => reference_schema().boxed()].boxed()
}

fn renderable_fields() -> impl Strategy<Value = Map<String, Value>> {
    (
        prop::option::of(prop::sample::select(vec![
            "null", "boolean", "object", "array", "number", "integer", "string",
        ])),
        prop::option::of(any::<String>()),
        prop::option::of(any::<String>()),
        prop::option::of(any::<String>()),
        prop::collection::btree_set(any::<String>(), 0..6),
        prop::option::of(arbitrary_json()),
        prop::option::of(vec(arbitrary_json(), 1..5)),
        prop::option::of(valid_uri_reference()),
    )
        .prop_map(
            |(kind, title, description, format, required, constant, enumeration, reference)| {
                let mut fields = Map::new();
                if let Some(kind) = kind {
                    fields.insert("type".to_owned(), Value::String(kind.to_owned()));
                }
                if let Some(title) = title {
                    fields.insert("title".to_owned(), Value::String(title));
                }
                if let Some(description) = description {
                    fields.insert("description".to_owned(), Value::String(description));
                }
                if let Some(format) = format {
                    fields.insert("format".to_owned(), Value::String(format));
                }
                if !required.is_empty() {
                    fields.insert(
                        "required".to_owned(),
                        Value::Array(required.into_iter().map(Value::String).collect()),
                    );
                }
                if let Some(constant) = constant {
                    fields.insert("const".to_owned(), constant);
                }
                if let Some(enumeration) = enumeration {
                    fields.insert("enum".to_owned(), Value::Array(enumeration));
                }
                if let Some(reference) = reference {
                    fields.insert("$ref".to_owned(), Value::String(reference));
                }
                fields
            },
        )
}

fn renderable_schema() -> BoxedStrategy<Value> {
    let leaf = prop_oneof![
        1 => any::<bool>().prop_map(Value::Bool),
        4 => renderable_fields().prop_map(Value::Object),
    ];

    leaf.prop_recursive(5, 64, 8, |inner| {
        (
            renderable_fields(),
            vec((any::<String>(), inner.clone()), 0..4),
            vec((any::<String>(), inner.clone()), 0..4),
            prop::option::of(inner.clone()),
            prop::option::of(vec(inner, 1..4)),
        )
            .prop_map(|(mut fields, properties, definitions, items, all_of)| {
                if !properties.is_empty() {
                    fields.insert(
                        "properties".to_owned(),
                        Value::Object(properties.into_iter().collect()),
                    );
                }
                if !definitions.is_empty() {
                    fields.insert(
                        "$defs".to_owned(),
                        Value::Object(definitions.into_iter().collect()),
                    );
                }
                if let Some(items) = items {
                    fields.insert("items".to_owned(), items);
                }
                if let Some(all_of) = all_of {
                    fields.insert("allOf".to_owned(), Value::Array(all_of));
                }
                Value::Object(fields)
            })
    })
    .boxed()
}

fn arbitrary_draft() -> impl Strategy<Value = Option<Dialect>> {
    prop_oneof![
        Just(None),
        Just(Some(Dialect::Draft202012)),
        Just(Some(Dialect::Draft201909)),
        Just(Some(Dialect::Draft7)),
    ]
}

fn render_pipeline(input: &str, draft: Option<Dialect>, verbose: bool) -> Result<String, String> {
    let schema = schema::parse(input, draft).map_err(|error| error.to_string())?;
    let options = view::ViewOptions {
        verbose,
        ..view::ViewOptions::default()
    };
    let document = view::from_schema(&schema, &options);
    let plain = console::Style::new();
    let theme = text::ColorTheme {
        key: plain.clone(),
        value: plain.clone(),
        connector: plain.clone(),
        constraint: plain.clone(),
        annotation: plain.clone(),
        metadata: plain.clone(),
        marker: plain,
    };
    Ok(text::render(&document, &theme))
}

fn exercise_pipeline(input: &str, draft: Option<Dialect>) {
    match schema::parse(input, draft) {
        Ok(schema) => {
            for verbose in [false, true] {
                let options = view::ViewOptions {
                    verbose,
                    ..view::ViewOptions::default()
                };
                let document = view::from_schema(&schema, &options);
                let _ = text::render(&document, &text::ColorTheme::default());
            }
        }
        Err(error) => {
            let _ = error.to_string();
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(PROPERTY_CASES))]

    #[test]
    fn arbitrary_utf8_input_does_not_panic(input in any::<String>()) {
        exercise_pipeline(&input, None);
    }

    #[test]
    fn arbitrary_json_input_does_not_panic(value in arbitrary_json()) {
        let input = serde_json::to_string(&value).expect("generated JSON should serialize");
        exercise_pipeline(&input, None);
    }

    #[test]
    fn schema_like_input_does_not_panic(
        value in schema_input(),
        draft in arbitrary_draft(),
    ) {
        let input = serde_json::to_string(&value).expect("generated schema should serialize");
        exercise_pipeline(&input, draft);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(OUTPUT_PROPERTY_CASES))]

    #[test]
    fn rendered_schema_is_deterministic_and_has_no_control_characters(
        value in renderable_schema(),
        verbose in any::<bool>(),
    ) {
        let input = serde_json::to_string(&value).expect("generated schema should serialize");
        let first = render_pipeline(&input, Some(Dialect::Draft202012), verbose);
        let second = render_pipeline(&input, Some(Dialect::Draft202012), verbose);

        prop_assert_eq!(&first, &second);
        let output = first.expect("renderable schema should parse");
        prop_assert!(output.ends_with('\n'));
        prop_assert!(
            output
                .chars()
                .all(|character| character == '\n' || !character.is_control())
        );
    }
}
