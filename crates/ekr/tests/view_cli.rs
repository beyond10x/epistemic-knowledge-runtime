//! `story:ekr-view-server`: `ekr view` serves a read-only viewer for an existing store on
//! 127.0.0.1, driven end to end through the real binary on both providers.
//!
//! Each case seeds a store from the retraction fixtures and commits one transaction through fresh
//! `ekr` processes, so the store holds revisions 0 and 1 and two retained evidence payloads. It then
//! starts `ekr view --port 0`, reads the one JSON line it prints, and speaks HTTP/1.1 to the URL
//! over a plain `TcpStream`. The expected projection bytes are what `ekr_views::project` renders
//! through the public kernel `Runtime` opened on the same store, and the store's head and published
//! event count are read the same way before the server starts and after it is stopped.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use ekr::host::CliHostConfigurationV1;
use ekr_core::{ContentHash, RevisionNumber};
use ekr_kernel::Runtime;
use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const T_ALICE: &str = "00000000-0000-4000-8000-000000000601";
/// A seeded evidence item whose payload the seed retains, and that payload's content hash.
const EVIDENCE: &str = "00000000-0000-4000-8000-000000000401";
const EVIDENCE_HASH: &str = "2f954f8f77731e11a4dca21e0bb6c566f719aae4116838f3095f60649e5429db";
/// An evidence id the store has never held.
const UNKNOWN_EVIDENCE: &str = "00000000-0000-4000-8000-000000000499";

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
}

fn fixture(name: &str) -> PathBuf {
    manifest_dir().join("tests/fixtures/retraction").join(name)
}

/// One provider in its own directory.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn new(backend: &'static str) -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        }
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            fixture("host.json").display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
            "--backend".to_owned(),
            self.backend.to_owned(),
        ];
        args.extend(verb.iter().map(|arg| (*arg).to_owned()));
        args
    }

    fn run(&self, verb: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(self.args(verb))
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: stderr {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// Revision 0 from the seed, revision 1 from one committed transaction.
    fn seeded_with_two_revisions(backend: &'static str) -> Self {
        let world = Self::new(backend);
        let seed = fixture("seed.yaml").display().to_string();
        assert_eq!(world.ok(&["seed", &seed])["result"]["revision"], 0);
        let propose = fixture("propose-alice.yaml").display().to_string();
        assert_eq!(world.ok(&["propose", &propose])["transaction_id"], T_ALICE);
        assert_eq!(
            world.ok(&["validate", T_ALICE, "--against", "0"])["kind"],
            "Validated"
        );
        assert_eq!(world.ok(&["commit", T_ALICE])["result"]["revision"], 1);
        world
    }

    /// The store, observed through the public kernel facade under the same host, opening an
    /// existing store only.
    fn runtime(&self) -> Runtime {
        let host = CliHostConfigurationV1::from_json(&std::fs::read(fixture("host.json")).unwrap())
            .unwrap();
        match self.backend {
            "file" => {
                Runtime::file_existing(&self.store(), &host.tenant, host.context, host.authority)
            }
            _ => {
                Runtime::sqlite_existing(&self.store(), &host.tenant, host.context, host.authority)
            }
        }
        .unwrap()
    }

    /// The head revision and the number of events the provider log has published.
    fn observed(&self) -> (u64, usize) {
        let runtime = self.runtime();
        let head = runtime.head().unwrap().unwrap().revision.get();
        (head, runtime.published_events().unwrap().len())
    }

    /// Starts `ekr view --port 0` and reads the URL it prints.
    fn serve(&self) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(self.args(&["view", "--port", "0"]))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let mut server = Server { child, url: None };
        let printed: Value = serde_json::from_str(&line).unwrap_or_else(|error| {
            let mut stderr = String::new();
            if let Some(mut pipe) = server.child.stderr.take() {
                server.child.kill().ok();
                pipe.read_to_string(&mut stderr).ok();
            }
            panic!(
                "{}: `ekr view` printed no JSON line ({error}): {line:?}; stderr {stderr}",
                self.backend
            )
        });
        let url = printed["url"].as_str().unwrap().to_owned();
        assert!(
            url.starts_with("http://127.0.0.1:") && url.ends_with('/'),
            "{}: the URL is on the loopback address: {url}",
            self.backend
        );
        server.url = Some(url);
        server
    }
}

/// A running `ekr view`, killed when dropped so a failed assertion leaves no server behind.
struct Server {
    child: Child,
    url: Option<String>,
}

impl Server {
    fn address(&self) -> String {
        self.url
            .as_deref()
            .unwrap()
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned()
    }

    fn request(&self, method: &str, path: &str) -> Response {
        let address = self.address();
        self.raw(&format!(
            "{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n"
        ))
    }

    /// Sends `text` as the whole request on a fresh connection and reads the answer.
    fn raw(&self, text: &str) -> Response {
        let mut stream = TcpStream::connect(self.address()).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        stream.write_all(text.as_bytes()).unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        Response::parse(&raw)
    }

    fn alive(&mut self) -> bool {
        self.child.try_wait().unwrap().is_none()
    }

    fn get(&self, path: &str) -> Response {
        self.request("GET", path)
    }

    fn stop(mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    fn parse(raw: &[u8]) -> Self {
        let split = raw
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap_or_else(|| panic!("no header end in {:?}", String::from_utf8_lossy(raw)));
        let head = std::str::from_utf8(&raw[..split]).unwrap();
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let headers: Vec<(String, String)> = lines
            .map(|line| {
                let (name, value) = line.split_once(':').unwrap();
                (name.trim().to_ascii_lowercase(), value.trim().to_owned())
            })
            .collect();
        let mut body = raw[split + 4..].to_vec();
        if headers
            .iter()
            .any(|(name, value)| name == "transfer-encoding" && value.contains("chunked"))
        {
            body = dechunk(&body);
        }
        Self {
            status,
            headers,
            body,
        }
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Every response: no cookie, no CORS allowance, never sniffed.
    fn assert_plain(&self, what: &str) {
        for (name, value) in &self.headers {
            assert!(
                name != "set-cookie" && !name.starts_with("access-control-"),
                "{what}: {name}: {value}"
            );
        }
        assert_eq!(
            self.header("x-content-type-options"),
            Some("nosniff"),
            "{what}"
        );
    }
}

fn dechunk(mut body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let end = body.windows(2).position(|w| w == b"\r\n").unwrap();
        let size_text = std::str::from_utf8(&body[..end]).unwrap();
        let size = usize::from_str_radix(size_text.split(';').next().unwrap().trim(), 16).unwrap();
        body = &body[end + 2..];
        if size == 0 {
            return out;
        }
        out.extend_from_slice(&body[..size]);
        body = &body[size + 2..];
    }
}

fn embedded_page() -> Vec<u8> {
    std::fs::read(manifest_dir().join("src/cli/viewer/index.html")).unwrap()
}

#[test]
fn ekr_view_serves_the_page_the_projection_and_evidence_and_writes_nothing_on_both_providers() {
    for backend in BACKENDS {
        let world = World::seeded_with_two_revisions(backend);
        let (at_head, at_zero, retained) = {
            let runtime = world.runtime();
            let hash: ContentHash = EVIDENCE_HASH.parse().unwrap();
            (
                ekr_views::project(&runtime, None).unwrap().bytes,
                ekr_views::project(&runtime, Some(RevisionNumber::new(0)))
                    .unwrap()
                    .bytes,
                runtime.content(&hash).unwrap().unwrap(),
            )
        };
        assert_ne!(at_head, at_zero, "{backend}: two revisions render apart");
        let before = world.observed();
        assert_eq!(before.0, 1, "{backend}");

        let server = world.serve();

        let page = server.get("/");
        assert_eq!(page.status, 200, "{backend} GET /");
        assert_eq!(page.body, embedded_page(), "{backend}: the embedded page");
        assert_eq!(
            page.header("content-type"),
            Some("text/html; charset=utf-8"),
            "{backend} GET /"
        );
        page.assert_plain("GET /");

        let head = server.get("/projection");
        assert_eq!(head.status, 200, "{backend} GET /projection");
        assert_eq!(head.body, at_head, "{backend}: the head projection's bytes");
        assert_eq!(
            head.header("content-type"),
            Some("application/json"),
            "{backend}"
        );
        head.assert_plain("GET /projection");

        let earlier = server.get("/projection?revision=0");
        assert_eq!(earlier.status, 200, "{backend} GET /projection?revision=0");
        assert_eq!(earlier.body, at_zero, "{backend}: revision 0's bytes");
        earlier.assert_plain("GET /projection?revision=0");

        let absent = server.get("/projection?revision=9");
        assert_eq!(absent.status, 404, "{backend}: an absent revision");
        assert!(
            String::from_utf8_lossy(&absent.body).contains("ekr.views.RevisionNotFound"),
            "{backend}: the refusal is named: {}",
            String::from_utf8_lossy(&absent.body)
        );
        absent.assert_plain("absent revision");

        let evidence = server.get(&format!("/evidence/{EVIDENCE}"));
        assert_eq!(evidence.status, 200, "{backend} GET /evidence/<id>");
        assert_eq!(evidence.body, retained, "{backend}: the retained bytes");
        assert_eq!(
            evidence.header("content-type"),
            Some("text/plain; charset=utf-8"),
            "{backend}"
        );
        evidence.assert_plain("GET /evidence/<id>");

        for path in [
            format!("/evidence/{UNKNOWN_EVIDENCE}"),
            "/evidence/not-an-id".to_owned(),
            "/elsewhere".to_owned(),
            "/projection/extra".to_owned(),
        ] {
            let missing = server.get(&path);
            assert_eq!(missing.status, 404, "{backend} GET {path}");
            missing.assert_plain(&path);
        }

        for (method, path) in [
            ("POST", "/"),
            ("POST", "/projection"),
            ("PUT", "/evidence/00000000-0000-4000-8000-000000000401"),
            ("DELETE", "/projection"),
        ] {
            let refused = server.request(method, path);
            assert_eq!(refused.status, 405, "{backend} {method} {path}");
            refused.assert_plain(&format!("{method} {path}"));
        }

        server.stop();
        assert_eq!(
            world.observed(),
            before,
            "{backend}: head revision and published event count"
        );
    }
}

/// A request whose `Host` is not this server's own loopback authority — another name, another
/// port, none, or two — is 421 and served nothing; `localhost:<port>` is served like the address.
#[test]
fn ekr_view_serves_only_a_request_whose_host_names_it() {
    let world = World::seeded_with_two_revisions("file");
    let server = world.serve();
    let address = server.address();
    let port = address.rsplit_once(':').unwrap().1.to_owned();
    let other_port = if port == "1" { "2" } else { "1" };
    for (path, hosts) in [
        ("/projection", vec!["rebound.example:80".to_owned()]),
        ("/projection", vec![format!("rebound.example:{port}")]),
        (
            &format!("/evidence/{EVIDENCE}")[..],
            vec![format!("127.0.0.1:{other_port}")],
        ),
        ("/", vec!["127.0.0.1".to_owned()]),
        ("/projection", vec![]),
        (
            "/projection",
            vec![address.clone(), "rebound.example".to_owned()],
        ),
    ] {
        let lines: String = hosts
            .iter()
            .map(|host| format!("Host: {host}\r\n"))
            .collect();
        let refused = server.raw(&format!(
            "GET {path} HTTP/1.1\r\n{lines}Connection: close\r\n\r\n"
        ));
        assert_eq!(refused.status, 421, "GET {path} with Host {hosts:?}");
        assert!(
            String::from_utf8_lossy(&refused.body).starts_with("misdirected-request"),
            "GET {path} with Host {hosts:?}: served {:?}",
            String::from_utf8_lossy(&refused.body)
        );
        refused.assert_plain(&format!("Host {hosts:?}"));
    }
    let local = server.raw(&format!(
        "GET /projection HTTP/1.1\r\nHost: localhost:{port}\r\nConnection: close\r\n\r\n"
    ));
    assert_eq!(local.status, 200, "Host: localhost:{port}");
    server.stop();
}

/// A `GET` that announces a body is 413, and no announced body — stalled, or of any claimed size up
/// to `usize::MAX` — stops the server answering the next client or ends its process.
#[test]
fn ekr_view_refuses_a_body_and_survives_any_announced_length() {
    let world = World::seeded_with_two_revisions("file");
    let mut server = world.serve();
    let address = server.address();

    let mut stalled = TcpStream::connect(&address).unwrap();
    write!(
        stalled,
        "GET / HTTP/1.1\r\nHost: {address}\r\nContent-Length: 4096\r\n\r\n"
    )
    .unwrap();
    assert_eq!(
        server.get("/").status,
        200,
        "a stalled body stalls nobody else"
    );

    // Up to 1024 bytes the library reads the body before the viewer sees the request and drops a
    // connection that closes short of it; up to 1 MiB the viewer answers 413; above that the
    // request is held unanswered rather than dropped, because dropping it allocates the whole
    // announced length and a failed allocation aborts (1 << 50 did, before it was held).
    const ANSWERED: u64 = 1 << 20;
    for length in [
        1_u64,
        1025,
        ANSWERED,
        ANSWERED + 1,
        1 << 40,
        1 << 50,
        u64::try_from(isize::MAX).unwrap(),
        u64::MAX,
    ] {
        let mut stream = TcpStream::connect(&address).unwrap();
        let wait = if length > ANSWERED { 2 } else { 60 };
        stream
            .set_read_timeout(Some(Duration::from_secs(wait)))
            .unwrap();
        write!(
            stream,
            "GET /projection HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Length: {length}\r\n\r\n"
        )
        .unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).ok();
        assert!(
            length <= 1024 || length > ANSWERED || !raw.is_empty(),
            "Content-Length {length}: no answer"
        );
        assert!(
            length <= ANSWERED || raw.is_empty(),
            "Content-Length {length}: answered {:?}",
            String::from_utf8_lossy(&raw)
        );
        if !raw.is_empty() {
            let refused = Response::parse(&raw);
            assert_eq!(refused.status, 413, "Content-Length {length}");
            refused.assert_plain(&format!("Content-Length {length}"));
        }
        std::thread::sleep(Duration::from_millis(200));
        assert!(server.alive(), "Content-Length {length} ended the server");
        assert_eq!(
            server.get("/").status,
            200,
            "the next client after Content-Length {length}"
        );
    }
    drop(stalled);
    server.stop();
}

#[test]
fn ekr_view_opens_an_existing_store_only() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let output = world.run(&["view", "--port", "0"]);
        assert_eq!(output.status.code(), Some(1), "{backend}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("store-not-found"), "{backend}: {stderr}");
        assert!(output.stdout.is_empty(), "{backend}");
        assert!(!world.store().exists(), "{backend}: nothing was created");
    }
}

#[test]
fn ekr_view_has_no_option_that_binds_another_address() {
    let world = World::new("file");
    for flag in ["--bind", "--address", "--listen", "--addr"] {
        let output = world.run(&["view", flag, "0.0.0.0"]);
        assert_eq!(output.status.code(), Some(2), "{flag}");
    }
    let help = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(["view", "--help"])
        .output()
        .unwrap();
    let help = String::from_utf8_lossy(&help.stdout);
    assert!(help.contains("127.0.0.1"), "{help}");
    let own: Vec<&str> = help
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("--"))
        .filter_map(|line| line.split_whitespace().next())
        .filter(|flag| {
            !matches!(
                *flag,
                "--host" | "--store" | "--backend" | "--full-replay" | "--help" | "--version"
            )
        })
        .collect();
    assert_eq!(own, ["--port"], "{help}");
}
