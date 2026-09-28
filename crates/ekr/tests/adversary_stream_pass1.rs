//! Adversary, story:view-streams-overview-and-expansion, pass 1: the streamed `ekr view` server.
//!
//! Driven through the real binary, against the stream contract (`/expand`: chunked NDJSON, meta,
//! records, progress, end), the refusal order of `systems/ekr/domains/views.yaml` ("Bounds and the
//! order of refusals"), and HTTP/1.1 itself (RFC 9112 § 6.1: a server MUST NOT send
//! Transfer-Encoding to a request that does not indicate HTTP/1.1).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use ekr::host::CliHostConfigurationV1;
use ekr_core::{NodeId, RevisionNumber};
use ekr_kernel::Runtime;
use ekr_views::{ExpandRequest, Index, SliceRecord};
use serde_json::Value;

const ALICE: &str = "00000000-0000-4000-8000-000000000301";
const STAR_NODES: usize = 400;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/retraction")
        .join(name)
}

struct World {
    directory: tempfile::TempDir,
}

impl World {
    fn new() -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
        }
    }

    fn store(&self) -> PathBuf {
        self.directory.path().join("store")
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            fixture("host.json").display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
            "--backend".to_owned(),
            "file".to_owned(),
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
            "{verb:?}: stderr {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn seeded() -> Self {
        let world = Self::new();
        let seed = fixture("seed.yaml").display().to_string();
        assert_eq!(world.ok(&["seed", &seed])["result"]["revision"], 0);
        world
    }

    /// Commits, into the star, one node joined to nothing: revision 1, unrelated to any
    /// neighbourhood of revision 0.
    fn commit_unrelated_node(&self) {
        let transaction = "00000000-0000-4000-8000-000000000799";
        let document = format!(
            "format: ekr.transaction-document/2\ntransaction:\n  id: {transaction}\n  proposer: 00000000-0000-4000-8000-000000000101\n  operations:\n  - !CreateNode\n    id: {}\n    root_id: {}\n    type_id: {}\n    canonical_name: unrelated\n    properties: {{}}\n  evidence: []\n",
            star_id(3, 999),
            star_id(0, 2),
            star_id(1, 1)
        );
        let path = self.directory.path().join("unrelated.yaml");
        std::fs::write(&path, document).unwrap();
        assert_eq!(
            self.ok(&["propose", &path.display().to_string()])["transaction_id"],
            transaction
        );
        assert_eq!(
            self.ok(&["validate", transaction, "--against", "0"])["kind"],
            "Validated"
        );
        assert_eq!(self.ok(&["commit", transaction])["result"]["revision"], 1);
    }

    /// [`STAR_NODES`] nodes: node 1 joined to every other, and a chain through the rest.
    fn seeded_star() -> Self {
        use std::fmt::Write as _;
        let world = Self::new();
        let (version, root, node_type, edge_type) =
            (star_id(0, 1), star_id(0, 2), star_id(1, 1), star_id(2, 1));
        let mut yaml = format!(
            "format: ekr-seed/2\nontology:\n  version:\n    id: {version}\n    number: 0\n    parent: null\n    created_at: 0\n  node_types:\n  - id: {node_type}\n    name: type-1\n    parents: []\n    properties: {{}}\n    abstract_type: false\n    lifecycle: null\n    operations: {{}}\n  edge_types:\n  - id: {edge_type}\n    name: link-1\n    source_types:\n    - {node_type}\n    target_types:\n    - {node_type}\n    cardinality: Many\n    properties: {{}}\n    inverse: null\n    symmetric: false\n    transitive: false\ngraph:\n  format: ekr.graph-document/2\n  graph:\n    root:\n      id: {root}\n      space: Canonical\n      schema_version_id: {version}\n      parent: null\n      created_at: 0\n    revision: 0\n    nodes:\n"
        );
        for n in 1..=STAR_NODES {
            let id = star_id(3, n);
            let _ = write!(
                yaml,
                "      {id}:\n        id: {id}\n        root_id: {root}\n        type_id: {node_type}\n        canonical_name: entity-{n}\n        aliases: []\n        type_state: null\n        properties: {{}}\n"
            );
        }
        yaml.push_str("    edges:\n");
        let mut edge = 0;
        let mut join = |yaml: &mut String, source: usize, target: usize| {
            edge += 1;
            let id = star_id(4, edge);
            let _ = write!(
                yaml,
                "      {id}:\n        id: {id}\n        root_id: {root}\n        type_id: {edge_type}\n        source: {}\n        target: {}\n        properties: {{}}\n",
                star_id(3, source),
                star_id(3, target)
            );
        };
        for n in 2..=STAR_NODES {
            join(&mut yaml, 1, n);
        }
        for n in 2..STAR_NODES {
            join(&mut yaml, n, n + 1);
        }
        yaml.push_str("    assertions: {}\n    evidence: {}\nevidence_payloads: {}\n");
        let seed = world.directory.path().join("seed.yaml");
        std::fs::write(&seed, yaml).unwrap();
        assert_eq!(
            world.ok(&["seed", &seed.display().to_string()])["result"]["revision"],
            0
        );
        world
    }

    fn runtime(&self) -> Runtime {
        let host = CliHostConfigurationV1::from_json(&std::fs::read(fixture("host.json")).unwrap())
            .unwrap();
        Runtime::file_existing(&self.store(), &host.tenant, host.context, host.authority).unwrap()
    }

    fn index(&self, at: Option<u64>) -> Index {
        Index::load(&self.runtime(), at.map(RevisionNumber::new)).unwrap()
    }

    fn serve(&self) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(self.args(&["view", "--port", "0"]))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let printed: Value = serde_json::from_str(&line).unwrap();
        let address = printed["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned();
        Server { child, address }
    }
}

fn star_id(space: u16, n: usize) -> String {
    format!("00000000-0000-4000-{:04x}-{n:012x}", 0x8200 + space)
}

struct Server {
    child: Child,
    address: String,
}

impl Server {
    fn raw(&self, version: &str, path: &str) -> Vec<u8> {
        let mut stream = TcpStream::connect(&self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        write!(
            stream,
            "GET {path} {version}\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.address
        )
        .unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        raw
    }

    fn get(&self, path: &str) -> Response {
        Response::parse(&self.raw("HTTP/1.1", path))
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
        let raw_body = &raw[split + 4..];
        let chunked = headers
            .iter()
            .any(|(name, value)| name == "transfer-encoding" && value == "chunked");
        let body = if chunked {
            dechunk(raw_body)
        } else {
            raw_body.to_vec()
        };
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

    fn json(&self) -> Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|error| panic!("{error}: {}", String::from_utf8_lossy(&self.body)))
    }

    fn lines(&self) -> Vec<Value> {
        assert_eq!(
            self.body.last(),
            Some(&b'\n'),
            "the last line is terminated"
        );
        self.body[..self.body.len() - 1]
            .split(|byte| *byte == b'\n')
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect()
    }
}

fn dechunk(mut body: &[u8]) -> Vec<u8> {
    let mut data = Vec::new();
    loop {
        let end = body
            .windows(2)
            .position(|w| w == b"\r\n")
            .expect("a chunk size line");
        let size = usize::from_str_radix(std::str::from_utf8(&body[..end]).unwrap(), 16)
            .expect("a hex chunk size");
        body = &body[end + 2..];
        if size == 0 {
            assert_eq!(body, b"\r\n", "the last chunk ends the body");
            return data;
        }
        data.extend_from_slice(&body[..size]);
        assert_eq!(&body[size..size + 2], b"\r\n", "a chunk ends with CRLF");
        body = &body[size + 2..];
    }
}

fn kinded(kind: &str, value: impl serde::Serialize) -> Value {
    let mut value = serde_json::to_value(value).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("kind".to_owned(), Value::from(kind));
    value
}

fn record_lines(page: &ekr_views::SlicePage) -> Vec<Value> {
    page.records()
        .iter()
        .map(|record| match record {
            SliceRecord::Node(node) => kinded("node", node),
            SliceRecord::Edge(edge) => kinded("edge", edge),
        })
        .collect()
}

fn expected_lines(page: &ekr_views::SlicePage) -> Vec<Value> {
    let mut lines = vec![kinded("meta", page.meta())];
    for (at, line) in record_lines(page).into_iter().enumerate() {
        lines.push(line);
        if (at + 1) % 256 == 0 {
            lines.push(serde_json::json!({"kind": "progress", "sent": at + 1}));
        }
    }
    lines.push(serde_json::json!({
        "kind": "end",
        "next": page.next(),
        "remaining": page.remaining(),
    }));
    lines
}

/// RFC 9112 § 6.1: "A server MUST NOT send a response containing Transfer-Encoding unless the
/// corresponding request indicates HTTP/1.1 (or later minor revisions)." An HTTP/1.0 client —
/// `curl --http1.0`, a proxy that speaks 1.0 — reads the chunk-size lines as body: the stream is
/// no longer NDJSON. `/expand` answers every request chunked, whatever version it named.
#[test]
fn an_http_1_0_request_is_not_answered_with_transfer_encoding() {
    let world = World::seeded();
    let server = world.serve();
    let path = format!("/expand?seeds={ALICE}&depth=1&limit=10");
    let answered = Response::parse(&server.raw("HTTP/1.0", &path));
    assert_eq!(
        answered.status,
        200,
        "{}",
        String::from_utf8_lossy(&answered.body)
    );
    assert_eq!(
        answered.header("transfer-encoding"),
        None,
        "GET {path} HTTP/1.0 was answered with Transfer-Encoding"
    );
}

/// views.yaml: "then a revision beyond the head answers RevisionNotFound; then a node the
/// revision does not hold answers NodeNotFound". `/node/<id>` refuses an id that does not parse
/// as NodeNotFound before it reads the revision, so a request naming both an absent revision and
/// no node is answered NodeNotFound — the refusal the specification orders last.
#[test]
fn an_absent_revision_is_refused_before_a_node_that_is_no_node_id() {
    let world = World::seeded();
    let server = world.serve();
    let answered = server.get("/node/not-an-id?revision=9");
    assert_eq!(answered.status, 404);
    assert_eq!(
        answered.json()["refusal"],
        "ekr.views.RevisionNotFound",
        "{}",
        String::from_utf8_lossy(&answered.body)
    );
}

/// views.yaml, ExpandNeighbourhood: "`seeds` is a set … an empty set answers an empty page with
/// node_total 0. A lower bound of one seed was tried and dropped". `/expand?seeds=` is the empty
/// set, and the server refuses it as `invalid-query`.
#[test]
fn an_empty_seed_set_answers_an_empty_page() {
    let world = World::seeded();
    let server = world.serve();
    let answered = server.get("/expand?seeds=&depth=1&limit=10");
    assert_eq!(
        answered.status,
        200,
        "{}",
        String::from_utf8_lossy(&answered.body)
    );
    let lines = answered.lines();
    assert_eq!(lines[0]["node_total"], 0);
    assert_eq!(lines.last().unwrap()["next"], Value::Null);
}

/// A reader pages a pinned revision while a new head is committed between two pages and while one
/// page is open and unread: every page is the engine's page of revision 0, the records together
/// are the whole neighbourhood once each, and the head moved.
#[test]
fn a_pinned_revision_pages_across_a_commit_with_nothing_lost_or_repeated() {
    let world = World::seeded_star();
    let hub: NodeId = star_id(3, 1).parse().unwrap();
    let before = world.index(Some(0));
    let whole = before
        .page(&ExpandRequest::new(vec![hub], 2, 2000, None, None).unwrap())
        .unwrap();
    assert_eq!(whole.records().len(), STAR_NODES + (STAR_NODES - 1) * 2 - 1);
    let server = world.serve();
    let mut after: Option<u64> = None;
    let mut delivered = Vec::new();
    let mut pages = 0;
    loop {
        let cursor = after.map(|at| format!("&after={at}")).unwrap_or_default();
        let path = format!("/expand?seeds={hub}&depth=2&limit=150&edges=300{cursor}&revision=0");
        let page = before
            .page(
                &ExpandRequest::new(
                    vec![hub],
                    2,
                    150,
                    Some(300),
                    after.map(|at| i64::try_from(at).unwrap()),
                )
                .unwrap(),
            )
            .unwrap();
        let answered = if pages == 1 {
            // Open the page, read its first bytes, commit a new head, then read the rest.
            let mut stream = TcpStream::connect(&server.address).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(60)))
                .unwrap();
            write!(
                stream,
                "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                server.address
            )
            .unwrap();
            let mut raw = vec![0_u8; 16];
            stream.read_exact(&mut raw).unwrap();
            world.commit_unrelated_node();
            stream.read_to_end(&mut raw).unwrap();
            Response::parse(&raw)
        } else {
            server.get(&path)
        };
        assert_eq!(answered.status, 200, "GET {path}");
        let lines = answered.lines();
        let mut expected = expected_lines(&page);
        // task:historical-projection-carries-the-head: meta.head is allowed to move.
        expected[0]["head"] = lines[0]["head"].clone();
        assert_eq!(lines, expected, "GET {path}");
        delivered.extend(
            lines
                .iter()
                .filter(|line| line["kind"] == "node" || line["kind"] == "edge")
                .cloned(),
        );
        pages += 1;
        match lines.last().unwrap()["next"].as_u64() {
            Some(next) => after = Some(next),
            None => break,
        }
        assert!(pages < 50, "the cursor advances");
    }
    assert!(pages > 2, "the commit fell between pages");
    assert_eq!(delivered, record_lines(&whole));
    assert_eq!(server.get("/overview").json()["meta"]["head"], 1);
}

/// Sixty-four readers — the in-flight cap — stream the same 1-hop expansion of the star's hub at
/// once, and each receives the whole stream, framed, identical to the engine's page.
#[test]
fn sixty_four_concurrent_streams_each_deliver_the_whole_page() {
    let world = World::seeded_star();
    let hub: NodeId = star_id(3, 1).parse().unwrap();
    let page = world
        .index(None)
        .page(&ExpandRequest::new(vec![hub], 1, 2000, None, None).unwrap())
        .unwrap();
    let expected = expected_lines(&page);
    let server = world.serve();
    assert_eq!(server.get("/overview").status, 200, "the index is loaded");
    // That connection's thread releases its place just after it closes.
    std::thread::sleep(Duration::from_millis(300));
    let path = format!("/expand?seeds={hub}&depth=1&limit=2000");
    let address = server.address.clone();
    let readers: Vec<_> = (0..64)
        .map(|_| {
            let (address, path) = (address.clone(), path.clone());
            std::thread::spawn(move || {
                let mut stream = TcpStream::connect(&address).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(60)))
                    .unwrap();
                write!(
                    stream,
                    "GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
                )
                .unwrap();
                let mut raw = Vec::new();
                stream.read_to_end(&mut raw).unwrap();
                Response::parse(&raw)
            })
        })
        .collect();
    for (n, reader) in readers.into_iter().enumerate() {
        let answered = reader.join().unwrap();
        assert_eq!(answered.status, 200, "reader {n}");
        assert_eq!(answered.lines(), expected, "reader {n}");
    }
}
