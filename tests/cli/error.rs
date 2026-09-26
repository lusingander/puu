use std::{fs, path::PathBuf};

use predicates::prelude::*;
use rstest::rstest;

use crate::support::command;

#[rstest]
fn rejects_fixture_through_cli(#[files("tests/fixtures/error/*.json")] input: PathBuf) {
    let expected_message = fs::read_to_string(input.with_extension("error"))
        .expect("expected error message should exist");

    command()
        .arg(&input)
        .assert()
        .append_context("fixture", input.display().to_string())
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(expected_message.trim().to_owned()));
}
