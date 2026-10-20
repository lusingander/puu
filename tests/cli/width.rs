use std::fs;

use predicates::prelude::*;
use rstest::rstest;

use crate::support::{command, fixture};

const INPUT: &str = r##"{
  "type": "object",
  "properties": {
    "customer": {
      "type": "object",
      "properties": { "name": { "type": "string" } }
    },
    "status": { "type": "string" }
  },
  "$defs": { "User": { "type": "string" } }
}"##;

#[rstest]
#[case("2", "..\n")]
#[case("10", "root str..\n")]
#[case("11", "root string\n")]
#[case("12", "root string\n")]
fn limits_lines_only_when_they_exceed_the_width(
    #[values("--max-width", "-W")] option: &str,
    #[case] width: &str,
    #[case] expected: &str,
) {
    command()
        .args([option, width])
        .arg(fixture("render", "scalar-string", "json"))
        .assert()
        .success()
        .stdout(expected.to_owned())
        .stderr("");
}

#[rstest]
#[case::sections(
    "sections",
    "\
root object
├─ custome..
│  └─ name..
└─ status ..

Definitions
└─ User st..
"
)]
#[case::tree(
    "tree",
    "\
Schema
├─ root ob..
│  ├─ cust..
│  │  └─ n..
│  └─ stat..
└─ Definit..
   └─ User..
"
)]
fn counts_indentation_and_connectors_in_both_layouts(#[case] layout: &str, #[case] expected: &str) {
    command()
        .args(["--max-width", "12", "--layout", layout, "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(expected.to_owned())
        .stderr("");
}

#[rstest]
#[case("sections", "..\n..\n..\n..\n\n..\n..\n")]
#[case("tree", "..\n..\n..\n..\n..\n..\n..\n")]
fn narrow_width_preserves_every_node_and_section(#[case] layout: &str, #[case] expected: &str) {
    command()
        .args(["--max-width", "2", "--layout", layout, "-"])
        .write_stdin(INPUT)
        .assert()
        .success()
        .stdout(expected.to_owned())
        .stderr("");
}

#[rstest]
#[case("8", "root o..\n└─ 日e\u{301}..\n")]
#[case("9", "root ob..\n└─ 日e\u{301}..\n")]
#[case("10", "root obj..\n└─ 日e\u{301}👨‍👩‍👧‍👦..\n")]
fn counts_unicode_columns_and_preserves_graphemes_with_or_without_color(
    #[case] width: &str,
    #[case] expected: &str,
    #[values("always", "never")] color: &str,
) {
    let assert = command()
        .env_remove("NO_COLOR")
        .args(["--max-width", width, "--color", color, "-"])
        .write_stdin(r#"{"type":"object","properties":{"日é👨‍👩‍👧‍👦abcdef":{"type":"string"}}}"#)
        .assert()
        .success()
        .stderr("");
    let output = std::str::from_utf8(&assert.get_output().stdout).expect("output should be UTF-8");

    assert_eq!(console::strip_ansi_codes(output), expected);
    if color == "always" {
        assert!(output.lines().all(|line| line.ends_with("\x1b[0m")));
    } else {
        assert!(!output.contains('\x1b'));
    }
}

#[test]
fn measures_control_characters_after_escaping() {
    command()
        .args(["--max-width", "32", "-"])
        .write_stdin(r#"{"type":"string","format":"abcdef\nxyz"}"#)
        .assert()
        .success()
        .stdout("root string [format: abcdef\\nx..\n")
        .stderr("");
}

#[test]
fn no_width_limit_is_applied_without_the_option() {
    let expected =
        fs::read(fixture("render", "annotations", "txt")).expect("expected output should exist");
    command()
        .env("COLUMNS", "2")
        .arg(fixture("render", "annotations", "json"))
        .assert()
        .success()
        .stdout(expected.clone())
        .stderr("");

    command()
        .args(["--max-width", &usize::MAX.to_string()])
        .arg(fixture("render", "annotations", "json"))
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
}

#[rstest]
#[case("0")]
#[case("1")]
#[case("-1")]
#[case("auto")]
#[case("unlimited")]
#[case("abc")]
#[case("18446744073709551616")]
fn rejects_invalid_widths(#[values("--max-width", "-W")] option: &str, #[case] width: &str) {
    command()
        .arg(format!("{option}={width}"))
        .arg("-")
        .assert()
        .failure()
        .stdout("")
        .stderr(
            predicate::str::contains("invalid value")
                .and(predicate::str::contains("integer of at least 2")),
        );
}

#[test]
fn requires_a_width_value() {
    command()
        .arg("--max-width")
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(
            "a value is required for '--max-width <N>'",
        ));
}
