//! `story:explain-bounds-documents`: `explain` with documents answers at most a stated number of
//! bytes of each evidence record, centred on the cited text, with the record's full length, and
//! `offset` and `limit` (bytes) read any other part of it through the same call.
//!
//! The store is seeded in process: one assertion of the example seed's ontology, Acme's
//! `legal_name`, whose value is the cited text, citing one evidence record of 5,000,000 bytes
//! that holds that text once, at byte 3,000,000. A transaction document cannot carry a payload
//! that large (`ekr.transaction-document/2` holds a sequence to 16,384 elements), so the seed is
//! the one way in. The record's filler holds two-byte characters, so a window that cuts one is
//! visible.
//!
//! Acceptance, on both providers, through `ekr explain` and the MCP `explain` tool:
//! 1. the default answer is at most 64 KiB per record, carries `truncated: true` and the full
//!    length, and contains the cited text;
//! 2. `offset: 0, limit: 1000` answers exactly the first 1,000 bytes of the record;
//! 3. reading the record in `limit`-sized steps from offset 0 to the full length reassembles
//!    its bytes exactly.

use std::io::{BufRead, BufReader, Write as _};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use base64::Engine as _;
use ekr::host::CliHostConfigurationV1;
use ekr_core::{ContentHash, Timestamp};
use ekr_kernel::{Runtime, SeedDocument};
use serde_json::{json, Value};

const ASSERTION: &str = "00000000-0000-4000-8000-000000000513";
const EVIDENCE: &str = "00000000-0000-4000-8000-000000000403";
const RECORD_BYTES: usize = 5_000_000;
const CITED_AT: usize = 3_000_000;
const CITED: &str = "Acme Widgets Holding (registered 2019, ledger entry 77-A)";
const DEFAULT_LIMIT: usize = 64 * 1024;

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

/// The evidence record: filler with two-byte characters, the cited text once at `CITED_AT`.
fn record() -> Vec<u8> {
    let filler = "Ledger line: the caf\u{e9} recorded nothing of note. ".as_bytes();
    let mut bytes = Vec::with_capacity(RECORD_BYTES);
    while bytes.len() < CITED_AT {
        bytes.extend_from_slice(filler);
    }
    bytes.truncate(CITED_AT);
    // Fill to a character boundary with ASCII, so the cited text starts exactly at CITED_AT.
    while std::str::from_utf8(&bytes).is_err() {
        bytes.pop();
    }
    bytes.resize(CITED_AT, b'.');
    bytes.extend_from_slice(CITED.as_bytes());
    while bytes.len() < RECORD_BYTES {
        bytes.extend_from_slice(filler);
    }
    bytes.truncate(RECORD_BYTES);
    while std::str::from_utf8(&bytes).is_err() {
        bytes.pop();
    }
    bytes.resize(RECORD_BYTES, b'.');
    assert_eq!(bytes.len(), RECORD_BYTES);
    assert!(std::str::from_utf8(&bytes).is_ok());
    assert_eq!(
        bytes
            .windows(CITED.len())
            .position(|window| window == CITED.as_bytes()),
        Some(CITED_AT)
    );
    bytes
}

/// The example seed plus Acme's `legal_name` assertion citing the large record.
fn seed(payload: &[u8]) -> SeedDocument {
    let example = String::from_utf8(
        ekr()
            .args(["example", "ekr-seed/2"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let hash = ContentHash::of_bytes(payload);
    let assertion = format!(
        "      {ASSERTION}:
        id: {ASSERTION}
        root_id: 00000000-0000-4000-8000-000000000002
        subject: !Node 00000000-0000-4000-8000-000000000303
        predicate: !Property 00000000-0000-4000-8000-000000000801
        object: !Value
          value_kind: String
          value: {CITED}
        evidence:
        - {EVIDENCE}
        proposed_by: 00000000-0000-4000-8000-000000000101
        assessment: Proposed
        lifecycle: Active
        valid_time:
          from: 1577836800000
          to: null
        transaction_time:
          recorded_from: 0
          recorded_to: null
    evidence:
      {EVIDENCE}:
        id: {EVIDENCE}
        source: !HumanStatement
          identity: Runtime operator
        content_hash: {hash}
        extracted_by: 00000000-0000-4000-8000-000000000101
        observed_at: 1773273600000
        confidence: 10000
"
    );
    let marker = "\n    evidence:\n";
    assert_eq!(example.matches(marker).count(), 1, "{example}");
    let text = example.replacen(marker, &format!("\n{assertion}"), 1);
    let mut document = SeedDocument::from_yaml(&text).unwrap();
    document
        .evidence_payloads
        .insert(hash, std::sync::Arc::new(payload.to_vec()));
    document
}

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn new(backend: &'static str, payload: &[u8]) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        let host = ekr().args(["example", "ekr.cli-host/1"]).output().unwrap();
        std::fs::write(world.host(), &host.stdout).unwrap();
        let host = CliHostConfigurationV1::from_json(&host.stdout).unwrap();
        let store = world.store();
        let runtime = match backend {
            "file" => Runtime::file(&store, &host.tenant, host.context, host.authority.clone()),
            _ => Runtime::sqlite(&store, &host.tenant, host.context, host.authority.clone()),
        }
        .unwrap();
        runtime
            .seed(seed(payload), || Timestamp::from_millis(10))
            .unwrap();
        world
    }
    fn host(&self) -> PathBuf {
        self.directory.path().join("host.json")
    }
    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }
    fn command(&self, verb: &[&str]) -> Command {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(self.host())
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb)
            .stdin(Stdio::null());
        command
    }
    fn run(&self, verb: &[&str]) -> Output {
        self.command(verb).output().unwrap()
    }
    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

/// An `ekr mcp` server past its handshake.
struct Server {
    child: std::process::Child,
    input: std::process::ChildStdin,
    output: BufReader<std::process::ChildStdout>,
    next: u64,
}

impl Server {
    fn start(world: &World) -> Self {
        let mut child = world
            .command(&["mcp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut server = Self {
            child,
            input,
            output,
            next: 1,
        };
        let initialized = server.exchange(&json!({"jsonrpc": "2.0", "id": 0,
            "method": "initialize", "params": {"protocolVersion": "2025-11-25",
            "capabilities": {}, "clientInfo": {"name": "explain-bounds", "version": "0"}}}));
        assert!(initialized.get("result").is_some(), "{initialized}");
        server
            .input
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .unwrap();
        server
    }
    fn exchange(&mut self, request: &Value) -> Value {
        self.input
            .write_all(request.to_string().as_bytes())
            .unwrap();
        self.input.write_all(b"\n").unwrap();
        self.input.flush().unwrap();
        let mut line = String::new();
        self.output.read_line(&mut line).unwrap();
        serde_json::from_str(&line).unwrap()
    }
    /// The tool's answer: the JSON-RPC response.
    fn call(&mut self, arguments: Value) -> Value {
        let id = self.next;
        self.next += 1;
        self.exchange(&json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
            "params": {"name": "explain", "arguments": arguments}}))
    }
    /// The tool's document, which must not be an error.
    fn explain(&mut self, arguments: Value) -> Value {
        let answer = self.call(arguments.clone());
        assert_eq!(
            answer["result"]["isError"], false,
            "{arguments}: {answer:.2000}"
        );
        answer["result"]["structuredContent"].clone()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The one Evidence link of an explanation.
fn evidence(explained: &Value) -> &Value {
    let links: Vec<&Value> = explained["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|link| link["kind"] == "Evidence")
        .collect();
    assert_eq!(links.len(), 1, "{explained:.2000}");
    assert_eq!(links[0]["id"], EVIDENCE);
    links[0]
}

/// The bytes an Evidence link answers: its `payload`, decoded.
fn bytes(link: &Value) -> Vec<u8> {
    base64::engine::general_purpose::STANDARD
        .decode(link["payload"].as_str().unwrap())
        .unwrap()
}

/// Acceptance 1 on one answer.
fn holds_the_default_bound(lane: &str, explained: &Value, record: &[u8]) {
    let link = evidence(explained);
    let answered = bytes(link);
    assert!(
        answered.len() <= DEFAULT_LIMIT,
        "{lane}: {} bytes answered",
        answered.len()
    );
    assert_eq!(link["truncated"], true, "{lane}");
    assert_eq!(link["record_length"], RECORD_BYTES, "{lane}");
    let offset = usize::try_from(link["offset"].as_u64().unwrap()).unwrap();
    assert_eq!(
        answered,
        record[offset..offset + answered.len()],
        "{lane}: the answer is the record's bytes from its offset"
    );
    let text = link["text"].as_str().unwrap();
    assert_eq!(text.as_bytes(), answered, "{lane}");
    assert!(
        text.contains(CITED),
        "{lane}: the cited text is in the answer"
    );
    let centre = offset + answered.len() / 2;
    let cited = CITED_AT + CITED.len() / 2;
    assert!(
        centre.abs_diff(cited) <= 4,
        "{lane}: the window is centred on the cited text: centre {centre}, cited {cited}"
    );
}

/// Acceptance 2 on one answer.
fn holds_the_first_thousand(lane: &str, explained: &Value, record: &[u8]) {
    let link = evidence(explained);
    assert_eq!(bytes(link), record[..1000], "{lane}");
    assert_eq!(link["offset"], 0, "{lane}");
    assert_eq!(link["truncated"], true, "{lane}");
    assert_eq!(link["record_length"], RECORD_BYTES, "{lane}");
}

#[test]
fn explain_bounds_each_evidence_record_and_reads_any_part_of_it_on_every_lane() {
    let record = record();
    for backend in ["file", "sqlite"] {
        let world = World::new(backend, &record);

        // 1. The default bound, centred on the cited text.
        let whole = world.ok(&["explain", ASSERTION, "--documents"]);
        holds_the_default_bound(&format!("{backend} one-shot"), &whole, &record);
        let mut server = Server::start(&world);
        let tool = server.explain(json!({"assertion": ASSERTION, "documents": true}));
        assert_eq!(tool, whole, "{backend}: MCP answers what the verb prints");

        // 2. The first 1,000 bytes.
        let first = world.ok(&[
            "explain",
            ASSERTION,
            "--documents",
            "--offset",
            "0",
            "--limit",
            "1000",
        ]);
        holds_the_first_thousand(&format!("{backend} one-shot"), &first, &record);
        let tool = server.explain(
            json!({"assertion": ASSERTION, "documents": true, "offset": 0, "limit": 1000}),
        );
        assert_eq!(tool, first, "{backend}: MCP answers what the verb prints");

        // 3. Steps of `limit` from offset 0 reassemble the record. The step does not divide the
        // record, and cuts its two-byte characters.
        let limit = 1_048_573;
        let mut reassembled = Vec::new();
        let mut offset = 0;
        let mut steps = 0;
        loop {
            let part = world.ok(&[
                "explain",
                ASSERTION,
                "--documents",
                "--offset",
                &offset.to_string(),
                "--limit",
                &limit.to_string(),
            ]);
            let link = evidence(&part);
            assert_eq!(link["offset"], offset, "{backend}");
            assert_eq!(link["record_length"], RECORD_BYTES, "{backend}");
            let answered = bytes(link);
            assert!(answered.len() <= limit, "{backend}");
            if let Some(text) = link["text"].as_str() {
                assert_eq!(text.as_bytes(), answered, "{backend}");
            }
            reassembled.extend_from_slice(&answered);
            offset += answered.len();
            steps += 1;
            if offset >= RECORD_BYTES {
                break;
            }
            assert_eq!(answered.len(), limit, "{backend}: a step short of the end");
        }
        assert_eq!(steps, RECORD_BYTES.div_ceil(limit), "{backend}");
        assert!(
            reassembled == record,
            "{backend}: the steps reassemble the record"
        );

        // The same steps through MCP, at the default step size.
        let mut reassembled = Vec::new();
        let mut offset = 0;
        while offset < RECORD_BYTES {
            let part = server.explain(json!({"assertion": ASSERTION, "documents": true,
                "offset": offset, "limit": DEFAULT_LIMIT}));
            let answered = bytes(evidence(&part));
            assert!(!answered.is_empty(), "{backend}: offset {offset}");
            reassembled.extend_from_slice(&answered);
            offset += answered.len();
        }
        assert!(
            reassembled == record,
            "{backend}: MCP steps reassemble the record"
        );
    }
}

/// A record within the bound is answered whole, as before: no `offset`, `record_length` or
/// `truncated`, the same document as without the new arguments.
#[test]
fn a_record_within_the_bound_is_answered_whole_and_unchanged() {
    let record = "Acme's legal name is ".to_owned() + CITED + ".";
    let world = World::new("sqlite", record.as_bytes());
    let whole = world.ok(&["explain", ASSERTION, "--documents"]);
    let link = evidence(&whole);
    assert_eq!(link["text"], record);
    for absent in ["offset", "record_length", "truncated"] {
        assert!(link.get(absent).is_none(), "{absent}: {link}");
    }
    let wide = world.ok(&["explain", ASSERTION, "--documents", "--limit", "1000000"]);
    assert_eq!(wide, whole);
}

/// `offset` and `limit` bound `--documents` and mean nothing without it; a limit of 0 reads
/// nothing and is refused. The verb refuses them as usage, MCP as invalid params.
#[test]
fn bounds_without_documents_or_with_a_zero_limit_are_refused() {
    let world = World::new("sqlite", b"Acme Widgets Holding");
    for verb in [
        vec!["explain", ASSERTION, "--offset", "0"],
        vec!["explain", ASSERTION, "--limit", "10"],
        vec!["explain", ASSERTION, "--documents", "--limit", "0"],
    ] {
        let output = world.run(&verb);
        assert_eq!(output.status.code(), Some(2), "{verb:?}");
        assert!(output.stdout.is_empty(), "{verb:?}");
    }
    let mut server = Server::start(&world);
    for arguments in [
        json!({"assertion": ASSERTION, "offset": 0}),
        json!({"assertion": ASSERTION, "documents": false, "limit": 10}),
        json!({"assertion": ASSERTION, "documents": true, "limit": 0}),
        json!({"assertion": ASSERTION, "documents": true, "offset": -1}),
    ] {
        let answer = server.call(arguments.clone());
        assert_eq!(answer["error"]["code"], -32602, "{arguments}: {answer}");
    }
}
