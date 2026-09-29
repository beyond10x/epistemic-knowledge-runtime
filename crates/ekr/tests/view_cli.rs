//! `story:ekr-view-server`: `ekr view` serves a read-only viewer for an existing store on
//! 127.0.0.1, driven end to end through the real binary on both providers.
//!
//! Each case seeds a store from the retraction fixtures and commits one transaction through fresh
//! `ekr` processes, so the store holds revisions 0 and 1 and two retained evidence payloads. It then
//! starts `ekr view --port 0`, reads the one JSON line it prints, and speaks HTTP/1.1 to the URL
//! over a plain `TcpStream`. The expected projection bytes are what `ekr_views::project` renders
//! through the public kernel `Runtime` opened on the same store, and the store's head and published
//! event count are read the same way before the server starts and after it is stopped.

#[path = "support/rust_source.rs"]
mod rust_source;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use ekr::host::CliHostConfigurationV1;
use ekr_core::{ContentHash, RevisionNumber};
use ekr_kernel::Runtime;
use ekr_views::{ChangesRequest, Index, SinceKind};
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
            "/alt".to_owned(),
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

    // A client that sends half a head and stops holds only its own connection.
    let mut half = TcpStream::connect(&address).unwrap();
    write!(half, "GET / HTTP/1.1\r\nHost: {address}\r\n").unwrap();
    assert_eq!(
        server.get("/").status,
        200,
        "a stalled head stalls nobody else"
    );

    // Every announced length is answered 413 at once, and the body is never read or allocated.
    for length in [
        1_u64,
        1025,
        1 << 20,
        (1 << 20) + 1,
        1 << 40,
        1 << 50,
        u64::try_from(isize::MAX).unwrap(),
        u64::MAX,
    ] {
        let mut stream = TcpStream::connect(&address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        write!(
            stream,
            "GET /projection HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Length: {length}\r\n\r\n"
        )
        .unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        let refused = Response::parse(&raw);
        assert_eq!(refused.status, 413, "Content-Length {length}");
        refused.assert_plain(&format!("Content-Length {length}"));
        std::thread::sleep(Duration::from_millis(50));
        assert!(server.alive(), "Content-Length {length} ended the server");
        assert_eq!(
            server.get("/").status,
            200,
            "the next client after Content-Length {length}"
        );
    }
    // The half head times out into a 400 of the viewer's own.
    half.set_read_timeout(Some(Duration::from_secs(30)))
        .unwrap();
    let mut raw = Vec::new();
    half.read_to_end(&mut raw).unwrap();
    let timed_out = Response::parse(&raw);
    assert_eq!(timed_out.status, 400, "a head that never ends");
    timed_out.assert_plain("a head that never ends");
    drop(stalled);
    server.stop();
}

/// A head the server cannot parse, or one that fills 16 KiB without ending, is a 400 the viewer
/// writes, with the same headers as every other response, and the server keeps serving.
#[test]
fn ekr_view_answers_a_malformed_or_oversized_head_with_its_own_400() {
    let world = World::seeded_with_two_revisions("file");
    let mut server = world.serve();
    let address = server.address();
    let oversized = format!(
        "GET / HTTP/1.1\r\nHost: {address}\r\nX-Pad: {}",
        "a".repeat(16 * 1024)
    )[..16 * 1024]
        .to_owned();
    for (what, text) in [
        (
            "a header line without a colon",
            format!("GET / HTTP/1.1\r\nHost: {address}\r\nno colon here\r\n\r\n"),
        ),
        (
            "a request line that is not one",
            "\u{1}\u{2}\u{3}\r\n\r\n".to_owned(),
        ),
        ("16 KiB of head without its end", oversized),
    ] {
        let refused = server.raw(&text);
        assert_eq!(refused.status, 400, "{what}");
        assert!(
            String::from_utf8_lossy(&refused.body).starts_with("bad-request"),
            "{what}"
        );
        refused.assert_plain(what);
        assert_eq!(refused.header("connection"), Some("close"), "{what}");
        assert_eq!(refused.header("cache-control"), Some("no-store"), "{what}");
    }
    assert!(server.alive());
    assert_eq!(server.get("/").status, 200);
    server.stop();
}

/// With 64 connections in flight, the 65th is answered 503 `busy` at once, unread; once those 64
/// reach their 5 s deadline and are refused, the viewer serves again.
#[test]
fn ekr_view_answers_busy_over_64_connections_and_serves_again_after_their_deadline() {
    let world = World::seeded_with_two_revisions("file");
    let server = world.serve();
    let address = server.address();
    let held: Vec<TcpStream> = (0..64)
        .map(|_| {
            let mut stream = TcpStream::connect(&address).unwrap();
            write!(stream, "GET / HTTP/1.1\r\n").unwrap();
            stream
        })
        .collect();
    // Let the accept thread count all 64 before the next one arrives.
    std::thread::sleep(Duration::from_millis(500));
    let asked = std::time::Instant::now();
    let busy = server.get("/projection");
    assert_eq!(busy.status, 503, "the 65th connection");
    assert!(
        asked.elapsed() < Duration::from_secs(2),
        "answered at once: {:?}",
        asked.elapsed()
    );
    assert!(String::from_utf8_lossy(&busy.body).starts_with("busy"));
    busy.assert_plain("503 busy");
    assert_eq!(busy.header("connection"), Some("close"));
    assert_eq!(busy.header("cache-control"), Some("no-store"));
    std::thread::sleep(Duration::from_secs(6));
    assert_eq!(server.get("/projection").status, 200, "after the deadline");
    drop(held);
    server.stop();
}

/// `task:historical-projection-carries-the-head`: a commit made by another process while
/// `ekr view` serves moves `/head` and nothing any revision already answered. Revision 0's
/// projection, roles and overview are byte for byte what they were, and what the engine renders;
/// a request naming no revision reads the new head.
#[test]
fn ekr_view_serves_the_head_at_head_and_a_past_revision_unchanged_across_a_commit() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let seed = fixture("seed.yaml").display().to_string();
        assert_eq!(world.ok(&["seed", &seed])["result"]["revision"], 0);
        let server = world.serve();
        let head = |server: &Server| {
            let answer = server.get("/head");
            assert_eq!(answer.status, 200, "{backend}");
            assert_eq!(answer.header("content-type"), Some("application/json"));
            answer.body
        };
        assert_eq!(
            head(&server),
            b"{\"format\":\"ekr.view-head/1\",\"head\":0}"
        );
        let past = [
            "/projection?revision=0",
            "/roles?revision=0",
            "/overview?revision=0",
        ];
        let before: Vec<Vec<u8>> = past.iter().map(|path| server.get(path).body).collect();
        assert_eq!(
            before[0],
            ekr_views::project(&world.runtime(), Some(RevisionNumber::new(0)))
                .unwrap()
                .bytes,
            "{backend}"
        );

        let propose = fixture("propose-alice.yaml").display().to_string();
        assert_eq!(world.ok(&["propose", &propose])["transaction_id"], T_ALICE);
        assert_eq!(
            world.ok(&["validate", T_ALICE, "--against", "0"])["kind"],
            "Validated"
        );
        assert_eq!(world.ok(&["commit", T_ALICE])["result"]["revision"], 1);

        assert_eq!(
            head(&server),
            b"{\"format\":\"ekr.view-head/1\",\"head\":1}"
        );
        for (path, before) in past.iter().zip(&before) {
            let after = server.get(path);
            assert_eq!(after.status, 200, "{backend} GET {path}");
            assert!(
                &after.body == before,
                "{backend} GET {path}: the bytes changed after a commit\nbefore {}\nafter  {}",
                String::from_utf8_lossy(before),
                String::from_utf8_lossy(&after.body)
            );
        }
        let newest: Value = serde_json::from_slice(&server.get("/projection").body).unwrap();
        assert_eq!(newest["meta"]["revision"], 1, "{backend}");
        assert!(newest["meta"].get("head").is_none(), "{backend}");
        for query in ["?revision=0", "?x=1", "?"] {
            let refused = server.get(&format!("/head{query}"));
            let expected = if query == "?" { 200 } else { 400 };
            assert_eq!(refused.status, expected, "{backend} GET /head{query}");
        }
        server.stop();
    }
}

/// `GET /changes` serves the `ekr.graph-changes/1` bytes `ekr_views::Index::changes` returns for
/// a revision, a valid-time and a transaction-time since, and a request naming `at` serves the
/// same bytes before and after a later commit, while one naming none reads the new head.
#[test]
fn ekr_view_serves_the_changes_since_as_ekr_views_reads_them_across_a_commit() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let seed = fixture("seed.yaml").display().to_string();
        assert_eq!(world.ok(&["seed", &seed])["result"]["revision"], 0);
        let server = world.serve();
        let requests = [
            ("since_revision=0&at=0", SinceKind::Revision, 0, None, None),
            (
                "since_recorded=0&at=0",
                SinceKind::TransactionTime,
                0,
                None,
                None,
            ),
            (
                "since_recorded=0&at=0&limit=2&after=1",
                SinceKind::TransactionTime,
                0,
                Some(2),
                Some(1),
            ),
            ("since_valid=0&at=0", SinceKind::ValidTime, 0, None, None),
        ];
        let runtime = world.runtime();
        let index = Index::load(&runtime, Some(RevisionNumber::new(0))).unwrap();
        let mut before = Vec::new();
        for (query, kind, since, limit, after) in requests {
            let served = server.get(&format!("/changes?{query}"));
            assert_eq!(served.status, 200, "{backend} GET /changes?{query}");
            assert_eq!(served.header("content-type"), Some("application/json"));
            served.assert_plain(query);
            let expected = index
                .changes(
                    &runtime,
                    &ChangesRequest::new(kind, since, limit, after).unwrap(),
                )
                .unwrap()
                .bytes;
            assert!(
                served.body == expected,
                "{backend} GET /changes?{query}\nserved   {}\nexpected {}",
                String::from_utf8_lossy(&served.body),
                String::from_utf8_lossy(&expected)
            );
            before.push(served.body);
        }
        let seeded: Value = serde_json::from_slice(&before[1]).unwrap();
        assert!(seeded["meta"]["total"].as_u64().unwrap() > 0, "{seeded}");

        let propose = fixture("propose-alice.yaml").display().to_string();
        assert_eq!(world.ok(&["propose", &propose])["transaction_id"], T_ALICE);
        assert_eq!(
            world.ok(&["validate", T_ALICE, "--against", "0"])["kind"],
            "Validated"
        );
        assert_eq!(world.ok(&["commit", T_ALICE])["result"]["revision"], 1);

        for ((query, ..), before) in requests.iter().zip(&before) {
            let after = server.get(&format!("/changes?{query}"));
            assert!(
                &after.body == before,
                "{backend} GET /changes?{query}: the bytes changed after a commit"
            );
        }
        let newest: Value =
            serde_json::from_slice(&server.get("/changes?since_revision=0").body).unwrap();
        assert_eq!(newest["meta"]["revision"], 1, "{backend}: {newest}");
        assert!(newest["meta"].get("head").is_none(), "{backend}");
        assert!(
            newest["changes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|change| change["revision"] == 1),
            "{backend}: {newest}"
        );
        assert!(
            !newest["changes"].as_array().unwrap().is_empty(),
            "{newest}"
        );
        server.stop();
    }
}

/// `path` with `suffix` appended to its last component.
fn suffixed(path: &std::path::Path, suffix: &str) -> PathBuf {
    let mut text = path.as_os_str().to_owned();
    text.push(suffix);
    PathBuf::from(text)
}

/// Moves what is at `store` to `aside` and `from` into its place, each by rename, as a host
/// promoting a new store does. A SQLite database moves with its `-wal` and `-shm` files.
fn replace(store: &std::path::Path, aside: &std::path::Path, from: &std::path::Path) {
    for suffix in ["", "-wal", "-shm"] {
        let at = suffixed(store, suffix);
        if std::fs::symlink_metadata(&at).is_ok() {
            std::fs::rename(&at, suffixed(aside, suffix)).unwrap();
        }
    }
    for suffix in ["", "-wal", "-shm"] {
        let at = suffixed(from, suffix);
        if std::fs::symlink_metadata(&at).is_ok() {
            std::fs::rename(&at, suffixed(store, suffix)).unwrap();
        }
    }
}

/// `task:readers-reopen-a-replaced-store`: a running `ekr view` whose store is replaced by rename
/// answers the next request from the store now at its path — `GET /head`, and a revision both
/// stores hold with different content, so an index or a rendered answer kept from the replaced
/// store would show. A file that is not a store is refused by name, 503 `store-replaced`, on
/// every later request that reads the store; the page is still served.
#[test]
fn ekr_view_answers_from_a_store_replaced_by_rename() {
    for backend in BACKENDS {
        let world = World::seeded_with_two_revisions(backend);
        let server = world.serve();
        let head = |server: &Server| {
            let answer = server.get("/head");
            assert_eq!(answer.status, 200, "{backend}");
            answer.body
        };
        assert_eq!(
            head(&server),
            b"{\"format\":\"ekr.view-head/1\",\"head\":1}"
        );
        let paths = ["/overview?revision=0", "/projection?revision=0"];
        let replaced: Vec<Vec<u8>> = paths.iter().map(|path| server.get(path).body).collect();

        let replacement = World::new(backend);
        let seed = fixture("seed-different.yaml").display().to_string();
        assert_eq!(replacement.ok(&["seed", &seed])["result"]["revision"], 0);
        let aside = world.directory.path().join("replaced");
        replace(&world.store(), &aside, &replacement.store());

        assert_eq!(
            head(&server),
            b"{\"format\":\"ekr.view-head/1\",\"head\":0}",
            "{backend}: the head of the store now at the path"
        );
        let runtime = world.runtime();
        let expected = [
            Index::load(&runtime, Some(RevisionNumber::new(0)))
                .unwrap()
                .overview(&ekr_views::OverviewRequest::new(None).unwrap())
                .unwrap()
                .bytes,
            ekr_views::project(&runtime, Some(RevisionNumber::new(0)))
                .unwrap()
                .bytes,
        ];
        drop(runtime);
        for ((path, before), expected) in paths.iter().zip(&replaced).zip(&expected) {
            let after = server.get(path);
            assert_eq!(after.status, 200, "{backend} GET {path}");
            assert!(
                &after.body != before,
                "{backend} GET {path}: the replaced store's answer"
            );
            assert!(
                &after.body == expected,
                "{backend} GET {path}: not the replacement's revision 0"
            );
        }

        let junk = world.directory.path().join("junk");
        std::fs::write(&junk, "not a store\n").unwrap();
        let second = world.directory.path().join("second");
        replace(&world.store(), &second, &junk);
        for _ in 0..2 {
            for path in ["/head", "/overview", "/projection?revision=0"] {
                let refused = server.get(path);
                assert_eq!(refused.status, 503, "{backend} GET {path}");
                let body: Value = serde_json::from_slice(&refused.body).unwrap();
                assert_eq!(body["refusal"], "store-replaced", "{backend} GET {path}");
                assert!(
                    body["message"]
                        .as_str()
                        .unwrap()
                        .contains(&world.store().display().to_string()),
                    "{backend} GET {path}: {body}"
                );
            }
        }
        assert_eq!(
            server.get("/").status,
            200,
            "{backend}: the page reads no store"
        );
        replace(&world.store(), &junk, &second);
        assert_eq!(
            head(&server),
            b"{\"format\":\"ekr.view-head/1\",\"head\":0}"
        );
        server.stop();
    }
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

/// Every `.rs` file at or below `directory`, with its text.
fn rust_files(directory: &std::path::Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(directory).expect("a source directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = std::fs::read_to_string(&path).expect("a source file");
            found.push((path, text));
        }
    }
    found
}

/// No read route of the binary or of the views copies the head graph: each reads it through the
/// kernel's shared verified read (`Runtime::read`, `Runtime::schema_history`), whose sharing
/// `crates/ekr-kernel/tests/verified_read.rs::read_verbs_copy_no_transaction_record_and_no_graph`
/// counts. `Runtime::snapshot()` hands back a copy of the whole head graph on every call, so a
/// route calling it copies the graph per request; this case finds each such call in product
/// source. It is a lexical tripwire: comments and string literals do not count, and a call
/// written through another name for the same method would not be found.
#[test]
fn no_read_route_copies_the_head_graph_through_runtime_snapshot() {
    let crates = manifest_dir().join("..");
    let mut sources = rust_files(&crates.join("ekr/src"));
    sources.extend(rust_files(&crates.join("ekr-views/src")));
    assert!(
        sources.len() > 10,
        "the source scan found too few files: {}",
        sources.len()
    );
    let copying: Vec<String> = sources
        .iter()
        .filter(|(_, text)| {
            rust_source::tokens(text)
                .windows(4)
                .any(|call| call == [".", "snapshot", "(", ")"])
        })
        .map(|(path, _)| path.display().to_string())
        .collect();
    assert!(
        copying.is_empty(),
        "read routes that copy the head graph through `Runtime::snapshot()`: {copying:?}"
    );
}
