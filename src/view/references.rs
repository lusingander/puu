use crate::{
    schema::{Reference, ReferenceTarget},
    view::{
        build::schema_node_with_role,
        context::BuildContext,
        depth::omission_node,
        model::{SchemaNodeRole, ViewDetail, ViewNode, ViewNodeRole},
    },
};

pub(super) fn expand(
    reference: &Reference,
    details: &mut Vec<ViewDetail>,
    context: &mut BuildContext<'_, '_>,
) -> Option<ViewNode> {
    if !context.options.expand_refs {
        return None;
    }
    let ReferenceTarget::Local {
        location,
        display_name,
        ..
    } = &reference.target
    else {
        return None;
    };
    if reference.dynamic_start {
        details.push(ViewDetail::marker(
            "initial target; dynamic scope not evaluated",
        ));
    }
    if context.beyond_max_depth() {
        return Some(omission_node());
    }
    if context.active_schemas.contains(location) {
        details.push(ViewDetail::marker("expansion stopped: cycle"));
        return None;
    }
    if let Some(reason) = context.expansion_stop_reason() {
        details.push(ViewDetail::marker(reason));
        return None;
    }

    let target = context
        .index
        .get(location)
        .expect("a local reference target should be indexed")
        .schema;
    let name = if display_name.is_empty() {
        location
    } else {
        display_name
    };
    context.expansion_depth += 1;
    let mut node = schema_node_with_role(
        target,
        name,
        false,
        false,
        SchemaNodeRole::ExpandedReference,
        context,
    );
    context.expansion_depth -= 1;
    if !matches!(node.role, ViewNodeRole::Omission) {
        node.details.push(ViewDetail::metadata(format!(
            "expanded from {}",
            reference.kind.keyword()
        )));
    }
    Some(node)
}

#[cfg(test)]
mod tests {
    use serde_json::{Map, Value, json};

    use crate::{schema, text, view};

    fn render(input: &str, pointer: Option<&str>, max_depth: Option<usize>) -> String {
        let parsed = schema::parse(input, None).expect("schema should parse");
        let index = schema::SchemaIndex::new(&parsed);
        let root = index
            .select(pointer.unwrap_or("#"))
            .expect("root should exist");
        let options = view::ViewOptions {
            expand_refs: true,
            definitions: view::DefinitionsMode::None,
            max_depth,
            ..view::ViewOptions::default()
        };
        let document = view::from_schema(
            &parsed.root,
            &index,
            root.schema,
            &root.display_name,
            pointer.is_some(),
            &options,
        );
        console::strip_ansi_codes(&text::render(&document, &text::ColorTheme::default()))
            .into_owned()
    }

    #[test]
    fn expands_shared_targets_and_reference_chains_without_merging_siblings() {
        let output = render(
            r##"{
            "type": "object", "required": ["left"],
            "properties": {
                "left": {"$ref": "#/$defs/A", "maxProperties": 3},
                "right": {"$ref": "#/$defs/A"}
            },
            "$defs": {
                "A": {"$ref": "#/$defs/B"},
                "B": {"type": "object", "properties": {"name": {"type": "string"}}}
            }
        }"##,
            None,
            None,
        );
        assert_eq!(
            output,
            "\
root object
├─ left -> A [required] [<= 3 properties]
│  └─ A -> B [expanded from $ref]
│     └─ B object [expanded from $ref]
│        └─ name string
└─ right -> A
   └─ A -> B [expanded from $ref]
      └─ B object [expanded from $ref]
         └─ name string
"
        );
    }

    #[test]
    fn stops_mutual_reference_cycles() {
        let output = render(
            r##"{
            "$ref": "#/$defs/A",
            "$defs": {"A": {"$ref": "#/$defs/B"}, "B": {"$ref": "#/$defs/A"}}
        }"##,
            None,
            None,
        );
        assert_eq!(
            output,
            "\
root -> A
└─ A -> B [expanded from $ref]
   └─ B -> A [expansion stopped: cycle] [expanded from $ref]
"
        );
    }

    #[test]
    fn seeds_the_active_path_with_the_document_or_selected_root() {
        assert_eq!(
            render(
                r##"{
            "type": "object", "properties": {"next": {"$ref": "#"}}
        }"##,
                None,
                None
            ),
            "root object\n└─ next -> root [recursive] [expansion stopped: cycle]\n"
        );
        assert_eq!(
            render(
                r##"{
            "$defs": {"User": {
                "type": "object", "properties": {"next": {"$ref": "#/$defs/User"}}
            }}
        }"##,
                Some("#/$defs/User"),
                None
            ),
            "User object [location: #/$defs/User]\n└─ next -> User [recursive] [expansion stopped: cycle]\n"
        );
    }

    #[test]
    fn expands_boolean_targets_and_leaves_unknown_references_visible() {
        assert_eq!(
            render(
                r##"{
            "properties": {
                "yes": {"$ref": "#/$defs/Yes"},
                "no": {"$ref": "#/$defs/No"},
                "missing": {"$ref": "#/$defs/Missing"},
                "remote": {"$ref": "https://example.test/schema"}
            },
            "$defs": {"Yes": true, "No": false}
        }"##,
                None,
                None
            ),
            "\
root <type unspecified>
├─ yes -> Yes
│  └─ Yes any [expanded from $ref]
├─ no -> No
│  └─ No never [expanded from $ref]
├─ missing -> #/$defs/Missing [unresolved]
└─ remote -> https://example.test/schema [external]
"
        );
    }

    #[test]
    fn preserves_the_selected_schema_on_the_path_when_expanding_its_ancestor() {
        let output = render(
            r##"{
                "type": "object",
                "properties": {"value": {"$ref": "#", "$dynamicRef": "#/properties/value"}}
            }"##,
            Some("#/properties/value"),
            None,
        );
        assert_eq!(output, "\
value -> root [recursive] [location: #/properties/value]
├─ root object [expanded from $ref]
│  └─ value -> root [recursive] [expansion stopped: cycle]
│     └─ $dynamicRef -> #/properties/value [recursive] [dynamic ref: static start] [expansion stopped: cycle]
└─ $dynamicRef -> #/properties/value [recursive] [dynamic ref: static start] [expansion stopped: cycle]
");
    }

    #[test]
    fn marks_dynamic_start_points_and_expands_additional_references() {
        let output = render(
            r##"{
            "$dynamicAnchor": "node",
            "properties": {"combined": {"$ref": "#plain", "$dynamicRef": "#node"}},
            "$defs": {"Plain": {"$anchor": "plain", "type": "string"}}
        }"##,
            None,
            None,
        );
        assert_eq!(output, "\
root <type unspecified> [dynamic anchor: node]
└─ combined -> Plain
   ├─ Plain string [anchor: plain] [expanded from $ref]
   └─ $dynamicRef -> root [recursive] [dynamic ref start] [initial target; dynamic scope not evaluated] [expansion stopped: cycle]
");
    }

    #[test]
    fn applies_display_depth_before_expanding_targets() {
        let input = r##"{
            "type": "object", "properties": {"value": {"$ref": "#/$defs/Value"}},
            "$defs": {"Value": {"type": "string"}}
        }"##;
        assert_eq!(
            render(input, None, Some(1)),
            "root object\n└─ value -> Value\n   └─ … [children omitted]\n"
        );
        assert_eq!(
            render(input, None, Some(2)),
            "root object\n└─ value -> Value\n   └─ Value string [expanded from $ref]\n"
        );
    }

    #[test]
    fn bounds_long_acyclic_reference_chains() {
        let mut definitions = Map::new();
        for index in 0..200 {
            definitions.insert(
                format!("N{index}"),
                json!({"$ref": format!("#/$defs/N{}", index + 1)}),
            );
        }
        definitions.insert("N200".to_owned(), json!({"type": "string"}));
        let input = json!({"$ref": "#/$defs/N0", "$defs": definitions}).to_string();
        let output = render(&input, None, None);
        assert!(output.contains("[expansion stopped: depth limit]"));
        assert!(!output.contains("[expansion stopped: cycle]"));
        assert_eq!(output.matches("[expanded from $ref]").count(), 64);
    }

    #[test]
    fn bounds_repeated_expansion_of_an_acyclic_reference_graph() {
        let mut definitions = Map::new();
        definitions.insert("N0".to_owned(), json!({"type": "string"}));
        for index in 1..=18 {
            let reference = json!({"$ref": format!("#/$defs/N{}", index - 1)});
            definitions.insert(
                format!("N{index}"),
                json!({
                    "type": "object", "properties": {"left": reference, "right": reference}
                }),
            );
        }
        let input = Value::Object(Map::from_iter([
            ("$ref".to_owned(), json!("#/$defs/N18")),
            ("$defs".to_owned(), Value::Object(definitions)),
        ]))
        .to_string();
        let output = render(&input, None, None);
        assert!(output.contains("[expansion stopped: node limit]"));
        assert!(!output.contains("[expansion stopped: cycle]"));
        assert!(output.lines().count() < 20_100);
    }
}
