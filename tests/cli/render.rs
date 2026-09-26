use std::{fs, path::PathBuf};

use rstest::rstest;

use crate::support::command;

#[rstest]
fn renders_fixture_through_cli(#[files("tests/fixtures/render/*.json")] input: PathBuf) {
    let expected = fs::read(input.with_extension("txt")).expect("expected output should exist");

    command()
        .arg(&input)
        .assert()
        .append_context("fixture", input.display().to_string())
        .success()
        .stdout(expected)
        .stderr("");
}

#[test]
fn escapes_terminal_control_characters_from_schema_values() {
    command()
        .args(["--color=never", "-"])
        .write_stdin(
            r#"{"type":"string","format":"line\n\u001b]8;;https://example.invalid\u0007link"}"#,
        )
        .assert()
        .success()
        .stdout("root string [format: line\\n\\u{1b}]8;;https://example.invalid\\u{7}link]\n")
        .stderr("");
}
