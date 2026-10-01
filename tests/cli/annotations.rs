use predicates::prelude::*;

use crate::support::{command, fixture};

#[test]
fn excludes_annotations_with_long_and_short_options() {
    let expected = "root object\n├─ id string\n├─ payload string\n└─ detached string\n";

    for option in ["--exclude-annotations", "-a"] {
        command()
            .arg(option)
            .arg(fixture("render", "annotations", "json"))
            .assert()
            .success()
            .stdout(expected)
            .stderr("");
    }
}

#[test]
fn verbose_keeps_metadata_while_excluding_annotations() {
    command()
        .args(["--verbose", "--exclude-annotations"])
        .arg(fixture("render", "annotations", "json"))
        .assert()
        .success()
        .stdout(
            predicate::str::contains("[$vocabulary:")
                .and(predicate::str::contains("[title:").not())
                .and(predicate::str::contains("[$comment:").not())
                .and(predicate::str::contains("content schema").not()),
        )
        .stderr("");
}

#[test]
fn excludes_annotations_from_expanded_references() {
    command()
        .args(["-a", "-r", "-D", "none", "-"])
        .write_stdin(
            r##"{
                "properties": {
                    "value": {
                        "$ref": "#/$defs/Value",
                        "description": "Reference annotation"
                    }
                },
                "$defs": {
                    "Value": {
                        "type": "string",
                        "title": "Expanded annotation"
                    }
                }
            }"##,
        )
        .assert()
        .success()
        .stdout(
            "root <type unspecified>\n└─ value -> Value\n   └─ Value string [expanded from $ref]\n",
        )
        .stderr("");
}

#[test]
fn excludes_annotation_only_references_from_referenced_definitions() {
    command()
        .args(["-a", "-D", "referenced", "-"])
        .write_stdin(
            r##"{
                "properties": {
                    "visible": {"$ref": "#/$defs/Visible"}
                },
                "contentSchema": {"$ref": "#/$defs/AnnotationOnly"},
                "$defs": {
                    "Visible": {"type": "integer"},
                    "AnnotationOnly": {"type": "string"}
                }
            }"##,
        )
        .assert()
        .success()
        .stdout(
            "root <type unspecified>\n└─ visible -> Visible\n\nDefinitions\n└─ Visible integer\n",
        )
        .stderr("");
}
