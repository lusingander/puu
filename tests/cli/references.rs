use predicates::prelude::*;
use rstest::rstest;

use crate::support::command;

const INPUT: &str = r##"{
  "type": "object",
  "properties": {"customer": {"$ref": "#/$defs/User"}},
  "$defs": {
    "User": {
      "type": "object", "required": ["name"],
      "properties": {"name": {"$ref": "#/$defs/Name"}}
    },
    "Name": {"type": "string"},
    "Unused": {"type": "integer"}
  }
}"##;

const EXPANDED: &str = "\
root object
└─ customer -> User
   └─ User object [expanded from $ref]
      └─ name -> Name [required]
         └─ Name string [expanded from $ref]
";

#[rstest]
#[case("-r")]
#[case("--expand-refs")]
fn accepts_short_and_long_options(#[case] option: &str) {
    command()
        .args([option, "--definitions", "none", "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(EXPANDED)
        .stderr("");
}

#[test]
fn leaves_references_unexpanded_by_default() {
    command()
        .args(["-D", "none", "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout("root object\n└─ customer -> User\n")
        .stderr("");
}

#[test]
fn documents_the_short_option_in_help() {
    command()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("-r, --expand-refs"))
        .stderr("");
}

#[rstest]
#[case(
    "all",
    "\nDefinitions\n├─ User object\n│  └─ name -> Name [required]\n│     └─ Name string [expanded from $ref]\n├─ Name string\n└─ Unused integer\n"
)]
#[case(
    "referenced",
    "\nDefinitions\n├─ User object\n│  └─ name -> Name [required]\n│     └─ Name string [expanded from $ref]\n└─ Name string\n"
)]
#[case("none", "")]
fn respects_definition_display_modes(#[case] mode: &str, #[case] definitions: &str) {
    command()
        .args(["-r", "-D", mode, "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(format!("{EXPANDED}{definitions}"))
        .stderr("");
}

#[test]
fn expands_targets_outside_the_pointer_selection() {
    command()
        .args(["-r", "-p", "#/$defs/User", "-D", "referenced", "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(
            "\
User object [location: #/$defs/User]
└─ name -> Name [required]
   └─ Name string [expanded from $ref]

Definitions
└─ Name string
",
        )
        .stderr("");
}

#[rstest]
#[case("0", "root object\n└─ … [children omitted]\n")]
#[case("1", "root object\n└─ customer -> User\n   └─ … [children omitted]\n")]
#[case(
    "2",
    "root object\n└─ customer -> User\n   └─ User object [expanded from $ref]\n      └─ … [children omitted]\n"
)]
fn limits_expanded_display_depth(#[case] depth: &str, #[case] expected: &str) {
    command()
        .args(["-r", "-D", "none", "-L", depth, "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(expected.to_owned())
        .stderr("");
}

#[test]
fn counts_the_definitions_section_in_display_depth() {
    command()
        .args(["-r", "-D", "referenced", "-L", "1", "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(
            "\
root object
└─ customer -> User
   └─ … [children omitted]

Definitions
├─ User object
│  └─ … [children omitted]
└─ Name string
",
        )
        .stderr("");
}

#[rstest]
#[case(
    "7",
    "root -> Value [ignored next to $ref: type, properties]\n└─ Value string [expanded from $ref]\n"
)]
#[case(
    "2020-12",
    "root -> Value [type: object]\n├─ Value string [expanded from $ref]\n└─ own integer\n"
)]
fn preserves_draft_specific_ref_sibling_rules(#[case] draft: &str, #[case] expected: &str) {
    command()
        .args(["-r", "-d", draft, "-D", "none", "-"])
        .write_stdin(
            r##"{
            "$ref": "#/definitions/Value", "type": "object",
            "properties": {"own": {"type": "integer"}},
            "definitions": {"Value": {"type": "string"}}
        }"##,
        )
        .assert()
        .success()
        .stdout(expected.to_owned())
        .stderr("");
}

#[test]
fn resolves_anchors_and_embedded_resource_ids_before_expansion() {
    command()
        .args(["-r", "-D", "none", "-"])
        .write_stdin(
            r##"{
            "$id": "https://example.test/root",
            "properties": {"value": {"$ref": "value#entry"}},
            "$defs": {"Value": {"$id": "value", "$anchor": "entry", "type": "string"}}
        }"##,
        )
        .assert()
        .success()
        .stdout(
            "\
root <type unspecified> [resource: https://example.test/root]
└─ value -> Value
   └─ Value string [resource: https://example.test/value] [anchor: entry] [expanded from $ref]
",
        )
        .stderr("");
}

#[test]
fn marks_expanded_dynamic_initial_targets() {
    command()
        .args(["-r", "-D", "none", "-"])
        .write_stdin(
            r##"{
            "properties": {"value": {"$dynamicRef": "#node"}},
            "$defs": {"Value": {"$dynamicAnchor": "node", "type": "string"}}
        }"##,
        )
        .assert()
        .success()
        .stdout(
            "\
root <type unspecified>
└─ value -> Value [dynamic ref start] [initial target; dynamic scope not evaluated]
   └─ Value string [dynamic anchor: node] [expanded from $dynamicRef]
",
        )
        .stderr("");
}

#[test]
fn marks_expanded_recursive_initial_targets() {
    command()
        .args(["-r", "-D", "none", "-p", "#/$defs/Tree/properties/next", "-"])
        .write_stdin(r##"{
            "$schema": "https://json-schema.org/draft/2019-09/schema",
            "$defs": {"Tree": {
                "$id": "https://example.test/tree", "$recursiveAnchor": true,
                "type": "object", "properties": {"next": {"$recursiveRef": "#"}}
            }}
        }"##)
        .assert()
        .success()
        .stdout("\
next -> https://example.test/tree [recursive] [recursive ref start] [initial target; dynamic scope not evaluated] [location: #/$defs/Tree/properties/next]
└─ https://example.test/tree object [resource: https://example.test/tree] [recursive anchor] [expanded from $recursiveRef]
   └─ next -> https://example.test/tree [recursive] [recursive ref start] [initial target; dynamic scope not evaluated] [expansion stopped: cycle]
")
        .stderr("");
}

#[test]
fn escapes_control_characters_in_expanded_names_and_annotations() {
    command()
        .args(["-r", "-D", "none", "-"])
        .write_stdin(
            r##"{
            "$ref": "#/$defs/a%0Ab",
            "$defs": {"a\nb": {"type": "string", "format": "\u001b[31m"}}
        }"##,
        )
        .assert()
        .success()
        .stdout("root -> a\\nb\n└─ a\\nb string [format: \\u{1b}[31m] [expanded from $ref]\n")
        .stderr("");
}
