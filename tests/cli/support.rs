use std::path::PathBuf;

use assert_cmd::Command;

pub(crate) fn fixture(group: &str, name: &str, extension: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(group)
        .join(format!("{name}.{extension}"))
}

pub(crate) fn command() -> Command {
    let mut command = Command::cargo_bin("puu").expect("binary should exist");
    command.env("CLICOLOR", "0").env_remove("CLICOLOR_FORCE");
    command
}
