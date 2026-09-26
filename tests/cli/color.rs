use std::fs;

use crate::support::{command, fixture};

#[test]
fn colors_key_value_and_connector() {
    let assert = command()
        .env_remove("NO_COLOR")
        .env("CLICOLOR_FORCE", "1")
        .arg(fixture("render", "object-properties", "json"))
        .assert()
        .success();

    let colored =
        String::from_utf8(assert.get_output().stdout.clone()).expect("tree should be UTF-8");
    let expected = fs::read_to_string(fixture("render", "object-properties", "txt"))
        .expect("expected output should exist");
    assert_eq!(console::strip_ansi_codes(&colored), expected);
    assert!(colored.contains("\x1b[36mroot\x1b[0m"));
    assert!(colored.contains("\x1b[33mobject\x1b[0m"));
    assert!(colored.contains("\x1b[34m├─ \x1b[0m"));
}

#[test]
fn colors_constraints_annotations_and_markers() {
    let assert = command()
        .env_remove("NO_COLOR")
        .env("CLICOLOR_FORCE", "1")
        .arg(fixture("render", "string-constraints", "json"))
        .assert()
        .success();
    let colored =
        String::from_utf8(assert.get_output().stdout.clone()).expect("tree should be UTF-8");
    assert!(colored.contains("\x1b[32m[1..32 chars]\x1b[0m"));
    assert!(colored.contains("\x1b[35m[format: uuid]\x1b[0m"));

    let assert = command()
        .env_remove("NO_COLOR")
        .env("CLICOLOR_FORCE", "1")
        .arg(fixture("render", "object-properties", "json"))
        .assert()
        .success();
    let colored =
        String::from_utf8(assert.get_output().stdout.clone()).expect("tree should be UTF-8");
    assert!(colored.contains("\x1b[31m[required]\x1b[0m"));
}

#[test]
fn always_overrides_disabled_color_environment() {
    let assert = command()
        .env("NO_COLOR", "1")
        .args(["--color=always"])
        .arg(fixture("render", "scalar-string", "json"))
        .assert()
        .success();

    let output =
        String::from_utf8(assert.get_output().stdout.clone()).expect("tree output should be UTF-8");
    assert!(output.contains("\x1b[36mroot\x1b[0m"));
    assert!(output.contains("\x1b[33mstring\x1b[0m"));
}

#[test]
fn never_overrides_forced_color_environment() {
    command()
        .env_remove("NO_COLOR")
        .env("CLICOLOR_FORCE", "1")
        .args(["-c", "never"])
        .arg(fixture("render", "scalar-string", "json"))
        .assert()
        .success()
        .stdout("root string\n")
        .stderr("");
}
