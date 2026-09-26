use std::fs;

use predicates::prelude::*;
use rstest::rstest;

use crate::support::{command, fixture};

#[test]
fn reports_missing_path_and_unreadable_file() {
    command()
        .assert()
        .failure()
        .stderr(predicate::str::contains("required arguments"));

    command()
        .arg("tests/fixtures/does-not-exist.json")
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot read"));
}

#[test]
fn displays_help() {
    command()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: puu"))
        .stderr("");
}

#[test]
fn rejects_unknown_color_choice() {
    command()
        .args(["--color", "sometimes"])
        .arg(fixture("render", "scalar-string", "json"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'sometimes'"));
}

#[test]
fn displays_version() {
    let expected = format!("puu {}\n", env!("CARGO_PKG_VERSION"));

    for argument in ["-V", "--version"] {
        command()
            .arg(argument)
            .assert()
            .success()
            .stdout(expected.clone())
            .stderr("");
    }
}

#[test]
fn accepts_short_options() {
    command()
        .args(["-d", "7", "-v"])
        .arg(fixture("render", "explicit-dialect", "json"))
        .assert()
        .success()
        .stdout(
            "root string [$schema overridden: https://json-schema.org/draft/2020-12/schema -> Draft 7]\n",
        )
        .stderr("");
}

#[test]
fn verbose_displays_uninterpreted_keyword_values() {
    command()
        .arg("--verbose")
        .arg(fixture("render", "uninterpreted-keyword", "json"))
        .assert()
        .success()
        .stdout("root string [uninterpreted: x-example={\"note\":\"demo\"}]\n")
        .stderr("");
}

#[test]
fn verbose_displays_annotation_details() {
    let expected =
        fs::read(fixture("verbose", "annotations", "txt")).expect("expected output should exist");
    command()
        .arg("--verbose")
        .arg(fixture("render", "annotations", "json"))
        .assert()
        .success()
        .stdout(expected)
        .stderr("");
}

#[rustfmt::skip]
#[rstest]
#[case("7", "root string [$schema overridden: https://json-schema.org/draft/2020-12/schema -> Draft 7]\n")]
#[case("2019-09", "root string [$schema overridden: https://json-schema.org/draft/2020-12/schema -> 2019-09]\n")]
fn explicit_draft_overrides_declaration(#[case] draft: &str, #[case] expected: &str) {
    command()
        .args(["--draft", draft])
        .arg(fixture("render", "explicit-dialect", "json"))
        .assert()
        .success()
        .stdout(expected.to_owned())
        .stderr("");
}

#[test]
fn explicit_draft_accepts_unknown_declaration_and_reports_override() {
    command()
        .args(["--draft", "7"])
        .arg(fixture("error", "unsupported-dialect", "json"))
        .assert()
        .success()
        .stdout(
            "root string [$schema overridden: https://example.test/custom/schema -> Draft 7]\n",
        );
}

#[test]
fn matching_explicit_draft_does_not_report_override() {
    command()
        .args(["--draft", "7"])
        .arg(fixture("render", "draft-7-simple", "json"))
        .assert()
        .success()
        .stdout("root string\n");
}

#[test]
fn rejects_unsupported_draft_argument() {
    command()
        .args(["--draft", "4"])
        .arg(fixture("render", "scalar-string", "json"))
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("invalid value '4'").and(predicate::str::contains(
                "possible values: 2020-12, 2019-09, 7",
            )),
        );
}

#[rustfmt::skip]
#[rstest]
#[case("2019-09", "root object [if \"modernTrigger\" exists, requires \"modernTarget\"] [uninterpreted: dependencies]\n└─ when property \"modernSchema\" exists <type unspecified> [required names: \"modernField\"]\n")]
#[case("7", "root object [if \"legacyTrigger\" exists, requires \"legacyTarget\"] [uninterpreted: dependentRequired, dependentSchemas]\n└─ when property \"legacySchema\" exists <type unspecified> [required names: \"legacyField\"]\n")]
fn explicit_draft_selects_dependency_keyword(#[case] draft: &str, #[case] expected: &str) {
    command()
        .args(["--draft", draft])
        .arg(fixture("render", "dependency-draft-selection", "json"))
        .assert()
        .success()
        .stdout(expected.to_owned())
        .stderr("");
}

#[test]
fn explicit_draft_selects_contains_count_rules() {
    command()
        .args(["--draft", "2020-12"])
        .arg(fixture("render", "draft-7-contains", "json"))
        .assert()
        .success()
        .stdout(
            "root array [$schema overridden: http://json-schema.org/draft-07/schema# -> 2020-12]\n└─ contains integer [2..4 matches]\n",
        )
        .stderr("");
}

#[test]
fn forced_draft_applies_to_embedded_resources() {
    let input = fixture("render", "mixed-dialects", "json");
    command()
        .args(["--draft", "2020-12"])
        .arg(&input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid 'items' value"));

    command()
        .args(["--draft", "7"])
        .arg(&input)
        .assert()
        .success()
        .stdout(
            predicate::str::contains(
                "$schema overridden: https://json-schema.org/draft/2020-12/schema -> Draft 7",
            )
            .and(predicate::str::contains(
                "current array [items: forbidden] [uninterpreted: prefixItems]",
            ))
            .and(predicate::str::contains(
                "inherited array [resource: https://example.test/inherited] [items: forbidden] [uninterpreted: prefixItems]",
            )),
        );
}
