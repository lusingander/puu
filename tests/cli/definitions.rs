use predicates::prelude::*;

use crate::support::command;

const INPUT: &str = r##"{
  "type": "object",
  "properties": {
    "part": { "$ref": "#/$defs/A/properties/value" }
  },
  "$defs": {
    "A": {
      "type": "object",
      "properties": {
        "value": { "$ref": "#/$defs/B" }
      }
    },
    "B": { "$ref": "#/$defs/A" },
    "Unused": { "type": "integer" }
  }
}"##;

const ALL: &str = "\
root object
└─ part -> #/$defs/A/properties/value

Definitions
├─ A object
│  └─ value -> B
├─ B -> A
└─ Unused integer
";

const REFERENCED: &str = "\
root object
└─ part -> #/$defs/A/properties/value

Definitions
├─ A object
│  └─ value -> B
└─ B -> A
";

const NONE: &str = "\
root object
└─ part -> #/$defs/A/properties/value
";

#[test]
fn defaults_to_all_definitions() {
    for arguments in [Vec::new(), vec!["--definitions", "all"]] {
        command()
            .args(arguments)
            .arg("-")
            .write_stdin(INPUT)
            .assert()
            .success()
            .stdout(ALL)
            .stderr("");
    }
}

#[test]
fn displays_transitively_referenced_definitions() {
    command()
        .args(["-D", "referenced", "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(REFERENCED)
        .stderr("");
}

#[test]
fn hides_definitions_without_hiding_references() {
    command()
        .args(["--definitions", "none", "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(NONE)
        .stderr("");
}

#[test]
fn scopes_referenced_definitions_to_pointer_selection() {
    command()
        .args(["-D", "referenced", "--pointer", "#/$defs/A", "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(
            "A object [location: #/$defs/A]\n\
             └─ value -> B\n\
             \n\
             Definitions\n\
             └─ B -> A\n",
        )
        .stderr("");
}

#[test]
fn rejects_unknown_definitions_mode() {
    command()
        .args(["--definitions", "sometimes", "-"])
        .write_stdin(INPUT)
        .assert()
        .failure()
        .stdout("")
        .stderr(
            predicate::str::contains("invalid value 'sometimes'")
                .and(predicate::str::contains("all, referenced, none")),
        );
}
