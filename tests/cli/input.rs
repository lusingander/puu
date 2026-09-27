use std::{io::Write, net::TcpListener, thread};

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

#[test]
fn reads_schema_from_url() {
    let (url, server) = serve("200 OK", r#"{"type":"string"}"#);

    command()
        .arg(url)
        .assert()
        .success()
        .stdout("root string\n")
        .stderr("");

    server.join().expect("server should exit cleanly");
}

#[test]
fn reports_unsuccessful_url_response() {
    let (url, server) = serve("404 Not Found", "not found");

    command().arg(&url).assert().failure().stdout("").stderr(
        predicate::str::contains(format!("cannot read {url}")).and(predicate::str::contains("404")),
    );

    server.join().expect("server should exit cleanly");
}

fn serve(status: &str, body: &str) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("test server should bind");
    let address = listener
        .local_addr()
        .expect("test server should have an address");
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let server = thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("test server should accept a request");
        stream
            .write_all(response.as_bytes())
            .expect("test server should write the response");
    });

    (format!("http://{address}/schema.json"), server)
}
