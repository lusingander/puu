use std::fs;

use predicates::prelude::*;
use rstest::rstest;

use crate::support::{command, fixture};

#[rustfmt::skip]
#[rstest]
#[case("zero")]
#[case("one")]
fn limits_display_depth(#[case] name: &str) {
    let expected =
        fs::read(fixture("depth", name, "txt")).expect("expected output should exist");
    let max_depth = if name == "zero" { "0" } else { "1" };

    command()
        .args(["--max-depth", max_depth])
        .arg(fixture("depth", name, "json"))
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
}

#[test]
fn leaves_a_leaf_unchanged() {
    command()
        .args(["--max-depth", "0"])
        .arg(fixture("render", "scalar-string", "json"))
        .assert()
        .success()
        .stdout("root string\n")
        .stderr("");
}

#[rustfmt::skip]
#[rstest]
#[case("-1", "unexpected argument")]
#[case("not-a-number", "invalid value")]
fn rejects_invalid_maximum_depths(#[case] value: &str, #[case] message: &str) {
    command()
        .args(["--max-depth", value])
        .arg(fixture("render", "scalar-string", "json"))
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(message));
}

#[test]
fn colors_the_omission_marker() {
    let assert = command()
        .env_remove("NO_COLOR")
        .args(["--color", "always", "--max-depth", "0"])
        .arg(fixture("depth", "zero", "json"))
        .assert()
        .success();

    let output =
        String::from_utf8(assert.get_output().stdout.clone()).expect("tree output should be UTF-8");
    assert!(output.contains("\x1b[31m…\x1b[0m"));
    assert!(output.contains("\x1b[31m[children omitted]\x1b[0m"));
}
