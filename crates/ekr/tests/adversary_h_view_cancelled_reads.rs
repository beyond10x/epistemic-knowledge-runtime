//! Adversary pass, unit H (issue #96, story:view-historical-changes-and-cancelled-reads): which
//! clients the store thread of `ekr view` takes as gone.
//!
//! The unit drops a queued job, before it starts, when a non-blocking peek of its connection
//! finds the end of the stream (`crates/ekr/src/cli/http.rs`, `waiting`). A client that sends its
//! request and then closes only its sending half (`shutdown(SHUT_WR)`, as `nc -N` and
//! `curl --data-binary @- ...` style pipes do) has not gone: it still reads the answer. Before the
//! unit it was answered; after it, its read is dropped even on an idle server, and it is told
//! `503 request deadline exceeded` at once.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
/// A hang guard only: no assertion reads a duration.
const GUARD: Duration = Duration::from_secs(60);

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
    .join("tests/fixtures/retraction")
    .join(name)
}

struct Server(Child, String);

impl Drop for Server {
    fn drop(&mut self) {
        self.0.kill().ok();
        self.0.wait().ok();
    }
}

fn args(directory: &std::path::Path, backend: &str, verb: &[&str]) -> Vec<String> {
    let store = match backend {
        "file" => directory.join("store"),
        _ => directory.join("state.db"),
    };
    let mut args = vec![
        "--host".to_owned(),
        fixture("host.json").display().to_string(),
        "--store".to_owned(),
        store.display().to_string(),
        "--backend".to_owned(),
        backend.to_owned(),
    ];
    args.extend(verb.iter().map(|arg| (*arg).to_owned()));
    args
}

fn seeded_server(directory: &std::path::Path, backend: &str) -> Server {
    let seed = fixture("seed.yaml").display().to_string();
    let output = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(args(directory, backend, &["seed", &seed]))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{backend} seed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(args(directory, backend, &["view", "--port", "0"]))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let printed: Value = serde_json::from_str(&line).expect("ekr view prints its URL");
    let address = printed["url"]
        .as_str()
        .unwrap()
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_owned();
    Server(child, address)
}

/// Sends a GET of `path`, closing the sending half after it when `half_close`, and reads the
/// whole answer.
fn get(server: &Server, path: &str, half_close: bool) -> String {
    let mut stream = TcpStream::connect(&server.1).unwrap();
    stream.set_read_timeout(Some(GUARD)).unwrap();
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        server.1
    )
    .unwrap();
    if half_close {
        stream.shutdown(Shutdown::Write).unwrap();
    }
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .unwrap_or_else(|error| panic!("hang guard: GET {path} got no answer: {error}"));
    String::from_utf8_lossy(&raw).into_owned()
}

/// A client that half-closes after its request still waits for the answer, and gets it as a
/// client that does not: on an idle server, nothing is queued ahead of it.
#[test]
fn adv_h_a_client_that_closes_only_its_sending_half_is_answered() {
    for backend in BACKENDS {
        let directory = tempfile::tempdir().unwrap();
        let server = seeded_server(directory.path(), backend);
        for path in ["/changes?since_revision=0", "/overview"] {
            let whole = get(&server, path, false);
            assert!(
                whole.starts_with("HTTP/1.1 200 "),
                "{backend} GET {path}, connection open: {whole}"
            );
            for attempt in 0..3 {
                let half = get(&server, path, true);
                assert!(
                    half.starts_with("HTTP/1.1 200 "),
                    "{backend} GET {path}, sending half closed after the request (attempt \
                     {attempt}): the client still reads, and was answered {:?}",
                    half.lines().next().unwrap_or_default()
                );
            }
        }
    }
}
