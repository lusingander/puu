use std::{
    io::Write,
    process::{Command, Stdio},
};

use assert_cmd::cargo::CommandCargoExt;

#[test]
fn exits_successfully_when_output_pipe_is_closed() {
    let mut child = Command::cargo_bin("puu")
        .expect("binary should exist")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("command should start");

    drop(
        child
            .stdout
            .take()
            .expect("standard output should be piped"),
    );
    let mut stdin = child.stdin.take().expect("standard input should be piped");
    stdin
        .write_all(br#"{"type":"string"}"#)
        .expect("schema should be written");
    drop(stdin);

    let output = child.wait_with_output().expect("command should finish");
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
}
