use predicates::prelude::*;

use crate::support::command;

#[test]
fn reads_schema_from_standard_input() {
    command()
        .arg("-")
        .write_stdin(r#"{"type":"string"}"#)
        .assert()
        .success()
        .stdout("root string\n")
        .stderr("");
}

#[test]
fn reports_invalid_schema_from_standard_input() {
    command()
        .arg("-")
        .write_stdin("{")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("invalid JSON"));
}
