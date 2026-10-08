//! Adversary pass 1 on the `ekr view` page (`crates/ekr/src/cli/viewer/index.html`) for
//! `story:view-streams-overview-and-expansion` and `task:timeline-rows-are-subjects`.
//!
//! Every case seeds a store through the real binary and serves it with the real `ekr view`. A front
//! server stands between the browser and `ekr view`: it answers `/` with the embedded page followed
//! by a probe script of the case's own (the page is not changed; the probe only reads the page's
//! state and writes it into a `<pre id="probe-out">` the case reads off `--dump-dom`), and forwards
//! every other request to `ekr view` unchanged — or, where a case says so, re-frames a streamed
//! answer into chunks of a few bytes, or answers one address itself. The page's graph libraries are
//! served to the browser from `tests/fixtures/viewer-libraries/` (`support/viewer_libraries.rs`).

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::Value;

#[path = "support/viewer_libraries.rs"]
mod viewer_libraries;

const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const DAY: i64 = 86_400_000;

fn manifest_dir() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR"))
}

/// The embedded page; `EKR_ADVERSARY_PAGE` names a mutated copy, to show a case can fail.
fn page() -> String {
    let path = std::env::var("EKR_ADVERSARY_PAGE").map_or_else(
        |_| manifest_dir().join("src/cli/viewer/index.html"),
        PathBuf::from,
    );
    std::fs::read_to_string(path).unwrap()
}

fn gid(space: u16, n: usize) -> String {
    format!("00000000-0000-4000-{:04x}-{n:012x}", 0x9100 + space)
}

// ---- stores ------------------------------------------------------------------------------------

struct Seeded {
    directory: tempfile::TempDir,
    fixture: tempfile::TempDir,
}

impl Seeded {
    /// Seeds `seed` and commits each of `commits` in order, each validated against the head before it.
    fn new(seed: &str, commits: &[String]) -> Self {
        let fixture = tempfile::tempdir().unwrap();
        std::fs::copy(
            manifest_dir().join("tests/fixtures/view-page/sounding/host.json"),
            fixture.path().join("host.json"),
        )
        .unwrap();
        std::fs::write(fixture.path().join("seed.yaml"), seed).unwrap();
        let seeded = Self {
            directory: tempfile::tempdir().unwrap(),
            fixture,
        };
        let path = seeded
            .fixture
            .path()
            .join("seed.yaml")
            .display()
            .to_string();
        seeded.ok(&["seed", &path]);
        for (at, commit) in commits.iter().enumerate() {
            let file = seeded.fixture.path().join(format!("commit-{at}.yaml"));
            std::fs::write(&file, commit).unwrap();
            let proposed = seeded.ok(&["propose", &file.display().to_string()]);
            let id = proposed["transaction_id"].as_str().unwrap().to_owned();
            let validated = seeded.ok(&["validate", &id, "--against", &at.to_string()]);
            assert_eq!(validated["kind"], "Validated", "commit {at}: {validated}");
            seeded.ok(&["commit", &id]);
        }
        seeded
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            self.fixture.path().join("host.json").display().to_string(),
            "--store".to_owned(),
            self.directory.path().join("store").display().to_string(),
            "--backend".to_owned(),
            "file".to_owned(),
        ];
        args.extend(verb.iter().map(|arg| (*arg).to_owned()));
        args
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(self.args(verb))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{verb:?}: stderr {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
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
        let url = printed["url"].as_str().unwrap().to_owned();
        Server { child, url }
    }
}

struct Server {
    child: Child,
    url: String,
}

impl Server {
    fn address(&self) -> String {
        self.url
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned()
    }

    fn get(&self, path: &str) -> Value {
        let address = self.address();
        let mut stream = TcpStream::connect(&address).unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        let end = find(&raw, b"\r\n\r\n").unwrap();
        serde_json::from_slice(&raw[end + 4..]).unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

/// A node type of a seed: its id, its name and its properties as `(id, name, value kind)`.
type Kind<'a> = (String, &'a str, Vec<(String, &'a str, &'a str)>);

/// The seed's ontology: `kinds` node types (id, name, properties as `(id, name, value kind)`) and
/// one edge type joining all of them; then the graph's opening lines up to `nodes:`.
fn seed_head(kinds: &[Kind<'_>], link: &str) -> String {
    use std::fmt::Write as _;
    let (version, root) = (gid(0, 1), gid(0, 2));
    let mut yaml = format!(
        "format: ekr-seed/2\nontology:\n  version:\n    id: {version}\n    number: 0\n    parent: null\n    created_at: 0\n  node_types:\n"
    );
    for (id, name, properties) in kinds {
        let _ = write!(yaml, "  - id: {id}\n    name: {name}\n    parents: []\n");
        if properties.is_empty() {
            yaml.push_str("    properties: {}\n");
        } else {
            yaml.push_str("    properties:\n");
            for (pid, pname, kind) in properties {
                let _ = write!(
                    yaml,
                    "      {pid}:\n        id: {pid}\n        name: {pname}\n        value_type:\n          value_kind: {kind}\n        cardinality: One\n        required: false\n        constraints: []\n"
                );
            }
        }
        yaml.push_str("    abstract_type: false\n    lifecycle: null\n    operations: {}\n");
    }
    let listed: String = kinds
        .iter()
        .map(|(id, _, _)| format!("\n    - {id}"))
        .collect();
    let _ = write!(
        yaml,
        "  edge_types:\n  - id: {link}\n    name: link-1\n    source_types:{listed}\n    target_types:{listed}\n    cardinality: Many\n    properties: {{}}\n    inverse: null\n    symmetric: false\n    transitive: false\ngraph:\n  format: ekr.graph-document/2\n  graph:\n    root:\n      id: {root}\n      space: Canonical\n      schema_version_id: {version}\n      parent: null\n      created_at: 0\n    revision: 0\n    nodes:\n"
    );
    yaml
}

fn node_yaml(id: &str, kind: &str, name: &str, aliases: &[String], properties: &str) -> String {
    let root = gid(0, 2);
    let aliases = if aliases.is_empty() {
        " []".to_owned()
    } else {
        aliases.iter().map(|a| format!("\n        - {a}")).collect()
    };
    format!(
        "      {id}:\n        id: {id}\n        root_id: {root}\n        type_id: {kind}\n        canonical_name: {name}\n        aliases:{aliases}\n        type_state: null\n        properties:{properties}\n"
    )
}

fn edge_yaml(id: &str, link: &str, source: &str, target: &str) -> String {
    let root = gid(0, 2);
    format!(
        "      {id}:\n        id: {id}\n        root_id: {root}\n        type_id: {link}\n        source: {source}\n        target: {target}\n        properties: {{}}\n"
    )
}

/// Two revisions of one node type: `entity-1` to `entity-3` in the seed, and `entity-late`, which
/// revision 1 creates.
fn two_revisions() -> Seeded {
    let (kind, link) = (gid(1, 1), gid(2, 1));
    let mut seed = seed_head(&[(kind.clone(), "kind-1", vec![])], &link);
    for n in 1..=3 {
        seed.push_str(&node_yaml(
            &gid(3, n),
            &kind,
            &format!("entity-{n}"),
            &[],
            " {}",
        ));
    }
    seed.push_str("    edges:\n");
    seed.push_str(&edge_yaml(&gid(4, 1), &link, &gid(3, 1), &gid(3, 2)));
    seed.push_str(&edge_yaml(&gid(4, 2), &link, &gid(3, 2), &gid(3, 3)));
    seed.push_str("    assertions: {}\n    evidence: {}\nevidence_payloads: {}\n");
    let commit = format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {}\n  proposer: {OPERATOR}\n  operations:\n  - !CreateNode\n    id: {}\n    root_id: {}\n    type_id: {kind}\n    canonical_name: entity-late\n    properties: {{}}\n  evidence: []\n",
        gid(8, 1),
        gid(3, 9),
        gid(0, 2)
    );
    Seeded::new(&seed, &[commit])
}

/// 400 nodes of one type: 1 to 300 on two rings (degree 4, and 5 for 1 to 100), and 301 to 400
/// each joined to one ring node (degree 1), so the overview's top 300 are the ring nodes and every
/// node past 300 is reached only by an expansion.
fn ring_and_leaves() -> Seeded {
    let (kind, link) = (gid(1, 1), gid(2, 1));
    let mut seed = seed_head(&[(kind.clone(), "kind-1", vec![])], &link);
    for n in 1..=400 {
        seed.push_str(&node_yaml(
            &gid(3, n),
            &kind,
            &format!("entity-{n}"),
            &[],
            " {}",
        ));
    }
    seed.push_str("    edges:\n");
    let mut e = 0;
    for k in 1..=300 {
        e += 1;
        seed.push_str(&edge_yaml(
            &gid(4, e),
            &link,
            &gid(3, k),
            &gid(3, k % 300 + 1),
        ));
        e += 1;
        seed.push_str(&edge_yaml(
            &gid(4, e),
            &link,
            &gid(3, k),
            &gid(3, (k + 1) % 300 + 1),
        ));
    }
    for n in 301..=400 {
        e += 1;
        seed.push_str(&edge_yaml(&gid(4, e), &link, &gid(3, n), &gid(3, n - 300)));
    }
    seed.push_str("    assertions: {}\n    evidence: {}\nevidence_payloads: {}\n");
    Seeded::new(&seed, &[])
}

/// `ring_and_leaves` with its leaves (301 to 400) of a second node type, `kind-2`: that type is
/// drawn only once an expansion reaches a leaf. Returns the store and the two type ids.
fn ring_and_typed_leaves() -> (Seeded, String, String) {
    let (ring, leaf, link) = (gid(1, 1), gid(1, 2), gid(2, 1));
    let mut seed = seed_head(
        &[
            (ring.clone(), "kind-1", vec![]),
            (leaf.clone(), "kind-2", vec![]),
        ],
        &link,
    );
    for n in 1..=400 {
        let kind = if n <= 300 { &ring } else { &leaf };
        seed.push_str(&node_yaml(
            &gid(3, n),
            kind,
            &format!("entity-{n}"),
            &[],
            " {}",
        ));
    }
    seed.push_str("    edges:\n");
    let mut e = 0;
    for k in 1..=300 {
        e += 1;
        seed.push_str(&edge_yaml(
            &gid(4, e),
            &link,
            &gid(3, k),
            &gid(3, k % 300 + 1),
        ));
        e += 1;
        seed.push_str(&edge_yaml(
            &gid(4, e),
            &link,
            &gid(3, k),
            &gid(3, (k + 1) % 300 + 1),
        ));
    }
    for n in 301..=400 {
        e += 1;
        seed.push_str(&edge_yaml(&gid(4, e), &link, &gid(3, n), &gid(3, n - 300)));
    }
    seed.push_str("    assertions: {}\n    evidence: {}\nevidence_payloads: {}\n");
    (Seeded::new(&seed, &[]), ring, leaf)
}

/// Markup that runs script if it is ever parsed as HTML, around text that is not ASCII (two- and
/// four-byte UTF-8), as one YAML single-quoted scalar.
fn hostile(tag: &str) -> String {
    format!(
        "<img src=x onerror=\"document.documentElement.setAttribute('data-pwned','{tag}')\">é😀<b>{tag}</b><script>document.documentElement.setAttribute('data-pwned','{tag}-s')</script>"
    )
}

fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "''"))
}

/// A store whose every text is hostile: node names, an alias, a String property value (on the
/// node and in an assertion), and evidence bytes; with an event type (an epoch-ms Integer on every
/// instance) so the timeline has subjects and events. Returns the store, the subject's id, the
/// evidence id, and every node's id with its name.
fn hostile_store() -> (Seeded, String, String, Vec<(String, String)>) {
    let (subject_kind, event_kind, link) = (gid(1, 1), gid(1, 2), gid(2, 1));
    let (note, at) = (gid(7, 2), gid(7, 1));
    let mut seed = seed_head(
        &[
            (
                subject_kind.clone(),
                "kind-s",
                vec![(note.clone(), "field-s", "String")],
            ),
            (
                event_kind.clone(),
                "kind-e",
                vec![(at.clone(), "field-e", "Integer")],
            ),
        ],
        &link,
    );
    let valid = 1_700_000_000_000_i64 + DAY;
    let mut names = Vec::new();
    let subject = gid(3, 1);
    let value = hostile("v1");
    let props = format!(
        "\n          {note}:\n          - value_kind: String\n            value: {}",
        quoted(&value)
    );
    seed.push_str(&node_yaml(
        &subject,
        &subject_kind,
        &quoted(&hostile("s1")),
        &[quoted(&hostile("a1"))],
        &props,
    ));
    names.push((subject.clone(), hostile("s1")));
    seed.push_str(&node_yaml(
        &gid(3, 2),
        &subject_kind,
        &quoted(&hostile("s2")),
        &[],
        " {}",
    ));
    names.push((gid(3, 2), hostile("s2")));
    for n in 1..=4_usize {
        let time = 1_700_000_000_000_i64 + i64::try_from(n).unwrap() * DAY;
        let props = format!(
            "\n          {at}:\n          - value_kind: Integer\n            value: {time}"
        );
        let name = hostile(&format!("e{n}"));
        seed.push_str(&node_yaml(
            &gid(3, 10 + n),
            &event_kind,
            &quoted(&name),
            &[],
            &props,
        ));
        names.push((gid(3, 10 + n), name));
    }
    seed.push_str("    edges:\n");
    for n in 1..=4_usize {
        let target = if n < 4 { gid(3, 1) } else { gid(3, 2) };
        seed.push_str(&edge_yaml(&gid(4, n), &link, &gid(3, 10 + n), &target));
    }
    // evidence bytes: a line carrying the facts' valid time, so the page shows it as the source
    let evidence = gid(5, 1);
    let dir = tempfile::tempdir().unwrap();
    let payload = dir.path().join("payload");
    std::fs::write(
        &payload,
        format!("first line\n{valid} {}\nlast line\n", hostile("ev")),
    )
    .unwrap();
    let hashed = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(["hash", &payload.display().to_string()])
        .output()
        .unwrap();
    let hashed: Value = serde_json::from_slice(&hashed.stdout).unwrap();
    let hash = hashed["content_hash"].as_str().unwrap().to_owned();
    let bytes = hashed["payload_yaml"].as_str().unwrap().to_owned();
    let root = gid(0, 2);
    seed.push_str(&format!(
        "    assertions:\n      {a1}:\n        id: {a1}\n        root_id: {root}\n        subject: !Node {subject}\n        predicate: !Property {note}\n        object: !Value\n          value_kind: String\n          value: {v}\n        evidence:\n        - {evidence}\n        proposed_by: {OPERATOR}\n        assessment: Proposed\n        lifecycle: Active\n        valid_time:\n          from: {valid}\n          to: null\n        transaction_time:\n          recorded_from: 0\n          recorded_to: null\n      {a2}:\n        id: {a2}\n        root_id: {root}\n        subject: !Node {e1}\n        predicate: !Relation {link}\n        object: !Node {subject}\n        evidence:\n        - {evidence}\n        proposed_by: {OPERATOR}\n        assessment: Proposed\n        lifecycle: Active\n        valid_time:\n          from: {valid}\n          to: null\n        transaction_time:\n          recorded_from: 0\n          recorded_to: null\n",
        a1 = gid(6, 1),
        a2 = gid(6, 2),
        v = quoted(&value),
        e1 = gid(3, 11),
    ));
    seed.push_str(&format!(
        "    evidence:\n      {evidence}:\n        id: {evidence}\n        source: !HumanStatement\n          identity: generated\n        content_hash: {hash}\n        extracted_by: {OPERATOR}\n        observed_at: 1700000000000\n        confidence: 10000\nevidence_payloads:\n  {hash}: {bytes}\n"
    ));
    (Seeded::new(&seed, &[]), subject, evidence, names)
}

// ---- the front server --------------------------------------------------------------------------

/// Answers a request target itself, writing to the connection, and says so; `false` forwards it.
type Answer = Box<dyn Fn(&str, &mut TcpStream) -> bool + Send + Sync>;

struct Shared {
    upstream: String,
    page: Vec<u8>,
    /// Re-frame every chunked answer so each chunk ends after a UTF-8 lead byte, this many ms apart.
    piece: Option<usize>,
    answer: Option<Answer>,
}

struct Front {
    url: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Front {
    fn start(upstream: &Server, probe: &str, piece: Option<usize>, answer: Option<Answer>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let stop = Arc::new(AtomicBool::new(false));
        let halt = Arc::clone(&stop);
        let shared = Arc::new(Shared {
            upstream: upstream.address(),
            page: with_probe(probe).into_bytes(),
            piece,
            answer,
        });
        let thread = std::thread::spawn(move || {
            for connection in listener.incoming() {
                if halt.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(connection) = connection else { continue };
                let shared = Arc::clone(&shared);
                std::thread::spawn(move || front_answer(connection, &shared));
            }
        });
        Self {
            url,
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Front {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let address = self.url.trim_start_matches("http://").trim_end_matches('/');
        TcpStream::connect(address).ok();
        if let Some(thread) = self.thread.take() {
            thread.join().ok();
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// Every request a front server took and when it finished answering it, in milliseconds since the
/// first: the network log a browser run that stalls or fails prints (issue #81).
static TRAFFIC: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn traffic(line: String) {
    static ORIGIN: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let at = ORIGIN.get_or_init(Instant::now).elapsed().as_millis();
    TRAFFIC
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .push(format!("{at:>8} ms {line}"));
}

/// Records the end of one request's answer, however [`front_answer`] returns.
struct Answered(String);

impl Drop for Answered {
    fn drop(&mut self) {
        traffic(format!("answered {}", self.0));
    }
}

fn front_answer(mut stream: TcpStream, shared: &Shared) {
    stream.set_read_timeout(Some(Duration::from_secs(60))).ok();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut request = String::new();
    if reader.read_line(&mut request).unwrap_or(0) == 0 {
        return;
    }
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
            break;
        }
    }
    let target = request.split_whitespace().nth(1).unwrap_or("").to_owned();
    if std::env::var_os("EKR_ADVERSARY_TRACE").is_some() {
        eprintln!("front: {target}");
    }
    traffic(format!("request  {target}"));
    let _answered = Answered(target.clone());
    if target.split('?').next() == Some("/") {
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
            shared.page.len()
        );
        stream.write_all(head.as_bytes()).ok();
        stream.write_all(&shared.page).ok();
        return;
    }
    if shared
        .answer
        .as_ref()
        .is_some_and(|answer| answer(&target, &mut stream))
    {
        return;
    }
    let Ok(mut up) = TcpStream::connect(&shared.upstream) else {
        return;
    };
    let upstream = &shared.upstream;
    write!(
        up,
        "GET {target} HTTP/1.1\r\nHost: {upstream}\r\nConnection: close\r\n\r\n"
    )
    .ok();
    let mut raw = Vec::new();
    up.read_to_end(&mut raw).ok();
    let Some(end) = find(&raw, b"\r\n\r\n") else {
        return;
    };
    let chunked = String::from_utf8_lossy(&raw[..end])
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked");
    match shared.piece {
        Some(pause) if chunked => {
            stream.write_all(&raw[..end + 4]).ok();
            // every chunk ends right after a UTF-8 lead byte, so each character that is not ASCII,
            // and the line it stands in, straddles two chunks, `pause` ms apart
            let body = dechunk(&raw[end + 4..]);
            for part in body.split_inclusive(|byte| *byte >= 0xc0) {
                let framed = [format!("{:x}\r\n", part.len()).as_bytes(), part, b"\r\n"].concat();
                if stream
                    .write_all(&framed)
                    .and_then(|()| stream.flush())
                    .is_err()
                {
                    return;
                }
                std::thread::sleep(Duration::from_millis(u64::try_from(pause).unwrap()));
            }
            stream.write_all(b"0\r\n\r\n").ok();
        }
        _ => {
            stream.write_all(&raw).ok();
        }
    }
}

fn dechunk(mut body: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    while let Some(at) = find(body, b"\r\n") {
        let size =
            usize::from_str_radix(std::str::from_utf8(&body[..at]).unwrap().trim(), 16).unwrap();
        if size == 0 {
            break;
        }
        out.extend_from_slice(&body[at + 2..at + 2 + size]);
        body = &body[at + 2 + size + 2..];
    }
    out
}

/// A streamed `/expand` answer as `ekr view` frames it: `lines`, 256 to a chunk.
fn ndjson_response(lines: &[String]) -> Vec<u8> {
    let mut out = b"HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n".to_vec();
    for group in lines.chunks(256) {
        let text: String = group.concat();
        out.extend_from_slice(format!("{:x}\r\n{text}\r\n", text.len()).as_bytes());
    }
    out.extend_from_slice(b"0\r\n\r\n");
    out
}

// ---- the browser -------------------------------------------------------------------------------

fn browser() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("EKR_VIEW_BROWSER") {
        return Some(PathBuf::from(path)).filter(|path| path.is_file());
    }
    let cache = PathBuf::from(std::env::var("HOME").ok()?).join(".cache/ms-playwright");
    let mut shells: Vec<PathBuf> = std::fs::read_dir(cache)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("chromium_headless_shell-"))
        })
        .map(|path| path.join("chrome-headless-shell-linux64/chrome-headless-shell"))
        .filter(|path| path.is_file())
        .collect();
    shells.sort();
    shells.pop()
}

fn one_browser() -> std::sync::MutexGuard<'static, ()> {
    static BROWSER: Mutex<()> = Mutex::new(());
    BROWSER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The DOM after `budget` ms of virtual time, as `--dump-dom --virtual-time-budget` writes it.
/// Headless Chromium's virtual time now and then stalls with nothing loading and nothing running
/// (seen twice in about twenty runs of these cases, and not tied to any one of them), so a stalled
/// run is ended and tried again, up to [`DUMP_ATTEMPTS`] runs.
///
/// A stall is told by the page's own state, not by how long the run takes (issue #81). The run is
/// driven over the DevTools protocol under the same virtual-time policy `--virtual-time-budget`
/// sets, and the page's clock (`performance.now()`, which reads virtual time) is watched: a run is
/// stalled once that clock has not moved for [`STALL_WINDOW`] while no request is in flight at the
/// front server. A slow run on a loaded machine (a CI runner, a gate at load 30–45) keeps moving
/// its clock and is waited for up to [`DUMP_CEILING`]. The two-minute kill this replaces cut such
/// runs short, and on 2026-10-06 a CI run failed on two of them in a row. Neither "no CPU" nor a
/// CPU rate tells the two apart: measured that day, a stalled browser's processes used about 25
/// clock ticks in 30 s, and a working one on a machine under `stress` as few as 7.
fn dump(browser: &Path, url: &str, budget: u32) -> String {
    let _one = one_browser();
    for _ in 0..DUMP_ATTEMPTS {
        if let Some(dom) = dump_once(browser, url, budget) {
            return dom;
        }
    }
    panic!("the browser stalled {DUMP_ATTEMPTS} times on {url}");
}

/// How many runs a dump gets before a stall fails the case.
const DUMP_ATTEMPTS: usize = 3;
/// How long the page's virtual clock may stand still, with nothing in flight, before the run is
/// taken for stalled.
const STALL_WINDOW: Duration = Duration::from_secs(30);
/// The longest one run may take while its clock keeps moving.
const DUMP_CEILING: Duration = Duration::from_secs(600);

/// One DevTools protocol connection to a page, the events it has seen kept in order.
struct Cdp {
    socket: tungstenite::WebSocket<TcpStream>,
    next: u64,
    events: Vec<Value>,
    /// The page's console messages, exceptions and log entries, one line each.
    console: Vec<String>,
}

impl Cdp {
    /// The reply to `method`, the events before it kept.
    fn call(&mut self, method: &str, params: Value) -> Result<Value, String> {
        self.next += 1;
        let id = self.next;
        let command = serde_json::json!({"id": id, "method": method, "params": params}).to_string();
        self.socket
            .send(tungstenite::Message::Text(command.into()))
            .map_err(|error| format!("{method}: sending: {error}"))?;
        loop {
            let message = self
                .socket
                .read()
                .map_err(|error| format!("{method}: reading: {error}"))?;
            let tungstenite::Message::Text(text) = message else {
                continue;
            };
            let value: Value = serde_json::from_str(&text).unwrap();
            if value["id"] == id {
                return match value.get("error") {
                    Some(error) => Err(format!("{method}: {error}")),
                    None => Ok(value["result"].clone()),
                };
            }
            let params = &value["params"];
            if value["method"] == "Fetch.requestPaused" {
                // a library the page loads, answered from the fixtures; the reply is passed over
                if let Some((answer, answered)) = viewer_libraries::answer(params) {
                    self.console.push(format!(
                        "library {answer} {}",
                        params["request"]["url"].as_str().unwrap_or_default()
                    ));
                    self.next += 1;
                    let command =
                        serde_json::json!({"id": self.next, "method": answer, "params": answered});
                    self.socket
                        .send(tungstenite::Message::Text(command.to_string().into()))
                        .map_err(|error| format!("{answer}: sending: {error}"))?;
                }
                continue;
            }
            if value.get("error").is_some() {
                self.console.push(format!("protocol error: {value}"));
            }
            match value["method"].as_str() {
                Some("Runtime.consoleAPICalled") => self.console.push(format!(
                    "console.{}: {}",
                    params["type"].as_str().unwrap_or(""),
                    params["args"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|arg| arg.get("value").map_or_else(
                            || arg["description"].as_str().unwrap_or("").to_owned(),
                            ToString::to_string
                        ))
                        .collect::<Vec<_>>()
                        .join(" ")
                )),
                Some("Runtime.exceptionThrown") => self.console.push(format!(
                    "exception: {}",
                    params["exceptionDetails"]["exception"]["description"]
                        .as_str()
                        .or_else(|| params["exceptionDetails"]["text"].as_str())
                        .unwrap_or("")
                )),
                Some("Log.entryAdded") => self.console.push(format!(
                    "log.{}: {} {}",
                    params["entry"]["level"].as_str().unwrap_or(""),
                    params["entry"]["text"].as_str().unwrap_or(""),
                    params["entry"]["url"].as_str().unwrap_or("")
                )),
                _ => {}
            }
            self.events.push(value);
        }
    }

    /// The value `expression` evaluates to in the page.
    fn eval(&mut self, expression: &str) -> Result<Value, String> {
        let result = self.call(
            "Runtime.evaluate",
            serde_json::json!({"expression": expression, "returnByValue": true}),
        )?;
        Ok(result["result"]["value"].clone())
    }
}

/// The address of the DevTools socket of the browser's page target, once it has one.
fn page_socket(port: u16) -> String {
    let start = Instant::now();
    loop {
        let mut list = TcpStream::connect(("127.0.0.1", port)).unwrap();
        list.set_read_timeout(Some(Duration::from_secs(30)))
            .unwrap();
        write!(
            list,
            "GET /json/list HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        // the DevTools server may keep the connection open: the body is read by its length
        let mut listed = BufReader::new(list);
        let mut length = 0;
        loop {
            let mut line = String::new();
            listed.read_line(&mut line).unwrap();
            if line == "\r\n" || line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.trim().parse().unwrap();
                }
            }
        }
        let mut body = vec![0_u8; length];
        listed.read_exact(&mut body).unwrap();
        let targets: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
        if let Some(address) = targets
            .as_array()
            .into_iter()
            .flatten()
            .find(|target| target["type"] == "page")
            .and_then(|target| target["webSocketDebuggerUrl"].as_str())
        {
            return address.to_owned();
        }
        assert!(
            start.elapsed() < Duration::from_secs(60),
            "the browser listed no page target: {targets}"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Requests the front server has taken since entry `from` of [`TRAFFIC`] and not finished
/// answering.
fn in_flight(from: usize) -> usize {
    let traffic = TRAFFIC
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let taken = traffic[from..]
        .iter()
        .filter(|line| line.contains(" ms request "))
        .count();
    taken.saturating_sub(traffic.len() - from - taken)
}

fn dump_once(browser: &Path, url: &str, budget: u32) -> Option<String> {
    let profile = tempfile::tempdir().unwrap();
    let from = TRAFFIC
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .len();
    // `taskset` and `nice` exec the browser, so the child is the browser itself
    let mut child = Command::new("taskset")
        .args(["-c", "0-3", "nice", "-n", "19"])
        .arg(browser)
        .args([
            "--headless",
            "--use-angle=swiftshader",
            "--enable-unsafe-swiftshader",
            "--no-sandbox",
            "--no-first-run",
            "--disable-component-update",
            "--disable-background-networking",
            "--disable-extensions",
            viewer_libraries::UNRESOLVABLE,
            "--window-size=1600,1000",
            "--remote-debugging-port=0",
            &format!("--user-data-dir={}", profile.path().display()),
            "about:blank",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut lines = BufReader::new(child.stderr.take().unwrap());
    let port: u16 = loop {
        let mut line = String::new();
        assert!(
            lines.read_line(&mut line).unwrap() > 0,
            "the browser printed no DevTools address"
        );
        if let Some(rest) = line
            .trim()
            .strip_prefix("DevTools listening on ws://127.0.0.1:")
        {
            break rest.split('/').next().unwrap().parse().unwrap();
        }
    };
    std::thread::spawn(move || std::io::copy(&mut lines, &mut std::io::sink()).ok());
    let address = page_socket(port);
    let tcp = TcpStream::connect(("127.0.0.1", port)).unwrap();
    tcp.set_read_timeout(Some(Duration::from_secs(60))).unwrap();
    let (socket, _) = tungstenite::client(address, tcp).unwrap();
    let mut cdp = Cdp {
        socket,
        next: 0,
        events: Vec::new(),
        console: Vec::new(),
    };
    let start = Instant::now();
    // the page, its virtual clock held until it is there, then what `--virtual-time-budget` sets:
    // a budget set on the idle blank page would run out before the page arrived
    let ran = (|| -> Result<Result<String, String>, String> {
        cdp.call("Runtime.enable", serde_json::json!({}))?;
        cdp.call("Log.enable", serde_json::json!({}))?;
        viewer_libraries::serve(|method, params| cdp.call(method, params))?;
        cdp.call("Page.navigate", serde_json::json!({ "url": url }))?;
        cdp.call(
            "Emulation.setVirtualTimePolicy",
            serde_json::json!({"policy": "pauseIfNetworkFetchesPending", "budget": budget}),
        )?;
        let (mut clock, mut moved) = (Value::Null, Instant::now());
        loop {
            if cdp
                .events
                .iter()
                .any(|event| event["method"] == "Emulation.virtualTimeBudgetExpired")
            {
                return Ok(Ok(cdp
                    .eval("document.documentElement.outerHTML")?
                    .as_str()
                    .unwrap_or("")
                    .to_owned()));
            }
            let now = cdp.eval("performance.now()")?;
            if now != clock || in_flight(from) > 0 {
                (clock, moved) = (now, Instant::now());
            }
            if moved.elapsed() >= STALL_WINDOW {
                return Ok(Err(format!(
                    "the page's virtual clock stood at {clock} ms for {STALL_WINDOW:?} with no request in flight, {:?} into the run",
                    start.elapsed()
                )));
            }
            if start.elapsed() >= DUMP_CEILING {
                return Ok(Err(format!(
                    "the page's virtual clock was at {clock} ms of {budget} after {DUMP_CEILING:?}"
                )));
            }
            std::thread::sleep(Duration::from_millis(200));
        }
    })();
    child.kill().ok();
    child.wait().ok();
    // what the run requested and the page logged
    let log = || {
        let traffic = TRAFFIC
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)[from..]
            .join("\n");
        format!(
            "requests through the front server:\n{traffic}\nthe page's console ({} lines):\n{}",
            cdp.console.len(),
            cdp.console.join("\n")
        )
    };
    match ran {
        Ok(Ok(dom)) => Some(dom),
        Ok(Err(stalled)) => {
            eprintln!("the browser stalled on {url}: {stalled}\n{}", log());
            None
        }
        Err(protocol) => panic!("the browser failed on {url}: {protocol}\n{}", log()),
    }
}

/// The page, then the probe `body` inside an async function with `out`, `sleep` and `until`; what
/// it leaves in `out` is written as JSON into `<pre id="probe-out">`.
fn with_probe(body: &str) -> String {
    let mut text = page();
    text.push_str(
        "<script>\n(async () => {\n  const out = {};\n  const sleep = ms => new Promise(r => setTimeout(r, ms));\n  const until = async (f, ms = 20000) => { const t0 = Date.now(); for (;;) { let ok = false; try { ok = f(); } catch (e) {} if (ok) return true; if (Date.now() - t0 > ms) return false; await sleep(50); } };\n  try {\n",
    );
    text.push_str(body);
    text.push_str(
        "\n  } catch (e) { out.error = String(e && e.stack || e); }\n  const pre = document.createElement('pre'); pre.id = 'probe-out'; pre.textContent = JSON.stringify(out); document.body.appendChild(pre);\n})();\n</script>\n",
    );
    text
}

fn probed(dom: &str) -> Value {
    let marker = "<pre id=\"probe-out\">";
    let start = dom
        .find(marker)
        .unwrap_or_else(|| panic!("the probe wrote nothing: {dom}"))
        + marker.len();
    let end = start + dom[start..].find("</pre>").unwrap();
    let text = dom[start..end]
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&nbsp;", "\u{a0}")
        .replace("&amp;", "&");
    serde_json::from_str(&text).unwrap_or_else(|error| panic!("{error}: {text}"))
}

macro_rules! need_browser {
    () => {
        match browser() {
            Some(browser) => browser,
            None => {
                eprintln!("skipped: no headless Chromium (set EKR_VIEW_BROWSER to one)");
                return;
            }
        }
    };
}

// ---- 5. URL state ------------------------------------------------------------------------------

/// A shared address naming a revision the store does not hold (another store's, or one past this
/// store's head) must still open the page. Going to a revision past the head from inside the page
/// (`goRevision`, a history step through `applyState`) is clamped to the head; the first load
/// passes the address's number to `/overview` unchecked, and a `RevisionNotFound` there stops the
/// page for good.
#[test]
fn an_address_naming_a_revision_past_the_head_still_opens_the_page() {
    let browser = need_browser!();
    let seeded = two_revisions();
    let server = seeded.serve();
    // through the front, with a probe that reads nothing, so `EKR_ADVERSARY_PAGE` can name the base page
    let front = Front::start(&server, "", None, None);
    for revision in ["7", "99999999999999999999"] {
        let dom = dump(
            &browser,
            &format!("{}#revision={revision}", front.url),
            8000,
        );
        // the page's own script text carries the failure phrase, and `--dump-dom` serialises it: only
        // the error element says whether the page started
        let error = dom
            .split("<div id=\"error\">")
            .nth(1)
            .and_then(|rest| rest.split("</div>").next())
            .unwrap_or("");
        assert!(
            !error.contains("the viewer did not start"),
            "#revision={revision} on a store whose head is 1: the page did not start: {error}"
        );
        assert!(
            dom.contains("head · revision 1"),
            "#revision={revision}: the page shows the head"
        );
    }
}

/// A node that only an expansion drew (it is not among the overview's top) can be given a
/// neighbourhood focus; the page writes that focus into the address (`focus=<id>&hops=1`). Loading
/// that very address must restore it. `applyState` filters the focus trail by what the graph holds
/// before the node's own expansion has arrived, so the focus is dropped, and the closing
/// `replaceState` then rewrites the address without it.
#[test]
fn a_neighbourhood_focus_on_a_streamed_in_node_survives_a_reload_of_its_own_address() {
    let browser = need_browser!();
    let seeded = ring_and_leaves();
    let server = seeded.serve();
    let leaf = gid(3, 350);
    let top: Vec<String> = server.get("/overview")["top"]
        .as_array()
        .unwrap()
        .iter()
        .map(|node| node["id"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        !top.contains(&leaf),
        "the leaf is not among the overview's top"
    );
    let probe = format!(
        r##"
    const X = "{leaf}";
    const phase = new URLSearchParams(location.search).get("phase");
    const crumbs = () => document.getElementById("crumbs");
    await until(() => window.__viewer && __viewer.graph.hasNode(X), 30000);
    if (phase === "1") {{
      await until(() => document.querySelector('[data-act=scope][data-id="' + X + '"]'));
      await sleep(300);
      document.querySelector('[data-act=scope][data-id="' + X + '"]').click();
      await until(() => crumbs().style.display === "flex");
    }} else {{
      await sleep(3000);
    }}
    out.hash = location.hash;
    out.crumbs = crumbs().style.display;
    out.trail = crumbs().textContent;
"##
    );
    let front = Front::start(&server, &probe, None, None);
    let first = probed(&dump(
        &browser,
        &format!("{}?phase=1#node={leaf}", front.url),
        20000,
    ));
    let written = first["hash"].as_str().unwrap().to_owned();
    assert_eq!(
        first["crumbs"], "flex",
        "phase 1: the focus was set: {first}"
    );
    assert!(
        written.contains(&format!("focus={leaf}")),
        "phase 1: the page wrote the focus into the address: {first}"
    );
    let second = probed(&dump(
        &browser,
        &format!("{}?phase=2{written}", front.url),
        20000,
    ));
    assert_eq!(
        second["crumbs"], "flex",
        "reloading the page's own address {written} drops its neighbourhood focus on a node an \
         expansion drew; the address is rewritten to {}",
        second["hash"]
    );
}

/// A path the page wrote (`path=a~b`) between two nodes only expansions drew must be found again
/// when that address is loaded: both ends are streamed in, and the path is looked for once the
/// edges are drawn, not before.
#[test]
fn a_path_between_streamed_in_nodes_survives_a_reload_of_its_own_address() {
    let browser = need_browser!();
    let seeded = ring_and_leaves();
    let server = seeded.serve();
    let (a, b) = (gid(3, 350), gid(3, 360));
    let probe = format!(
        r##"
    await until(() => window.__viewer && __viewer.graph.hasNode("{a}") && __viewer.graph.hasNode("{b}"), 12000);
    await until(() => /Path ·/.test(document.getElementById("panel").textContent), 4000);
    await sleep(1000);
    out.hash = location.hash;
    out.panel = document.getElementById("panel").textContent;
"##
    );
    let front = Front::start(&server, &probe, None, None);
    let out = probed(&dump(
        &browser,
        &format!("{}#view=2d&path={a}~{b}", front.url),
        20000,
    ));
    assert!(
        out["panel"].as_str().unwrap().contains("Path · 7 steps"),
        "the path between two streamed-in leaves is shown again: {out}"
    );
    let hash = out["hash"].as_str().unwrap();
    assert!(
        hash.contains("path=") && hash.contains(&a) && hash.contains(&b),
        "the address keeps its path: {out}"
    );
}

/// A type hidden by the address (`types=` lists the ones shown) stays hidden when an expansion
/// draws its first node after the address was read, and the address keeps saying so.
#[test]
fn a_type_the_address_hides_stays_hidden_when_a_stream_first_draws_it() {
    let browser = need_browser!();
    let (seeded, ring, _) = ring_and_typed_leaves();
    let server = seeded.serve();
    let (hub, leaf) = (gid(3, 50), gid(3, 350));
    let probe = format!(
        r##"
    await until(() => window.__viewer && __viewer.graph.hasNode("{leaf}") && __viewer.renderer, 30000);
    await sleep(2000);
    const d = __viewer.renderer.getNodeDisplayData("{leaf}");
    out.leafHidden = !!(d && d.hidden);
    out.hash = location.hash;
"##
    );
    let front = Front::start(&server, &probe, None, None);
    let out = probed(&dump(
        &browser,
        &format!("{}#view=2d&node={hub}&types={ring}", front.url),
        20000,
    ));
    assert_eq!(
        out["leafHidden"], true,
        "a node of a type the address hides was drawn: {out}"
    );
    assert!(
        out["hash"]
            .as_str()
            .unwrap()
            .contains(&format!("types={ring}")),
        "the address keeps the types it shows: {out}"
    );
}

// ---- 3. a late answer after the revision changed -----------------------------------------------

/// The search's hits are a read of the revision shown. After the page moves to another revision,
/// what the hits list must be a read of that revision: here, `entity-late` does not exist at
/// revision 0, and `/search` of revision 0 answers no match. The page keeps listing the head's hit.
#[test]
fn the_hits_listed_after_a_revision_change_are_the_new_revisions() {
    let browser = need_browser!();
    let seeded = two_revisions();
    let server = seeded.serve();
    let probe = r##"
    await until(() => window.__viewer && document.querySelector("#hits .hit"), 30000);
    out.before = [...document.querySelectorAll("#hits .hit")].map(e => e.dataset.node);
    out.beforeRevision = __viewer.doc.meta.revision;
    await __viewer.goRevision(0);
    await until(() => __viewer.doc.meta.revision === 0);
    await sleep(2000);
    out.afterRevision = __viewer.doc.meta.revision;
    out.after = [...document.querySelectorAll("#hits .hit")].map(e => e.dataset.node);
    out.afterText = document.getElementById("hits").textContent;
    const truth = await (await fetch("/search?q=entity-late&limit=25&revision=0")).json();
    out.truth = truth.matches.map(m => m.id);
"##;
    let front = Front::start(&server, probe, None, None);
    let out = probed(&dump(
        &browser,
        &format!("{}#q=entity-late", front.url),
        20000,
    ));
    assert_eq!(out["before"], serde_json::json!([gid(3, 9)]), "{out}");
    assert_eq!(out["afterRevision"], 0, "{out}");
    assert_eq!(out["truth"], serde_json::json!([]), "{out}");
    assert_eq!(
        out["after"], out["truth"],
        "at revision 0 the hits still list the head's match: {}",
        out["afterText"]
    );
}

// ---- 4. the render budget ----------------------------------------------------------------------

/// Past the render budget of 20,000 nodes, "labels go by degree": the page's own constant says the
/// 2,000 best-connected keep a label (`LABELLED`). The cut is `deg < degrees[2000]`, so every node
/// that ties the 2,001st degree keeps its label too; on a graph whose degrees repeat (every real
/// one), several times 2,000 nodes stay labelled. The state is reached after 41 pages of "load more"
/// on any store over 20,000 nodes; here one streamed answer of 20,500 nodes stands in for them.
#[test]
fn past_the_render_budget_at_most_the_two_thousand_best_connected_keep_a_label() {
    let browser = need_browser!();
    let seeded = two_revisions();
    let server = seeded.serve();
    let kind = gid(1, 1);
    let answer: Answer = Box::new(move |target: &str, stream: &mut TcpStream| {
        if !(target.starts_with("/expand?") && target.contains("depth=0")) {
            return false;
        }
        let count = 20_500_usize;
        let mut lines = vec![format!(
            "{{\"kind\":\"meta\",\"format\":\"ekr.graph-slice/1\",\"revision\":1,\"seeds\":[],\"depth\":0,\"after\":0,\"node_total\":{count},\"edge_total\":0}}\n"
        )];
        for n in 0..count {
            lines.push(format!(
                "{{\"kind\":\"node\",\"id\":\"{}\",\"type\":\"{kind}\",\"name\":\"n-{n}\",\"degree\":{},\"distance\":0}}\n",
                gid(9, n + 1),
                1 + n % 4
            ));
        }
        lines.push("{\"kind\":\"end\",\"next\":null,\"remaining\":0}\n".to_owned());
        stream.write_all(&ndjson_response(&lines)).ok();
        true
    });
    let probe = r##"
    await until(() => window.__viewer && __viewer.graph.order > 20000 && __viewer.renderer, 60000);
    await sleep(3000);
    const r = __viewer.renderer;
    let shown = 0, labelled = 0;
    __viewer.graph.forEachNode(id => { const d = r.getNodeDisplayData(id); if (!d || d.hidden) return; shown++; if (d.label) labelled++; });
    out.order = __viewer.graph.order; out.shown = shown; out.labelled = labelled;
"##;
    let front = Front::start(&server, probe, None, Some(answer));
    let out = probed(&dump(&browser, &front.url, 40000));
    assert!(out["order"].as_u64().unwrap() > 20_000, "{out}");
    assert!(
        out["labelled"].as_u64().unwrap() <= 2000,
        "past the budget, {} of {} drawn nodes keep a label; the page's rule is the 2,000 best connected",
        out["labelled"],
        out["shown"]
    );
}

// ---- 1. XSS, through every address, with the stream re-framed across UTF-8 sequences ------------

const XSS_CHECK: &str = r##"
    out.pwned = document.documentElement.getAttribute("data-pwned");
    out.elements = [...document.querySelectorAll("img, iframe, object, embed, video, audio")].length;
    out.handlers = [...document.querySelectorAll("*")].flatMap(e => [...e.attributes].filter(a => /^on/i.test(a.name)).map(a => e.tagName + "." + a.name));
    out.scripts = document.querySelectorAll("script").length;
    out.bolds = [...document.querySelectorAll("b")].map(b => b.textContent).filter(t => /^(s|a|p|v|e|ev)\d*(-s)?$/.test(t));
"##;

/// Names, an alias, a property value and evidence bytes carrying markup reach the page through
/// `/overview`, `/expand` (re-framed so lines and UTF-8 sequences split across reads), `/node/<id>`,
/// `/search`, `/evidence/<id>` and `/timeline`; none may become HTML, and each must arrive whole.
#[test]
fn store_text_through_every_address_and_a_split_stream_never_becomes_html() {
    let browser = need_browser!();
    let (seeded, subject, evidence, names) = hostile_store();
    let server = seeded.serve();
    let expected: serde_json::Map<String, Value> = names
        .iter()
        .map(|(id, name)| (id.clone(), Value::String(name.clone())))
        .collect();
    // inside the probe's own <script>: a `</` would end it
    let expected = Value::Object(expected).to_string().replace("</", "<\\/");
    let probe = format!(
        r##"
    const names = {expected};
    const phase = new URLSearchParams(location.search).get("phase");
    await until(() => window.__viewer && __viewer.graph.order > 0, 30000);
    if (phase === "graph") {{
      await until(() => document.querySelector("#evcard .evbody") && !/reading/.test(document.querySelector("#evcard .evbody").textContent), 20000);
      await until(() => [...document.querySelectorAll("#panel .src")].every(e => !/source…/.test(e.textContent)), 10000);
      await sleep(1500);
      document.dispatchEvent(new KeyboardEvent("keydown", {{key: "k", ctrlKey: true, bubbles: true}}));
      const q = document.getElementById("palQ"); q.value = "img"; q.dispatchEvent(new Event("input", {{bubbles: true}}));
      await sleep(300);
      out.panel = document.getElementById("panel").textContent;
      out.hits = document.getElementById("hits").textContent;
      out.palette = document.getElementById("palList").textContent;
      out.evidence = document.querySelector("#evcard .evbody").textContent;
      out.sources = [...document.querySelectorAll("#panel .src")].map(e => e.textContent);
      out.drawn = {{}};
      for (const id of Object.keys(names)) if (__viewer.graph.hasNode(id)) out.drawn[id] = __viewer.graph.getNodeAttribute(id, "name");
      out.names = names;
    }} else {{
      await until(() => document.querySelector(".tlrow"), 20000);
      out.rows = [...document.querySelectorAll(".tlrow .nm")].map(e => e.textContent);
      location.hash = "#mode=timeline&subject={subject}";
      await until(() => document.querySelector(".tllane"), 20000);
      await sleep(1000);
      out.head = document.getElementById("tlHead").textContent;
    }}
{XSS_CHECK}
"##
    );
    let front = Front::start(&server, &probe, Some(40), None);
    let graph = probed(&dump(
        &browser,
        &format!(
            "{}?phase=graph#node={subject}&evidence={evidence}&q=img",
            front.url
        ),
        30000,
    ));
    let timeline = probed(&dump(
        &browser,
        &format!("{}?phase=timeline#mode=timeline", front.url),
        30000,
    ));
    for (what, out) in [("graph", &graph), ("timeline", &timeline)] {
        assert!(out.get("error").is_none(), "{what}: {out}");
        assert_eq!(out["pwned"], Value::Null, "{what}: markup ran: {out}");
        assert_eq!(
            out["elements"], 0,
            "{what}: an element was parsed from store text: {out}"
        );
        assert_eq!(out["handlers"], serde_json::json!([]), "{what}: {out}");
        assert_eq!(out["bolds"], serde_json::json!([]), "{what}: {out}");
    }
    // the text did arrive, as text, and whole
    let s1 = hostile("s1");
    assert!(graph["panel"].as_str().unwrap().contains(&s1), "{graph}");
    assert!(
        graph["panel"].as_str().unwrap().contains(&hostile("a1")),
        "{graph}"
    );
    assert!(
        graph["panel"].as_str().unwrap().contains(&hostile("v1")),
        "{graph}"
    );
    assert!(
        graph["evidence"].as_str().unwrap().contains(&hostile("ev")),
        "{graph}"
    );
    assert!(graph["hits"].as_str().unwrap().contains("match"), "{graph}");
    assert_eq!(
        graph["drawn"], graph["names"],
        "a name split across reads arrived changed"
    );
    assert!(
        timeline["rows"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str() == Some(s1.as_str())),
        "{timeline}"
    );
    assert!(
        timeline["head"].as_str().unwrap().contains(&s1),
        "{timeline}"
    );
}

// ---- 3. stream handling: a malformed line, a stream cut mid-way ---------------------------------

const STREAM_HEAD: &str = "HTTP/1.1 200 OK\r\nContent-Type: application/x-ndjson\r\nTransfer-Encoding: chunked\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n";

fn chunk(stream: &mut TcpStream, text: &str) -> bool {
    stream
        .write_all(format!("{:x}\r\n{text}\r\n", text.len()).as_bytes())
        .and_then(|()| stream.flush())
        .is_ok()
}

fn meta_line(total: usize) -> String {
    format!(
        "{{\"kind\":\"meta\",\"format\":\"ekr.graph-slice/1\",\"revision\":1,\"seeds\":[],\"depth\":0,\"after\":0,\"node_total\":{total},\"edge_total\":0}}\n"
    )
}

fn node_line(id: &str, kind: &str, name: &str) -> String {
    format!(
        "{{\"kind\":\"node\",\"id\":\"{id}\",\"type\":\"{kind}\",\"name\":\"{name}\",\"degree\":1,\"distance\":0}}\n"
    )
}

/// The plan's transport: "closing the reader cancels it", and the contract: "a client that closes
/// the connection ends the stream; the server stops writing and frees the slot". When a line does
/// not parse, `Data.expand` rethrows and marks the expansion failed, but never cancels the reader,
/// so the connection it has given up on stays open while the server keeps writing; and the node
/// that parsed before the bad line in the same read is dropped with it. The real server writes no
/// malformed line; the case builds one.
#[test]
fn a_stream_the_page_gives_up_on_after_a_malformed_line_is_closed() {
    let browser = need_browser!();
    let seeded = two_revisions();
    let server = seeded.serve();
    let (kind, first) = (gid(1, 1), gid(9, 1));
    let closed: Arc<Mutex<Option<Option<u128>>>> = Arc::new(Mutex::new(None));
    let seen = Arc::clone(&closed);
    let (kind_, first_) = (kind.clone(), first.clone());
    let answer: Answer = Box::new(move |target: &str, stream: &mut TcpStream| {
        if target == "/slow" {
            // holds the page open for 3 s of real time: virtual time waits on a pending read
            std::thread::sleep(Duration::from_secs(3));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok")
                .ok();
            return true;
        }
        if !(target.starts_with("/expand?") && target.contains("depth=0")) {
            return false;
        }
        stream.write_all(STREAM_HEAD.as_bytes()).ok();
        let text = format!(
            "{}{}{{\"kind\":\"node\",\"id\"\n",
            meta_line(3),
            node_line(&first_, &kind_, "streamed-1")
        );
        chunk(stream, &text);
        // how long after the bad line the connection is closed: the first write that fails
        let start = Instant::now();
        let mut when = None;
        while start.elapsed() < Duration::from_secs(8) {
            std::thread::sleep(Duration::from_millis(100));
            if !chunk(stream, "{\"kind\":\"progress\",\"sent\":1}\n") {
                when = Some(start.elapsed().as_millis());
                break;
            }
        }
        *seen.lock().unwrap() = Some(when);
        stream.write_all(b"0\r\n\r\n").ok();
        true
    });
    let probe = format!(
        r##"
    await until(() => window.__viewer && /failed/.test(document.getElementById("stream").textContent), 30000);
    out.stream = document.getElementById("stream").textContent;
    out.parsedBeforeTheBadLine = __viewer.graph.hasNode("{first}");
    await fetch("/slow");
"##
    );
    let front = Front::start(&server, &probe, None, Some(answer));
    let out = probed(&dump(&browser, &front.url, 20000));
    // the browser has exited, which closes whatever it left open; wait for the stream to report
    let started = Instant::now();
    while closed.lock().unwrap().is_none() && started.elapsed() < Duration::from_secs(12) {
        std::thread::sleep(Duration::from_millis(50));
    }
    let closed = *closed.lock().unwrap();
    assert!(
        out["stream"].as_str().unwrap().contains("failed"),
        "the expansion is marked failed: {out}"
    );
    assert!(
        matches!(closed, Some(Some(ms)) if ms < 2000),
        "the page marked the expansion failed ({}) but did not close its connection: the first \
         failed write came {closed:?} ms after the bad line (the page stayed open for 3,000 ms of \
         it; the browser's exit closes the socket); a node parsed before the bad line in the same \
         read was drawn: {}",
        out["stream"],
        out["parsedBeforeTheBadLine"]
    );
    assert_eq!(
        out["parsedBeforeTheBadLine"], true,
        "a node parsed before the bad line in the same read is kept: {out}"
    );
}

/// A stream that ends without its `end` line (the server's 60 s write deadline, a dropped
/// connection) is reported failed, and what arrived before the cut stays drawn.
///
/// The cut comes after the page has drawn what arrived, never on a timer: a cut that reaches the
/// browser before the page has read the bytes ahead of it errors the response body and drops
/// those bytes unread (measured: 5 of 5 runs with the cut written right after the lines, and 10 of
/// 12 at load 50 with it 200 ms after them), so "what arrived" would be nothing the page could
/// keep; the page has no timeout of its own on this path. The probe asks for the cut (`/cut`) once
/// both nodes are drawn; the front server holds the stream open until then, or for 60 s of real
/// time when the page never draws them.
#[test]
fn a_stream_cut_before_its_end_line_is_failed_and_keeps_what_arrived() {
    let browser = need_browser!();
    let seeded = two_revisions();
    let server = seeded.serve();
    let (kind, a, b) = (gid(1, 1), gid(9, 1), gid(9, 2));
    let (kind_, a_, b_) = (kind.clone(), a.clone(), b.clone());
    let asked = Arc::new((Mutex::new(false), Condvar::new()));
    let answer: Answer = Box::new(move |target: &str, stream: &mut TcpStream| {
        let (cut, bell) = &*asked;
        if target == "/cut" {
            *cut.lock().unwrap() = true;
            bell.notify_all();
            stream
                .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n")
                .ok();
            return true;
        }
        if !(target.starts_with("/expand?") && target.contains("depth=0")) {
            return false;
        }
        stream.write_all(STREAM_HEAD.as_bytes()).ok();
        let text = format!(
            "{}{}{}",
            meta_line(5),
            node_line(&a_, &kind_, "streamed-a"),
            node_line(&b_, &kind_, "streamed-b")
        );
        chunk(stream, &text);
        // the cut, once the page has drawn both nodes; the request is answered once
        let asked = cut.lock().unwrap();
        let (mut asked, _) = bell
            .wait_timeout_while(asked, Duration::from_secs(60), |asked| !*asked)
            .unwrap();
        *asked = false;
        drop(asked);
        stream.shutdown(std::net::Shutdown::Both).ok();
        true
    });
    // A mutation observer, which the page's own drawing triggers, asks for the cut in the turn the
    // second node is drawn, rather than a timer polling for it.
    let probe = format!(
        r##"
    let askedForTheCut = false;
    const askForTheCut = () => {{
      if (askedForTheCut || !window.__viewer || !__viewer.graph.hasNode("{a}") || !__viewer.graph.hasNode("{b}")) return;
      askedForTheCut = true;
      fetch("/cut", {{cache: "no-store"}});
    }};
    new MutationObserver(askForTheCut).observe(document.body, {{childList: true, subtree: true, characterData: true}});
    askForTheCut();
    await until(() => window.__viewer && /failed|records/.test(document.getElementById("stream").textContent) && !/streaming/.test(document.getElementById("stream").textContent), 30000);
    // 500 ms more of virtual time: a removal the page schedules after marking the stream failed is seen
    await sleep(500);
    out.stream = document.getElementById("stream").textContent;
    out.drawn = [__viewer.graph.hasNode("{a}"), __viewer.graph.hasNode("{b}")];
"##
    );
    let front = Front::start(&server, &probe, None, Some(answer));
    let out = probed(&dump(&browser, &front.url, 20000));
    assert!(out["stream"].as_str().unwrap().contains("failed"), "{out}");
    assert_eq!(out["drawn"], serde_json::json!([true, true]), "{out}");
}
