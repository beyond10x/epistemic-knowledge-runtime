//! `story:ekr-explain-evidence-fixes`: explanations and quality samples return whole, pinned
//! evidence (review of EKR 0.0.42, findings 20, 21 and 23), on both providers.
//!
//! The store is the example seed with one more assertion, Acme's `legal_name`, whose 48-byte value
//! is the cited text, citing one evidence record of 81,900 bytes that holds it once at byte
//! 40,800; revision 1 attaches the example seed's second evidence entry to its first assertion.
//!
//! * 20: `ekr.explanation/3`, the MCP default, halves a 64 KiB evidence window to fit its 48 KiB
//!   cap; the half it keeps holds the cited text, as the window it halves did.
//! * 21: `ekr explain --at N` and the MCP tool's `at` explain the assertion as revision N held it,
//!   and the answer's `at` names N.
//! * 23: `ekr sample` gives a judge the evidence attached to a fact after it was written, each such
//!   entry with `attached_at`, the revision that attached it.

use std::io::{BufRead, BufReader, Write as _};
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use ekr_core::ContentHash;
use serde_json::{json, Value};

const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
/// The example seed's first assertion, citing its first evidence entry.
const SEEDED: &str = "00000000-0000-4000-8000-000000000510";
const SEEDED_EVIDENCE: &str = "00000000-0000-4000-8000-000000000401";
/// The example seed's second evidence entry, which `SEEDED` does not cite; revision 1 attaches it.
const ATTACHED: &str = "00000000-0000-4000-8000-000000000402";
const ATTACHING: &str = "00000000-0000-4000-8000-000000000903";
/// Acme's `legal_name`, citing `LARGE`.
const LEGAL: &str = "00000000-0000-4000-8000-000000000513";
const LARGE: &str = "00000000-0000-4000-8000-000000000403";
const RECORD_BYTES: usize = 81_900;
const CITED_AT: usize = 40_800;
const CITED: &str = "Acme Widgets Holding was registered in 2019 (A7)";

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// ASCII filler with `CITED` once at `CITED_AT`.
fn record() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(RECORD_BYTES);
    let mut line = 0;
    while bytes.len() < RECORD_BYTES {
        bytes.extend_from_slice(format!("Ledger line {line:05}: nothing of note.\n").as_bytes());
        line += 1;
    }
    bytes.truncate(RECORD_BYTES);
    bytes[CITED_AT..CITED_AT + CITED.len()].copy_from_slice(CITED.as_bytes());
    assert_eq!(CITED.len(), 48);
    assert_eq!(
        bytes
            .windows(CITED.len())
            .filter(|window| *window == CITED.as_bytes())
            .count(),
        1
    );
    bytes
}

/// The example seed plus Acme's `legal_name` assertion citing `LARGE`, whose bytes the seed takes
/// from a file (`--evidence`).
fn seed(record: &[u8]) -> String {
    let example = text(&["example", "ekr-seed/2"]);
    let hash = ContentHash::of_bytes(record);
    let added = format!(
        "      {LEGAL}:
        id: {LEGAL}
        root_id: 00000000-0000-4000-8000-000000000002
        subject: !Node 00000000-0000-4000-8000-000000000303
        predicate: !Property 00000000-0000-4000-8000-000000000801
        object: !Value
          value_kind: String
          value: {CITED}
        evidence:
        - {LARGE}
        proposed_by: {OPERATOR}
        assessment: Proposed
        lifecycle: Active
        valid_time:
          from: 1577836800000
          to: null
        transaction_time:
          recorded_from: 0
          recorded_to: null
    evidence:
      {LARGE}:
        id: {LARGE}
        source: !HumanStatement
          identity: Runtime operator
        content_hash: {hash}
        extracted_by: {OPERATOR}
        observed_at: 1773273600000
        confidence: 10000
"
    );
    let marker = "\n    evidence:\n";
    assert_eq!(example.matches(marker).count(), 1, "{example}");
    example.replacen(marker, &format!("\n{added}"), 1)
}

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
    record: Vec<u8>,
}

impl World {
    /// Seeded (revision 0) and with `ATTACHED` attached to `SEEDED` (revision 1).
    fn new(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
            record: record(),
        };
        world.file("host.json", text(&["example", "ekr.cli-host/1"]).as_bytes());
        let record = world.file("record.txt", &world.record);
        let seed = world.file("seed.yaml", seed(&world.record).as_bytes());
        assert_eq!(
            world.ok(&["seed", &seed, "--evidence", &record])["result"]["revision"],
            0
        );
        let attach = world.file(
            "attach.yaml",
            format!(
                "format: ekr.transaction-document/2\ntransaction:\n  id: {ATTACHING}\n  \
                 proposer: {OPERATOR}\n  operations:\n  - !AttachEvidence\n    assertion: \
                 {SEEDED}\n    evidence: {ATTACHED}\n  evidence:\n  - {ATTACHED}\n"
            )
            .as_bytes(),
        );
        world.ok(&["propose", &attach]);
        assert_eq!(world.ok(&["validate", ATTACHING])["kind"], "Validated");
        assert_eq!(world.ok(&["commit", ATTACHING])["kind"], "Committed");
        world
    }

    fn file(&self, name: &str, contents: &[u8]) -> String {
        let path = self.directory.path().join(name);
        std::fs::write(&path, contents).unwrap();
        path.display().to_string()
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
            .arg(self.directory.path().join("host.json"))
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
            "capabilities": {}, "clientInfo": {"name": "explain-pinned", "version": "0"}}}));
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

    /// The JSON-RPC response to an `explain` call.
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

/// The links of `kind` an explanation holds.
fn links<'a>(explained: &'a Value, kind: &str) -> Vec<&'a Value> {
    explained["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|link| link["kind"] == kind)
        .collect()
}

/// The ids of an explanation's Evidence links, in order.
fn evidence_ids(explained: &Value) -> Vec<&str> {
    links(explained, "Evidence")
        .into_iter()
        .map(|link| link["id"].as_str().unwrap())
        .collect()
}

// ---- finding 20 ----------------------------------------------------------------------------------

fn halving_keeps_the_cited_text(backend: &'static str) {
    let world = World::new(backend);
    let v2 = world.ok(&["explain", LEGAL, "--documents"]);
    let window = links(&v2, "Evidence")[0];
    assert_eq!(window["offset"], 8_056, "{backend}: /2's window, as reviewed");
    assert!(window["text"].as_str().unwrap().contains(CITED));

    let v3 = world.ok(&[
        "explain",
        LEGAL,
        "--documents",
        "--format",
        "ekr.explanation/3",
    ]);
    let evidence = links(&v3, "Evidence");
    assert_eq!(evidence.len(), 1, "{backend}");
    let link = evidence[0];
    assert_eq!(link["id"], LARGE);
    assert_eq!(link["truncated"], true, "{backend}");
    assert_eq!(link["record_length"], RECORD_BYTES, "{backend}");
    let kept = link["text"].as_str().unwrap();
    assert!(
        kept.len() <= 32_768,
        "{backend}: halved, {} bytes",
        kept.len()
    );
    let offset = usize::try_from(link["offset"].as_u64().unwrap()).unwrap();
    assert_eq!(
        kept.as_bytes(),
        &world.record[offset..offset + kept.len()],
        "{backend}: the kept text is the record's bytes from its offset"
    );
    assert!(
        kept.contains(CITED),
        "{backend}: the half kept holds the cited text; offset {offset}, {} bytes",
        kept.len()
    );
    let printed = serde_json::to_string_pretty(&v3).unwrap().len() + 1;
    assert!(printed <= 48 * 1024, "{backend}: {printed} bytes");

    let mut server = Server::start(&world);
    let tool = server.explain(json!({"assertion": LEGAL, "documents": true}));
    assert_eq!(tool, v3, "{backend}: the MCP default answers what the verb prints");
}

#[test]
fn explanation_v3_halves_a_window_around_the_cited_text_file() {
    halving_keeps_the_cited_text("file");
}

#[test]
fn explanation_v3_halves_a_window_around_the_cited_text_sqlite() {
    halving_keeps_the_cited_text("sqlite");
}

// ---- finding 21 ----------------------------------------------------------------------------------

fn explain_at_reads_that_revision(backend: &'static str) {
    let world = World::new(backend);
    let head = world.ok(&["explain", SEEDED]);
    assert_eq!(head["at"], 1, "{backend}");
    assert_eq!(evidence_ids(&head), [SEEDED_EVIDENCE, ATTACHED], "{backend}");
    assert_eq!(links(&head, "Attachment").len(), 1, "{backend}");

    let pinned = world.ok(&["explain", SEEDED, "--at", "0"]);
    assert_eq!(pinned["at"], 0, "{backend}: the answer names the revision read");
    assert_eq!(evidence_ids(&pinned), [SEEDED_EVIDENCE], "{backend}");
    assert!(links(&pinned, "Attachment").is_empty(), "{backend}");
    assert_eq!(world.ok(&["explain", SEEDED, "--at", "1"]), head, "{backend}");

    let documents = world.ok(&["explain", SEEDED, "--at", "0", "--documents"]);
    assert_eq!(documents["at"], 0, "{backend}");
    assert_eq!(evidence_ids(&documents), [SEEDED_EVIDENCE], "{backend}");

    let missing = world.run(&["explain", SEEDED, "--at", "2"]);
    assert_eq!(missing.status.code(), Some(2), "{backend}");
    assert!(missing.stdout.is_empty(), "{backend}");
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("ekr.kernel.RevisionNotFound"),
        "{backend}: {}",
        String::from_utf8_lossy(&missing.stderr)
    );

    let v3 = world.ok(&[
        "explain",
        SEEDED,
        "--at",
        "0",
        "--format",
        "ekr.explanation/3",
    ]);
    assert_eq!(v3["at"], 0, "{backend}");
    let mut server = Server::start(&world);
    assert_eq!(
        server.explain(json!({"assertion": SEEDED, "at": 0})),
        v3,
        "{backend}: the MCP tool answers what the verb prints"
    );
    assert_eq!(
        server.explain(json!({"assertion": SEEDED, "at": 0, "format": "ekr.explanation/2"})),
        pinned,
        "{backend}"
    );
    let refused = server.call(json!({"assertion": SEEDED, "at": 2}));
    assert_eq!(refused["result"]["isError"], true, "{backend}: {refused}");
    assert!(
        refused.to_string().contains("ekr.kernel.RevisionNotFound"),
        "{backend}: {refused}"
    );
}

#[test]
fn explain_at_a_revision_reads_that_snapshot_file() {
    explain_at_reads_that_revision("file");
}

#[test]
fn explain_at_a_revision_reads_that_snapshot_sqlite() {
    explain_at_reads_that_revision("sqlite");
}

// ---- finding 23 ----------------------------------------------------------------------------------

/// The sampled item of `assertion` in an `ekr.fact-sample/1` document.
fn item<'a>(sample: &'a Value, assertion: &str) -> &'a Value {
    sample["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|item| item["assertion"]["id"] == assertion)
        .unwrap_or_else(|| panic!("{assertion} is not sampled: {sample:.2000}"))
}

fn sample_carries_attached_evidence(backend: &'static str) {
    let world = World::new(backend);
    let head = world.ok(&["sample", "--seed", "1", "--size", "1000"]);
    assert_eq!(head["meta"]["revision"], 1, "{backend}");
    let evidence = item(&head, SEEDED)["evidence"].as_array().unwrap();
    let ids: Vec<&str> = evidence
        .iter()
        .map(|entry| entry["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, [SEEDED_EVIDENCE, ATTACHED], "{backend}");
    assert!(evidence[0].get("attached_at").is_none(), "{backend}");
    assert_eq!(evidence[1]["attached_at"], 1, "{backend}: its provenance");
    assert!(evidence[1]["text"].as_str().is_some(), "{backend}");
    let explained = world.ok(&["explain", SEEDED]);
    assert_eq!(
        evidence_ids(&explained),
        ids,
        "{backend}: the judge reads what explain names"
    );
    for other in head["items"].as_array().unwrap() {
        if other["assertion"]["id"] != SEEDED {
            for entry in other["evidence"].as_array().unwrap() {
                assert!(entry.get("attached_at").is_none(), "{backend}: {entry}");
            }
        }
    }

    let before = world.ok(&[
        "sample",
        "--seed",
        "1",
        "--size",
        "1000",
        "--revision",
        "0",
    ]);
    let evidence = item(&before, SEEDED)["evidence"].as_array().unwrap();
    assert_eq!(evidence.len(), 1, "{backend}: nothing was attached at 0");
    assert_eq!(evidence[0]["id"], SEEDED_EVIDENCE, "{backend}");
}

#[test]
fn a_sample_carries_evidence_attached_after_the_fact_file() {
    sample_carries_attached_evidence("file");
}

#[test]
fn a_sample_carries_evidence_attached_after_the_fact_sqlite() {
    sample_carries_attached_evidence("sqlite");
}
