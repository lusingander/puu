use std::fs;

use predicates::prelude::*;
use rstest::rstest;

use crate::support::{command, fixture};

#[test]
fn renders_selected_schema_and_global_definitions() {
    let expected =
        fs::read(fixture("pointer", "selection", "txt")).expect("expected output should exist");

    command()
        .args(["--pointer", "#/$defs/User"])
        .arg(fixture("pointer", "selection", "json"))
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
}

#[rustfmt::skip]
#[rstest]
#[case("#", "root <type unspecified> [location: #]\n")]
#[case("/properties/customer", "customer -> User [location: #/properties/customer]\n")]
#[case("#/%24defs/a~1b~0c", "a/b~c integer [location: #/$defs/a~1b~0c]\n")]
#[case("#/properties/caf%C3%A9", "café string [location: #/properties/café]\n")]
#[case("#/properties/flag", "flag any [location: #/properties/flag]\n")]
#[case("#/items", "items boolean [location: #/items]\n")]
#[case("#/prefixItems/0", "[0] null [location: #/prefixItems/0]\n")]
#[case("#/properties/embedded/properties/value", "value number [location: #/properties/embedded/properties/value]\n")]
fn accepts_supported_pointer_forms(#[case] pointer: &str, #[case] first_line: &str) {
    command()
        .args(["--pointer", pointer])
        .arg(fixture("pointer", "selection", "json"))
        .assert()
        .success()
        .stdout(predicate::str::starts_with(first_line))
        .stderr("");
}

#[test]
fn selects_legacy_definitions() {
    command()
        .args(["--pointer", "#/definitions/Address"])
        .arg(fixture("render", "draft-7-definitions", "json"))
        .assert()
        .success()
        .stdout(predicate::str::starts_with(
            "Address object [location: #/definitions/Address]\n",
        ))
        .stderr("");
}

#[rustfmt::skip]
#[rstest]
#[case("#/properties/customer/$ref", "does not identify a schema")]
#[case("#/properties/missing", "schema pointer not found")]
#[case("#User", "invalid schema pointer")]
fn reports_pointer_errors(#[case] pointer: &str, #[case] message: &str) {
    command()
        .args(["--pointer", pointer])
        .arg(fixture("pointer", "selection", "json"))
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(message).and(predicate::str::contains(pointer)));
}
