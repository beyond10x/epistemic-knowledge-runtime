//! `ekr view`'s bounded reads: `/overview`, `/node/<id>`, `/search` and the streamed `/expand`,
//! driven end to end through the real binary.
//!
//! Each answer is compared with what the `ekr-views` engine returns for the same request through
//! the public kernel `Runtime` opened on the same store: the JSON endpoints byte for byte, the
//! stream line for line against the engine's `SlicePage` (meta, records in order, a `progress`
//! line after every 256 records, an `end` line with the cursor). The stream's framing is read off
//! the raw bytes — HTTP/1.1 chunks, one flush per chunk — rather than trusted to a client library.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use ekr::host::CliHostConfigurationV1;
use ekr_core::{NodeId, RevisionNumber};
use ekr_kernel::Runtime;
use ekr_views::{ExpandRequest, Index, OverviewRequest, SearchRequest, SliceRecord};
use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const T_ALICE: &str = "00000000-0000-4000-8000-000000000601";
const ALICE: &str = "00000000-0000-4000-8000-000000000301";
/// A node id no store here holds.
const UNKNOWN_NODE: &str = "00000000-0000-4000-8000-000000000399";
/// Nodes of the generated star: one hub joined to every other node, and a chain through the rest.
const STAR_NODES: usize = 400;

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

    /// Revision 0 from the retraction seed, revision 1 from one committed transaction.
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

    /// A store of [`STAR_NODES`] nodes seeded in one revision: node 1 is joined to every other
    /// node, and every other node to the next, so a 1-hop expansion of node 1 is the whole store.
    fn seeded_star(backend: &'static str) -> Self {
        use std::fmt::Write as _;
        let world = Self::new(backend);
        let version = star_id(0, 1);
        let root = star_id(0, 2);
        let node_type = star_id(1, 1);
        let edge_type = star_id(2, 1);
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

    /// The store, observed through the public kernel facade under the same host.
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

    fn index(&self, at: Option<u64>) -> Index {
        Index::load(&self.runtime(), at.map(RevisionNumber::new)).unwrap()
    }

    /// Starts `ekr view --port 0` and reads the URL it prints.
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
        let printed: Value = serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("`ekr view` printed no JSON line ({error}): {line:?}"));
        let address = printed["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned();
        Server { child, address }
    }
}

/// An id of the generated star: `space` tells kinds of object apart, `n` numbers them.
fn star_id(space: u16, n: usize) -> String {
    format!("00000000-0000-4000-{:04x}-{n:012x}", 0x8200 + space)
}

/// A running `ekr view`, killed when dropped.
struct Server {
    child: Child,
    address: String,
}

impl Server {
    fn connect(&self, method: &str, path: &str) -> TcpStream {
        let mut stream = TcpStream::connect(&self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.address
        )
        .unwrap();
        stream
    }

    fn request(&self, method: &str, path: &str) -> Response {
        let mut raw = Vec::new();
        self.connect(method, path).read_to_end(&mut raw).unwrap();
        Response::parse(&raw)
    }

    fn get(&self, path: &str) -> Response {
        self.request("GET", path)
    }

    fn alive(&mut self) -> bool {
        self.child.try_wait().unwrap().is_none()
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
    /// The body as sent: for a chunked answer, each chunk's data in order.
    chunks: Vec<Vec<u8>>,
    chunked: bool,
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
        let body = &raw[split + 4..];
        let chunked = headers
            .iter()
            .any(|(name, value)| name == "transfer-encoding" && value == "chunked");
        let chunks = if chunked {
            dechunk(body)
        } else {
            vec![body.to_vec()]
        };
        Self {
            status,
            headers,
            chunks,
            chunked,
        }
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn body(&self) -> Vec<u8> {
        self.chunks.concat()
    }

    fn json(&self) -> Value {
        serde_json::from_slice(&self.body()).unwrap_or_else(|error| {
            panic!(
                "{error}: {}",
                String::from_utf8_lossy(&self.body())
                    .chars()
                    .take(400)
                    .collect::<String>()
            )
        })
    }

    /// The headers every answer carries: never sniffed, never stored, closed, no cookie, no CORS.
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
        assert_eq!(self.header("cache-control"), Some("no-store"), "{what}");
        assert_eq!(self.header("connection"), Some("close"), "{what}");
    }

    /// A named JSON refusal, whole, never a stream.
    fn assert_refused(&self, what: &str, status: u16, name: &str) {
        assert_eq!(
            self.status,
            status,
            "{what}: {}",
            String::from_utf8_lossy(&self.body())
        );
        assert!(!self.chunked, "{what}: a refusal is not streamed");
        assert_eq!(
            self.header("content-type"),
            Some("application/json"),
            "{what}"
        );
        assert_eq!(self.json()["refusal"], name, "{what}");
        self.assert_plain(what);
    }
}

/// Each chunk's data, checking the framing: a hex size line, that many bytes, CRLF, and a last
/// chunk of size zero followed by an empty trailer.
fn dechunk(mut body: &[u8]) -> Vec<Vec<u8>> {
    let mut chunks = Vec::new();
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
            return chunks;
        }
        chunks.push(body[..size].to_vec());
        assert_eq!(&body[size..size + 2], b"\r\n", "a chunk ends with CRLF");
        body = &body[size + 2..];
    }
}

/// `value`, a JSON object, with `"kind": kind` added.
fn kinded(kind: &str, value: impl serde::Serialize) -> Value {
    let mut value = serde_json::to_value(value).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .insert("kind".to_owned(), Value::from(kind));
    value
}

/// The lines a stream of `page` carries, as the contract states them.
fn expected_lines(page: &ekr_views::SlicePage) -> Vec<Value> {
    let mut lines = vec![kinded("meta", page.meta())];
    for (at, record) in page.records().iter().enumerate() {
        lines.push(match record {
            SliceRecord::Node(node) => kinded("node", node),
            SliceRecord::Edge(edge) => kinded("edge", edge),
        });
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

/// The stream's lines, checking each is one `\n`-terminated JSON object that opens with its kind.
fn stream_lines(response: &Response) -> Vec<Value> {
    let body = response.body();
    assert_eq!(body.last(), Some(&b'\n'), "the last line is terminated");
    body[..body.len() - 1]
        .split(|byte| *byte == b'\n')
        .map(|line| {
            assert!(
                line.starts_with(b"{\"kind\":\""),
                "a line opens with its kind: {}",
                String::from_utf8_lossy(line)
            );
            serde_json::from_slice(line).unwrap()
        })
        .collect()
}

fn assert_streamed(response: &Response, what: &str) {
    assert_eq!(response.status, 200, "{what}");
    assert!(response.chunked, "{what}: Transfer-Encoding: chunked");
    assert_eq!(response.header("content-length"), None, "{what}");
    assert_eq!(
        response.header("content-type"),
        Some("application/x-ndjson"),
        "{what}"
    );
    response.assert_plain(what);
}

/// `/overview`, `/node/<id>` and `/search` answer the engine's bytes for the head and for an
/// earlier revision, as `application/json`, on both providers; `/projection` still answers the
/// projection beside them.
#[test]
fn overview_node_and_search_answer_the_engines_bytes_on_both_providers() {
    for backend in BACKENDS {
        let world = World::seeded_with_two_revisions(backend);
        let alice: NodeId = ALICE.parse().unwrap();
        let (head, zero) = (world.index(None), world.index(Some(0)));
        let overview = |index: &Index, limit| {
            index
                .overview(&OverviewRequest::new(limit).unwrap())
                .unwrap()
                .bytes
        };
        let search = |index: &Index, text: &str, limit| {
            index
                .search(&SearchRequest::new(text.to_owned(), limit).unwrap())
                .unwrap()
                .bytes
        };
        let projection = ekr_views::project(&world.runtime(), None).unwrap().bytes;
        let server = world.serve();
        for (path, expected) in [
            ("/overview".to_owned(), overview(&head, None)),
            ("/overview?revision=0".to_owned(), overview(&zero, None)),
            ("/overview?limit=1".to_owned(), overview(&head, Some(1))),
            (
                "/overview?revision=0&limit=2".to_owned(),
                overview(&zero, Some(2)),
            ),
            (
                format!("/node/{ALICE}"),
                head.describe(alice).unwrap().bytes,
            ),
            (
                format!("/node/{ALICE}?revision=0"),
                zero.describe(alice).unwrap().bytes,
            ),
            ("/search?q=Ali".to_owned(), search(&head, "Ali", 20)),
            ("/search?q=%41li".to_owned(), search(&head, "Ali", 20)),
            ("/search?q=ac+me".to_owned(), search(&head, "ac me", 20)),
            (
                "/search?q=b&limit=1&revision=0".to_owned(),
                search(&zero, "b", 1),
            ),
            ("/projection".to_owned(), projection.clone()),
        ] {
            let answered = server.get(&path);
            assert_eq!(answered.status, 200, "{backend} GET {path}");
            assert!(!answered.chunked, "{backend} GET {path}");
            assert_eq!(
                answered.header("content-type"),
                Some("application/json"),
                "{backend} GET {path}"
            );
            answered.assert_plain(&path);
            assert_eq!(
                String::from_utf8_lossy(&answered.body()),
                String::from_utf8_lossy(&expected),
                "{backend} GET {path}: the engine's bytes"
            );
        }
    }
}

/// A 1-hop `/expand` of the hub streams the whole neighbourhood as chunked NDJSON: the meta line
/// alone in the first chunk, the records in the engine's order with a `progress` line after every
/// 256, and an `end` line with no cursor.
#[test]
fn expand_streams_meta_records_progress_and_end_as_chunked_ndjson() {
    let world = World::seeded_star("file");
    let hub: NodeId = star_id(3, 1).parse().unwrap();
    let page = world
        .index(None)
        .page(&ExpandRequest::new(vec![hub], 1, 2000, None, None).unwrap())
        .unwrap();
    assert_eq!(page.records().len(), STAR_NODES + (STAR_NODES - 1) * 2 - 1);
    let server = world.serve();
    let path = format!("/expand?seeds={hub}&depth=1&limit=2000");
    let streamed = server.get(&path);
    assert_streamed(&streamed, &path);
    let lines = stream_lines(&streamed);
    assert_eq!(lines, expected_lines(&page), "GET {path}");
    assert_eq!(lines.last().unwrap()["next"], Value::Null);
    assert_eq!(
        lines
            .iter()
            .filter(|line| line["kind"] == "progress")
            .count(),
        page.records().len() / 256
    );
    // The meta line is flushed before any record, and every progress line ends its chunk.
    assert_eq!(
        serde_json::from_slice::<Value>(&streamed.chunks[0]).unwrap(),
        lines[0],
        "the first chunk is the meta line alone"
    );
    for chunk in &streamed.chunks[1..streamed.chunks.len() - 1] {
        let last = chunk[..chunk.len() - 1]
            .rsplit(|byte| *byte == b'\n')
            .next()
            .unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(last).unwrap()["kind"],
            "progress",
            "a chunk before the last ends with its progress line"
        );
    }
    assert!(streamed.chunks.len() >= 2 + page.records().len() / 256);
}

/// A cursor pages to the end: each page's `end.next` is the next request's `after`, the records
/// of every page together are the whole neighbourhood's, and the last page names no cursor. `edges`
/// and `revision` reach the engine.
#[test]
fn expand_pages_with_its_cursor_to_the_end_and_passes_every_parameter() {
    let world = World::seeded_star("sqlite");
    let hub: NodeId = star_id(3, 1).parse().unwrap();
    let index = world.index(None);
    let whole = index
        .page(&ExpandRequest::new(vec![hub], 1, 2000, None, None).unwrap())
        .unwrap();
    let server = world.serve();
    let mut after: Option<u64> = None;
    let mut records = Vec::new();
    let mut pages = 0;
    loop {
        let cursor = after.map(|at| format!("&after={at}")).unwrap_or_default();
        let path = format!("/expand?seeds={hub}&depth=1&limit=150&edges=300{cursor}&revision=0");
        let request = ExpandRequest::new(
            vec![hub],
            1,
            150,
            Some(300),
            after.map(|at| i64::try_from(at).unwrap()),
        )
        .unwrap();
        let page = index.page(&request).unwrap();
        let streamed = server.get(&path);
        assert_streamed(&streamed, &path);
        let lines = stream_lines(&streamed);
        assert_eq!(lines, expected_lines(&page), "GET {path}");
        records.extend(
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
        assert!(pages < 100, "the cursor advances");
    }
    assert!(pages > 1, "a limit of 150 pages the star");
    assert_eq!(
        records,
        expected_lines(&whole)
            .into_iter()
            .filter(|line| line["kind"] == "node" || line["kind"] == "edge")
            .collect::<Vec<_>>()
    );
}

/// Every refusal is decided before the first byte and answered as a whole JSON refusal: an unknown
/// node or seed and an absent revision are 404, a bound outside its range is 400
/// `ekr.views.LimitExceeded`, and a query the endpoint cannot read is 400 `invalid-query`.
#[test]
fn the_bounded_reads_refuse_as_whole_json_before_any_byte() {
    let world = World::seeded_with_two_revisions("file");
    let server = world.serve();
    for (path, status, name) in [
        (
            format!("/expand?seeds={UNKNOWN_NODE}&depth=1&limit=10"),
            404,
            "ekr.views.NodeNotFound",
        ),
        (
            format!("/expand?seeds={ALICE},{UNKNOWN_NODE}&depth=1&limit=10"),
            404,
            "ekr.views.NodeNotFound",
        ),
        (
            format!("/expand?seeds={ALICE}&depth=1&limit=10&revision=9"),
            404,
            "ekr.views.RevisionNotFound",
        ),
        (
            format!("/expand?seeds={ALICE}&depth=3&limit=10"),
            400,
            "ekr.views.LimitExceeded",
        ),
        (
            format!("/expand?seeds={ALICE}&depth=1&limit=0"),
            400,
            "ekr.views.LimitExceeded",
        ),
        (
            format!("/expand?seeds={ALICE}&depth=1&limit=2001"),
            400,
            "ekr.views.LimitExceeded",
        ),
        (
            format!("/expand?seeds={ALICE}&depth=1&limit=10&edges=5001"),
            400,
            "ekr.views.LimitExceeded",
        ),
        (
            format!("/expand?seeds={ALICE}&depth=-1&limit=10"),
            400,
            "ekr.views.LimitExceeded",
        ),
        (
            format!("/expand?seeds={ALICE}&depth=1&limit=10&after=-1"),
            400,
            "ekr.views.LimitExceeded",
        ),
        (
            "/expand?seeds=not-an-id&depth=1&limit=10".to_owned(),
            400,
            "invalid-query",
        ),
        ("/expand?depth=1&limit=10".to_owned(), 400, "invalid-query"),
        (
            "/expand?seeds=&depth=1&limit=10".to_owned(),
            400,
            "invalid-query",
        ),
        (
            format!("/expand?seeds={ALICE}&limit=10"),
            400,
            "invalid-query",
        ),
        (
            format!("/expand?seeds={ALICE}&depth=1"),
            400,
            "invalid-query",
        ),
        (
            format!("/expand?seeds={ALICE}&depth=one&limit=10"),
            400,
            "invalid-query",
        ),
        (
            format!("/expand?seeds={ALICE}&depth=1&limit=10&depth=1"),
            400,
            "invalid-query",
        ),
        (
            format!("/expand?seeds={ALICE}&depth=1&limit=10&colour=red"),
            400,
            "invalid-query",
        ),
        (
            format!("/node/{UNKNOWN_NODE}"),
            404,
            "ekr.views.NodeNotFound",
        ),
        ("/node/not-an-id".to_owned(), 404, "ekr.views.NodeNotFound"),
        (
            format!("/node/{ALICE}?revision=9"),
            404,
            "ekr.views.RevisionNotFound",
        ),
        (format!("/node/{ALICE}?limit=1"), 400, "invalid-query"),
        (
            "/overview?revision=9".to_owned(),
            404,
            "ekr.views.RevisionNotFound",
        ),
        (
            "/overview?limit=501".to_owned(),
            400,
            "ekr.views.LimitExceeded",
        ),
        (
            "/overview?limit=0".to_owned(),
            400,
            "ekr.views.LimitExceeded",
        ),
        ("/overview?revision=x".to_owned(), 400, "invalid-query"),
        (
            "/overview?revision=1&revision=1".to_owned(),
            400,
            "invalid-query",
        ),
        ("/overview?q=a".to_owned(), 400, "invalid-query"),
        ("/overview?&".to_owned(), 400, "invalid-query"),
        ("/search".to_owned(), 400, "invalid-query"),
        (
            "/search?q=a&limit=101".to_owned(),
            400,
            "ekr.views.LimitExceeded",
        ),
        (
            "/search?q=a&limit=0".to_owned(),
            400,
            "ekr.views.LimitExceeded",
        ),
        ("/search?q=%4".to_owned(), 400, "invalid-query"),
        ("/search?q=%zz".to_owned(), 400, "invalid-query"),
        ("/search?q=%ff".to_owned(), 400, "invalid-query"),
        (
            "/search?q=a&revision=9".to_owned(),
            404,
            "ekr.views.RevisionNotFound",
        ),
    ] {
        server
            .get(&path)
            .assert_refused(&format!("GET {path}"), status, name);
    }
    for path in ["/overview", "/expand", "/node/x", "/search"] {
        let refused = server.request("POST", path);
        assert_eq!(refused.status, 405, "POST {path}");
        refused.assert_plain(&format!("POST {path}"));
    }
    for path in ["/overview/", "/node/", "/node/a/b", "/search/", "/expand/"] {
        let missing = server.get(path);
        assert_eq!(missing.status, 404, "GET {path}");
        missing.assert_plain(path);
    }
}

/// A client that closes an `/expand` after its first bytes ends that stream and frees its place:
/// after 80 such clients — more than the 64 places — the viewer still serves, and an `/overview`
/// asked while a stream is open and unread is answered at once, because the stream is written from
/// its own connection and not from the thread that reads the store.
#[test]
fn a_closed_stream_frees_its_place_and_an_open_one_holds_nobody_else() {
    let world = World::seeded_star("file");
    let hub = star_id(3, 1);
    let mut server = world.serve();
    let path = format!("/expand?seeds={hub}&depth=2&limit=2000");
    assert_eq!(server.get("/overview").status, 200, "the index is loaded");
    for n in 0..80 {
        let mut stream = server.connect("GET", &path);
        let mut first = [0_u8; 64];
        stream.read_exact(&mut first).unwrap();
        assert!(first.starts_with(b"HTTP/1.1 200 "), "client {n}");
        drop(stream);
    }
    std::thread::sleep(Duration::from_millis(200));
    assert!(server.alive(), "closed streams ended the server");
    let open = server.connect("GET", &path);
    let asked = Instant::now();
    assert_eq!(server.get("/overview").status, 200, "beside an open stream");
    assert!(
        asked.elapsed() < Duration::from_secs(2),
        "answered at once: {:?}",
        asked.elapsed()
    );
    drop(open);
    assert_eq!(server.get("/").status, 200, "after every stream closed");
}
