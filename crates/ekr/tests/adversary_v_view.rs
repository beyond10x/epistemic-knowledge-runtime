//! Adversary cases against `story:ekr-view-server` (`ekr view`).
//!
//! Each case seeds a store from the retraction fixtures, starts `ekr view --port 0` through the
//! real binary and speaks raw HTTP/1.1 to it over a `TcpStream`, so it can send what a browser
//! would not: a body that never arrives, a `Content-Length` no body could have, a foreign `Host`,
//! an HTTP version the server does not speak.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
    .join("tests/fixtures/retraction")
    .join(name)
}

struct Served {
    _directory: tempfile::TempDir,
    child: Child,
    address: String,
}

impl Drop for Served {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

fn args(directory: &tempfile::TempDir, verb: &[&str]) -> Vec<String> {
    let mut args = vec![
        "--host".to_owned(),
        fixture("host.json").display().to_string(),
        "--store".to_owned(),
        directory.path().join("store").display().to_string(),
        "--backend".to_owned(),
        "file".to_owned(),
    ];
    args.extend(verb.iter().map(|arg| (*arg).to_owned()));
    args
}

/// A seeded file store with `ekr view --port 0` serving it.
fn serve() -> Served {
    let directory = tempfile::tempdir().unwrap();
    let seed = fixture("seed.yaml").display().to_string();
    let seeded = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(args(&directory, &["seed", &seed]))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(
        seeded.status.code(),
        Some(0),
        "seed: {}",
        String::from_utf8_lossy(&seeded.stderr)
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(args(&directory, &["view", "--port", "0"]))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    BufReader::new(child.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let printed: serde_json::Value = serde_json::from_str(&line).unwrap();
    let address = printed["url"]
        .as_str()
        .unwrap()
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_owned();
    Served {
        _directory: directory,
        child,
        address,
    }
}

impl Served {
    fn connect(&self) -> TcpStream {
        let stream = TcpStream::connect(&self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        stream
    }

    /// Sends `raw` on a fresh connection and reads until the server closes it or 10 s pass.
    fn exchange(&self, raw: &str) -> Result<Vec<u8>, std::io::Error> {
        let mut stream = self.connect();
        stream.write_all(raw.as_bytes())?;
        let mut out = Vec::new();
        stream.read_to_end(&mut out)?;
        Ok(out)
    }

    /// Sends `raw` and returns whatever arrives before the server closes or goes quiet for 2 s.
    fn exchange_partial(&self, raw: &str) -> Vec<u8> {
        let mut stream = self.connect();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream.write_all(raw.as_bytes()).unwrap();
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        while let Ok(n) = stream.read(&mut buf) {
            if n == 0 {
                break;
            }
            out.extend_from_slice(&buf[..n]);
        }
        out
    }

    fn plain_get(&self, path: &str) -> Result<Vec<u8>, std::io::Error> {
        let address = self.address.clone();
        self.exchange(&format!(
            "GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        ))
    }

    fn alive(&mut self) -> bool {
        self.child.try_wait().unwrap().is_none()
    }
}

fn status(raw: &[u8]) -> Option<u16> {
    std::str::from_utf8(raw)
        .ok()?
        .split("\r\n")
        .next()?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

fn head(raw: &[u8]) -> String {
    let end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).to_ascii_lowercase()
}

/// One client that announces a request body and never sends it must not stop the server from
/// answering every other client: the server is single-threaded, and `tiny_http` drains an
/// unread body (over 1024 bytes) when the request is dropped, on the thread that answers.
#[test]
fn a_body_that_never_arrives_does_not_hang_the_server_for_everyone_else() {
    let mut served = serve();
    let address = served.address.clone();
    let mut stalled = served.connect();
    write!(
        stalled,
        "POST / HTTP/1.1\r\nHost: {address}\r\nContent-Length: 4096\r\n\r\n"
    )
    .unwrap();
    // The stalled client keeps its connection open and sends nothing more.
    std::thread::sleep(Duration::from_millis(500));

    let other = served.plain_get("/");
    assert!(served.alive(), "the server process exited");
    let other = other.expect("a second client got no answer within 10 s");
    assert_eq!(status(&other), Some(200), "{}", head(&other));
    drop(stalled);
}

/// A `Content-Length` no body could have must not take the server down: the client closes its
/// write half at once, so there is nothing to wait for.
#[test]
fn a_huge_content_length_does_not_kill_the_server() {
    let mut served = serve();
    let address = served.address.clone();
    {
        let mut stream = served.connect();
        write!(
            stream,
            "GET / HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
            usize::MAX
        )
        .unwrap();
        stream.shutdown(Shutdown::Write).ok();
        let mut sink = Vec::new();
        stream.read_to_end(&mut sink).ok();
    }
    std::thread::sleep(Duration::from_millis(500));
    assert!(
        served.alive(),
        "one request with Content-Length {} ended the server process",
        usize::MAX
    );
    let after = served.plain_get("/").expect("an answer after it");
    assert_eq!(status(&after), Some(200), "{}", head(&after));
}

/// Loopback-only binding does not stop a web page reached through DNS rebinding: the browser
/// sends it to 127.0.0.1 as same-origin with its own `Host`. A server whose only protection is
/// the loopback address must not serve the store to a `Host` that is not that address.
#[test]
fn a_foreign_host_header_is_not_served_the_store() {
    let served = serve();
    let raw = served
        .exchange(
            "GET /projection HTTP/1.1\r\nHost: rebound.example:80\r\nConnection: close\r\n\r\n",
        )
        .unwrap();
    assert_ne!(
        status(&raw),
        Some(200),
        "the projection was served to Host: rebound.example: {}",
        head(&raw)
    );
}

/// The module doc and `docs/cli.md` promise `X-Content-Type-Options: nosniff` on every response
/// the viewer writes, and name the responses `tiny_http` writes itself, before the handler sees
/// the request (here the 400 for a header line without a colon), as outside that promise. Such a
/// response carries no nosniff, so it must carry nothing a browser could sniff: an empty body.
#[test]
fn a_response_tiny_http_writes_itself_is_empty_and_outside_the_nosniff_promise() {
    let served = serve();
    let address = served.address.clone();
    let raw = served.exchange_partial(&format!(
        "GET / HTTP/1.1\r\nHost: {address}\r\na header line without a colon\r\n\r\n"
    ));
    assert_eq!(status(&raw), Some(400), "{}", head(&raw));
    let end = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .expect("a complete header block");
    assert!(
        raw[end + 4..].is_empty() && head(&raw).contains("content-length: 0"),
        "nosniff is promised only on viewer-written responses, so a response tiny_http writes \
         itself must have an empty body; this one does not: {:?}",
        String::from_utf8_lossy(&raw)
    );
}
