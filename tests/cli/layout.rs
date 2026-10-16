use std::fs;

use predicates::prelude::*;
use rstest::rstest;

use crate::support::{command, fixture};

const INPUT: &str = r##"{
  "type": "object",
  "properties": {
    "customer": { "$ref": "#/$defs/User" }
  },
  "$defs": {
    "User": {
      "type": "object",
      "required": ["name"],
      "properties": {
        "name": { "$ref": "#/$defs/Name" }
      }
    },
    "Name": { "type": "string" },
    "Unused": { "type": "integer" }
  }
}"##;

#[test]
fn defaults_to_sections_and_accepts_explicit_sections() {
    let expected =
        fs::read(fixture("render", "definitions", "txt")).expect("expected output should exist");

    for arguments in [vec![], vec!["--layout", "sections"], vec!["-l", "sections"]] {
        command()
            .args(arguments)
            .arg(fixture("render", "definitions", "json"))
            .assert()
            .success()
            .stdout(expected.clone())
            .stderr("");
    }
}

#[rstest]
#[case::all(
    "all",
    "\
Schema
├─ root object
│  └─ customer -> User
└─ Definitions
   ├─ User object
   │  └─ name -> Name [required]
   ├─ Name string
   └─ Unused integer
"
)]
#[case::referenced(
    "referenced",
    "\
Schema
├─ root object
│  └─ customer -> User
└─ Definitions
   ├─ User object
   │  └─ name -> Name [required]
   └─ Name string
"
)]
#[case::none(
    "none",
    "\
Schema
└─ root object
   └─ customer -> User
"
)]
fn tree_layout_respects_definition_selection(
    #[values("--layout", "-l")] option: &str,
    #[case] mode: &str,
    #[case] expected: &str,
) {
    command()
        .args([option, "tree", "--definitions", mode, "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(expected.to_owned())
        .stderr("");
}

#[test]
fn tree_layout_wraps_a_schema_without_definitions() {
    command()
        .args(["-l", "tree", "-L", "0"])
        .arg(fixture("render", "scalar-string", "json"))
        .assert()
        .success()
        .stdout("Schema\n└─ root string\n")
        .stderr("");
}

#[test]
fn tree_layout_preserves_pointer_selection_and_location() {
    command()
        .args([
            "--layout",
            "tree",
            "--pointer",
            "#/$defs/User",
            "-D",
            "referenced",
            "-",
        ])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(
            "\
Schema
├─ User object [location: #/$defs/User]
│  └─ name -> Name [required]
└─ Definitions
   └─ Name string
",
        )
        .stderr("");
}

#[test]
fn tree_layout_expands_references_in_each_section() {
    command()
        .args(["--layout", "tree", "-r", "-D", "referenced", "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(
            "\
Schema
├─ root object
│  └─ customer -> User
│     └─ User object [expanded from $ref]
│        └─ name -> Name [required]
│           └─ Name string [expanded from $ref]
└─ Definitions
   ├─ User object
   │  └─ name -> Name [required]
   │     └─ Name string [expanded from $ref]
   └─ Name string
",
        )
        .stderr("");
}

#[rstest]
#[case::zero(
    "0",
    "\
Schema
├─ root object
│  └─ … [children omitted]
└─ Definitions
   └─ … [children omitted]
"
)]
#[case::one(
    "1",
    "\
Schema
├─ root object
│  └─ customer -> User
│     └─ … [children omitted]
└─ Definitions
   ├─ User object
   │  └─ … [children omitted]
   └─ Name string
"
)]
fn tree_layout_excludes_wrapper_from_maximum_depth(#[case] depth: &str, #[case] expected: &str) {
    command()
        .args([
            "--layout",
            "tree",
            "-r",
            "-D",
            "referenced",
            "-L",
            depth,
            "-",
        ])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(expected.to_owned())
        .stderr("");
}

#[test]
fn tree_layout_colors_the_wrapper_and_connectors() {
    let assert = command()
        .env_remove("NO_COLOR")
        .args(["--layout", "tree", "--color", "always"])
        .arg(fixture("render", "scalar-string", "json"))
        .assert()
        .success()
        .stderr("");
    let output =
        String::from_utf8(assert.get_output().stdout.clone()).expect("tree output should be UTF-8");

    assert_eq!(
        console::strip_ansi_codes(&output),
        "Schema\n└─ root string\n"
    );
    assert!(output.contains("\x1b[33mSchema\x1b[0m"));
    assert!(output.contains("\x1b[34m└─ \x1b[0m"));
}

#[test]
fn rejects_unknown_layout() {
    command()
        .args(["--layout", "unknown", "-"])
        .assert()
        .failure()
        .stdout("")
        .stderr(
            predicate::str::contains("invalid value 'unknown'")
                .and(predicate::str::contains("sections, tree")),
        );
}
