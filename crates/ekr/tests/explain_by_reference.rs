//! `story:explain-reads-an-index`, acceptance 1: at a consumer's 1× shape, `explain` of an
//! assertion answers in under 0.5 s and under 200 KB through the one-shot verb and the session,
//! and under 400 KB through MCP.
//!
//! The 1× shape is the one the performance audit of 2026-09-29 measured
//! (`story:reads-share-verified-state`): 4,127 nodes, about 14.4k edges and 67k assertions in 57
//! committed batches of 1,561 operations. What the audit did not record — how much evidence a
//! batch adds — is taken here as 61 `AddEvidence` of 600 bytes each, every assertion of a batch
//! citing one of them. The last batch also supersedes 20 assertions of batch 30, so that a chain
//! running through two batches is measured beside one running through one.
//!
//! Building the store takes minutes, so the case is ignored by the default run. Run it with the
//! release profile:
//!
//! ```text
//! cargo test --release --locked -p ekr --test explain_by_reference -- --ignored --nocapture
//! ```
//!
//! With `EKR_EXPLAIN_COST_DIR` set, the store is built in that directory and kept, and a later run
//! with the same directory measures it again without rebuilding it.
//!
//! Each lane is timed in CPU (user and system, from `/proc`, so Linux only) and wall clock: the
//! one-shot verb as its whole process (the open included), the session and MCP from the request
//! line written to the answer line read, on a process that has already answered one read. The
//! time bound is held on the CPU median of the runs, as the audit measured; every run is printed.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{BufRead, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use ekr::host::CliHostConfigurationV1;
use ekr_core::{
    AssertionId, ContentHash, EdgeId, EvidenceId, NodeId, PropertyId, Timestamp, TransactionId,
    TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Node, Object,
    Predicate, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    CommitCommandResult, EdgeDraft, EvidenceAddition, GraphOperation, GraphTransaction, NodeDraft,
    Runtime, SeedDocument, Supersession, ValidationCommandResult,
};
use ekr_ontology::{Cardinality, EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use serde_json::{json, Value as Json};

const BATCHES: u64 = 57;
const SEED_NODES: usize = 23;
const NODES_PER_BATCH: usize = 72;
const EDGES_PER_BATCH: usize = 253;
const ASSERTIONS_PER_BATCH: usize = 1_175;
const EVIDENCE_PER_BATCH: usize = 61;
const PAYLOAD_BYTES: usize = 600;
const SUPERSEDED: usize = 20;
/// The batch whose assertions the last batch supersedes.
const EARLIER: u64 = 30;
const RUNS: usize = 5;

const WITHIN: Duration = Duration::from_millis(500);
const ONE_SHOT_BYTES: usize = 200 * 1024;
const SESSION_BYTES: usize = 200 * 1024;
const MCP_BYTES: usize = 400 * 1024;

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

/// The ontology's ids, fixed so that the shape is the same on every run.
fn type_id() -> TypeId {
    "00000000-0000-4000-8000-00000000c001".parse().unwrap()
}
fn edge_type() -> TypeId {
    "00000000-0000-4000-8000-00000000c002".parse().unwrap()
}
fn label() -> PropertyId {
    "00000000-0000-4000-8000-00000000c003".parse().unwrap()
}

fn seed() -> SeedDocument {
    let mut seed = SeedDocument::from_yaml(
        &String::from_utf8(
            ekr()
                .args(["example", "ekr-seed/2"])
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap(),
    )
    .unwrap();
    seed.graph.nodes.clear();
    seed.graph.edges.clear();
    seed.graph.assertions.clear();
    seed.graph.evidence.clear();
    seed.evidence_payloads.clear();
    let mut subject = NodeType::new(type_id(), "Subject");
    let mut definition = PropertyDefinition::new(label(), "label", ValueType::String);
    definition.cardinality = Cardinality::Many;
    subject.properties.insert(label(), definition);
    seed.ontology.node_types = vec![subject];
    let mut relates = EdgeType::new(edge_type(), "relates");
    relates.source_types = BTreeSet::from([type_id()]);
    relates.target_types = BTreeSet::from([type_id()]);
    relates.cardinality = Cardinality::Many;
    seed.ontology.edge_types = vec![relates];
    for n in 0..SEED_NODES {
        let node = Node::<Value>::new(
            NodeId::mint(),
            seed.graph.root.id,
            type_id(),
            format!("seeded subject {n}"),
        );
        seed.graph.nodes.insert(node.id, node);
    }
    seed
}

/// What the shape keeps between batches.
struct Shape {
    root: ekr_core::GraphRootId,
    nodes: Vec<NodeId>,
    /// Batch 30's assertions, which the last batch supersedes.
    earlier: Vec<AssertionId>,
}

fn payload(batch: u64, item: usize) -> Vec<u8> {
    let mut text = format!("Source chunk {batch}-{item}: ");
    while text.len() < PAYLOAD_BYTES {
        text.push_str("the subject relates to another subject as the record states. ");
    }
    text.truncate(PAYLOAD_BYTES);
    text.into_bytes()
}

fn batch(host: &CliHostConfigurationV1, shape: &mut Shape, number: u64) -> GraphTransaction {
    let operator = host.context.operator;
    let mut operations = Vec::new();
    let mut evidence = Vec::new();
    for item in 0..EVIDENCE_PER_BATCH {
        let bytes = payload(number, item);
        let entry = Evidence {
            id: EvidenceId::mint(),
            source: EvidenceSource::HumanStatement {
                identity: Some("synthetic source".into()),
            },
            content_hash: ContentHash::of_bytes(&bytes),
            extracted_by: operator,
            observed_at: Timestamp::EPOCH,
            confidence: Confidence::from_basis_points(9000).unwrap(),
        };
        evidence.push(entry.id);
        operations.push(GraphOperation::AddEvidence(Box::new(EvidenceAddition {
            evidence: entry,
            payload: bytes,
        })));
    }
    let first = shape.nodes.len();
    for n in 0..NODES_PER_BATCH {
        let id = NodeId::mint();
        operations.push(GraphOperation::CreateNode(NodeDraft {
            id,
            root_id: shape.root,
            type_id: type_id(),
            canonical_name: format!("subject {number}-{n}"),
            properties: BTreeMap::new(),
            aliases: Vec::new(),
        }));
        shape.nodes.push(id);
    }
    for n in 0..EDGES_PER_BATCH {
        let source = shape.nodes[first + n % NODES_PER_BATCH];
        let target = shape.nodes[(n * 7919 + usize::try_from(number).unwrap()) % first.max(1)];
        operations.push(GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id: shape.root,
            type_id: edge_type(),
            source,
            target,
            properties: BTreeMap::new(),
        }));
    }
    let mut added = Vec::new();
    for n in 0..ASSERTIONS_PER_BATCH {
        let assertion = Assertion {
            id: AssertionId::mint(),
            root_id: shape.root,
            subject: Subject::Node(shape.nodes[first + n % NODES_PER_BATCH]),
            predicate: Predicate::Property(label()),
            object: Object::Value(Value::String(format!("label {number}-{n}"))),
            evidence: BTreeSet::from([evidence[n % EVIDENCE_PER_BATCH]]),
            proposed_by: operator,
            assessment: Assessment::Proposed,
            lifecycle: AssertionLifecycle::Active,
            valid_time: TemporalRange::since(Timestamp::from_millis(0)),
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        };
        added.push(assertion.id);
        operations.push(GraphOperation::AddAssertion(Box::new(assertion)));
    }
    if number == EARLIER {
        shape.earlier = added.iter().copied().take(SUPERSEDED).collect();
    }
    if number == BATCHES {
        let from = Timestamp::from_millis(1_000);
        for (n, old) in shape.earlier.iter().enumerate() {
            let replacement = Assertion {
                id: AssertionId::mint(),
                root_id: shape.root,
                subject: Subject::Node(shape.nodes[first + n % NODES_PER_BATCH]),
                predicate: Predicate::Property(label()),
                object: Object::Value(Value::String(format!("replacement {n}"))),
                evidence: BTreeSet::from([evidence[n % EVIDENCE_PER_BATCH]]),
                proposed_by: operator,
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time: TemporalRange::since(from),
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            };
            operations.push(GraphOperation::SupersedeAssertion(Supersession {
                assertion: *old,
                by: replacement.id,
                effective_from: from,
            }));
            operations.push(GraphOperation::AddAssertion(Box::new(replacement)));
        }
    }
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: operator,
        operations,
        evidence: evidence.into_iter().collect(),
        schema_version: None,
    }
}

fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}

/// Seeds `store` and commits the 57 batches through the kernel handlers; the two assertions to
/// explain: one of batch 30 that stays active, and one of batch 30 that batch 57 supersedes.
fn build(store: &Path, host: &CliHostConfigurationV1) -> (AssertionId, AssertionId) {
    let runtime =
        Runtime::sqlite(store, &host.tenant, host.context, host.authority.clone()).unwrap();
    let seed = seed();
    let mut shape = Shape {
        root: seed.graph.root.id,
        nodes: seed.graph.nodes.keys().copied().collect(),
        earlier: Vec::new(),
    };
    runtime.seed(seed, || Timestamp::from_millis(10)).unwrap();
    let operator = host.context.operator;
    let mut active = None;
    let started = Instant::now();
    for number in 1..=BATCHES {
        let tx = batch(host, &mut shape, number);
        if number == EARLIER {
            active = tx.operations.iter().rev().find_map(|op| match op {
                GraphOperation::AddAssertion(a) => Some(a.id),
                _ => None,
            });
        }
        let at = i64::try_from(number * 100).unwrap();
        runtime
            .propose(&encode(&tx), operator, || Timestamp::from_millis(at))
            .unwrap();
        let head = runtime.head().unwrap().unwrap().revision;
        let validated = runtime
            .validate(tx.id, head, || Timestamp::from_millis(at + 1))
            .unwrap();
        assert!(
            matches!(validated, ValidationCommandResult::Validated(_)),
            "batch {number}: {validated:?}"
        );
        let committed = runtime
            .commit(tx.id, operator, || Timestamp::from_millis(at + 2))
            .unwrap();
        assert!(
            matches!(committed, CommitCommandResult::Committed(_)),
            "batch {number}: {committed:?}"
        );
        if number % 10 == 0 {
            eprintln!("batch {number} committed at {:?}", started.elapsed());
        }
    }
    let read = runtime.read(None).unwrap();
    assert_eq!(read.graph.nodes.len(), SEED_NODES + 57 * NODES_PER_BATCH);
    eprintln!(
        "built in {:?}: {} nodes, {} edges, {} assertions, {} evidence",
        started.elapsed(),
        read.graph.nodes.len(),
        read.graph.edges.len(),
        read.graph.assertions.len(),
        read.graph.evidence.len()
    );
    (active.unwrap(), shape.earlier[0])
}

struct World {
    directory: PathBuf,
    /// The temporary directory `directory` is, removed with the world; `None` for a kept one.
    _held: Option<tempfile::TempDir>,
    backend: &'static str,
}

impl World {
    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.join("store"),
            _ => self.directory.join("state.db"),
        }
    }
    fn command(&self, verb: &[&str]) -> Command {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(self.directory.join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb);
        command
    }
}

/// CPU time, user and system, read from `/proc/<pid>/stat` fields `first` and `first + 1`
/// (1-based, as `proc(5)` numbers them) in clock ticks of 10 ms, the `USER_HZ` of 100 that
/// `/proc` reports in on Linux.
fn ticks(pid: &str, first: usize) -> Duration {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).unwrap();
    // Fields 1 and 2 are the pid and `(comm)`, which may hold spaces; count from after it.
    let rest: Vec<&str> = stat[stat.rfind(')').unwrap() + 2..]
        .split_whitespace()
        .collect();
    let field = |n: usize| rest[n - 3].parse::<u64>().unwrap();
    Duration::from_millis((field(first) + field(first + 1)) * 10)
}

/// One line-oriented child: a request written, its one answer line read.
struct Child {
    child: std::process::Child,
    input: std::process::ChildStdin,
    output: BufReader<std::process::ChildStdout>,
}

impl Child {
    fn start(mut command: Command) -> Self {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        Self {
            child,
            input,
            output,
        }
    }
    /// Writes `line` and reads the answer: the wall clock between the two, and the CPU time the
    /// child spent in it (`utime` and `stime`).
    fn timed(&mut self, line: &str) -> (Duration, Duration, String) {
        let pid = self.child.id().to_string();
        let cpu = ticks(&pid, 14);
        let started = Instant::now();
        self.input.write_all(line.as_bytes()).unwrap();
        self.input.write_all(b"\n").unwrap();
        self.input.flush().unwrap();
        let mut answer = String::new();
        self.output.read_line(&mut answer).unwrap();
        let wall = started.elapsed();
        (wall, ticks(&pid, 14).saturating_sub(cpu), answer)
    }
    fn exchange(&mut self, line: &str) -> (Duration, String) {
        let (wall, _, answer) = self.timed(line);
        (wall, answer)
    }
}

impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// One lane's runs: every wall-clock and CPU time, and the size of the answer.
struct Lane {
    name: String,
    wall: Vec<Duration>,
    cpu: Vec<Duration>,
    bytes: usize,
    bound: usize,
}

fn median(times: &[Duration]) -> Duration {
    let mut sorted = times.to_vec();
    sorted.sort();
    sorted[sorted.len() / 2]
}

fn one_shot(world: &World, assertion: &str) -> Lane {
    let (mut wall, mut cpu) = (Vec::new(), Vec::new());
    let mut bytes = 0;
    for _ in 0..RUNS {
        // The waited children's CPU time, `cutime` and `cstime`: the one-shot process's own.
        let before = ticks("self", 16);
        let started = Instant::now();
        let output = world
            .command(&["explain", assertion])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        wall.push(started.elapsed());
        cpu.push(ticks("self", 16).saturating_sub(before));
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        bytes = output.stdout.len();
    }
    Lane {
        name: format!("one-shot {assertion}"),
        wall,
        cpu,
        bytes,
        bound: ONE_SHOT_BYTES,
    }
}

fn session(world: &World, assertion: &str) -> Lane {
    let mut child = Child::start(world.command(&["session"]));
    let (_, warm) = child.exchange(&json!({"argv": ["head"]}).to_string());
    assert!(warm.contains("\"exit\":0"), "{warm}");
    let (mut wall, mut cpu) = (Vec::new(), Vec::new());
    let mut bytes = 0;
    for _ in 0..RUNS {
        let (took, spent, answer) =
            child.timed(&json!({"argv": ["explain", assertion]}).to_string());
        let parsed: Json = serde_json::from_str(&answer).unwrap();
        assert_eq!(parsed["exit"], 0, "{}", parsed["stderr"]);
        wall.push(took);
        cpu.push(spent);
        bytes = answer.len();
    }
    Lane {
        name: format!("session {assertion}"),
        wall,
        cpu,
        bytes,
        bound: SESSION_BYTES,
    }
}

fn mcp(world: &World, assertion: &str) -> Lane {
    let mut child = Child::start(world.command(&["mcp"]));
    let (_, initialized) = child.exchange(
        &json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
            "protocolVersion": "2025-11-25", "capabilities": {},
            "clientInfo": {"name": "explain-cost", "version": "0"}}})
        .to_string(),
    );
    assert!(initialized.contains("\"result\""), "{initialized}");
    child
        .input
        .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
        .unwrap();
    let (_, warm) = child.exchange(
        &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "head", "arguments": {}}})
        .to_string(),
    );
    assert!(warm.contains("\"isError\":false"), "{warm}");
    let (mut wall, mut cpu) = (Vec::new(), Vec::new());
    let mut bytes = 0;
    for run in 0..RUNS {
        let (took, spent, answer) = child.timed(
            &json!({"jsonrpc": "2.0", "id": 2 + run, "method": "tools/call",
                "params": {"name": "explain", "arguments": {"assertion": assertion}}})
            .to_string(),
        );
        let parsed: Json = serde_json::from_str(&answer).unwrap();
        assert_eq!(parsed["result"]["isError"], false, "{answer:.2000}");
        wall.push(took);
        cpu.push(spent);
        bytes = answer.len();
    }
    Lane {
        name: format!("mcp {assertion}"),
        wall,
        cpu,
        bytes,
        bound: MCP_BYTES,
    }
}

#[test]
#[ignore = "builds a 1x store for minutes; run with --release -- --ignored"]
fn explain_at_the_one_x_shape_answers_within_its_time_and_size_bounds() {
    // `EKR_EXPLAIN_COST_DIR` keeps the store in that directory and reuses it on the next run.
    let (directory, held) = match std::env::var_os("EKR_EXPLAIN_COST_DIR") {
        Some(kept) => (PathBuf::from(kept), None),
        None => {
            let held = tempfile::tempdir().unwrap();
            (held.path().to_path_buf(), Some(held))
        }
    };
    std::fs::create_dir_all(&directory).unwrap();
    let world = World {
        directory,
        _held: held,
        backend: "sqlite",
    };
    let host = ekr().args(["example", "ekr.cli-host/1"]).output().unwrap();
    std::fs::write(world.directory.join("host.json"), &host.stdout).unwrap();
    let host = CliHostConfigurationV1::from_json(&host.stdout).unwrap();
    let built = world.directory.join("explained.json");
    let (active, superseded) = if world.store().exists() && built.exists() {
        serde_json::from_slice(&std::fs::read(&built).unwrap()).unwrap()
    } else {
        let explained = build(&world.store(), &host);
        std::fs::write(&built, serde_json::to_vec(&explained).unwrap()).unwrap();
        explained
    };
    eprintln!(
        "store: {} bytes",
        std::fs::metadata(world.store()).unwrap().len()
    );

    let mut lanes = Vec::new();
    for assertion in [active, superseded] {
        let assertion = assertion.to_string();
        lanes.push(one_shot(&world, &assertion));
        lanes.push(session(&world, &assertion));
        lanes.push(mcp(&world, &assertion));
    }
    let mut over = Vec::new();
    let ms = |times: &[Duration]| times.iter().map(Duration::as_millis).collect::<Vec<_>>();
    for lane in &lanes {
        println!(
            "{}: CPU median {} ms {:?}, wall median {} ms {:?}, answer {} bytes (bound {} bytes)",
            lane.name,
            median(&lane.cpu).as_millis(),
            ms(&lane.cpu),
            median(&lane.wall).as_millis(),
            ms(&lane.wall),
            lane.bytes,
            lane.bound
        );
        // The time bound is held on CPU, as the audit measured it: on a shared machine the wall
        // clock also counts the time other processes held the cores.
        if median(&lane.cpu) >= WITHIN {
            over.push(format!("{}: CPU median {:?}", lane.name, median(&lane.cpu)));
        }
        if lane.bytes >= lane.bound {
            over.push(format!("{}: {} bytes", lane.name, lane.bytes));
        }
    }
    assert!(over.is_empty(), "over the bounds:\n  {}", over.join("\n  "));
}

/// Every place a byte string could still sit in a document: a `document_bytes` key.
fn carries_document_bytes(value: &Json) -> bool {
    match value {
        Json::Object(map) => map
            .iter()
            .any(|(key, item)| key == "document_bytes" || carries_document_bytes(item)),
        Json::Array(items) => items.iter().any(carries_document_bytes),
        _ => false,
    }
}

fn links_of<'a>(explained: &'a Json, kind: &str) -> Vec<&'a Json> {
    explained["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|link| link["kind"] == kind)
        .collect()
}

/// The one-shot verb, the session and MCP answer `ekr.explanation/2` on both providers: the
/// example transaction's assertion is explained through its proposal, validation, commit and
/// evidence by reference — no retained document, receipt or payload in the answer — and with
/// `--documents` (MCP `documents: true`) each link also carries the whole record it names.
#[test]
fn every_lane_explains_by_reference_and_returns_whole_records_when_asked() {
    const ASSERTION: &str = "00000000-0000-4000-8000-000000000501";
    const TRANSACTION: &str = "00000000-0000-4000-8000-000000000601";
    for backend in ["file", "sqlite"] {
        let held = tempfile::tempdir().unwrap();
        let world = World {
            directory: held.path().to_path_buf(),
            _held: Some(held),
            backend,
        };
        let example = |format: &str, name: &str| {
            let output = ekr().args(["example", format]).output().unwrap();
            std::fs::write(world.directory.join(name), output.stdout).unwrap();
            world.directory.join(name)
        };
        example("ekr.cli-host/1", "host.json");
        let seed = example("ekr-seed/2", "seed.yaml");
        let change = example("ekr.transaction-document/2", "change.yaml");
        let ok = |verb: &[&str]| -> Json {
            let output = world.command(verb).stdin(Stdio::null()).output().unwrap();
            assert_eq!(
                output.status.code(),
                Some(0),
                "{backend} {verb:?}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            serde_json::from_slice(&output.stdout).unwrap()
        };
        ok(&["seed", seed.to_str().unwrap()]);
        ok(&["propose", change.to_str().unwrap()]);
        ok(&["validate", TRANSACTION]);
        ok(&["commit", TRANSACTION]);

        let explained = ok(&["explain", ASSERTION]);
        assert_eq!(explained["format"], "ekr.explanation/2", "{backend}");
        let kinds: Vec<&str> = explained["links"]
            .as_array()
            .unwrap()
            .iter()
            .map(|link| link["kind"].as_str().unwrap())
            .collect();
        assert_eq!(
            kinds,
            ["Assertion", "Proposal", "Validation", "Commit", "Evidence"],
            "{backend}"
        );
        assert!(
            !carries_document_bytes(&explained),
            "{backend}: {explained}"
        );
        let proposal = links_of(&explained, "Proposal")[0];
        assert_eq!(proposal["transaction_id"], TRANSACTION, "{backend}");
        let operations = proposal["operations"].as_array().unwrap();
        assert_eq!(operations.len(), 1, "{backend}: {proposal}");
        assert_eq!(operations[0]["AddAssertion"]["id"], ASSERTION, "{backend}");
        let commit = links_of(&explained, "Commit")[0];
        assert_eq!(commit["transaction_id"], TRANSACTION, "{backend}");
        assert_eq!(commit["result"]["revision"], 1, "{backend}");
        assert!(commit.get("receipt").is_none(), "{backend}: {commit}");
        let evidence = links_of(&explained, "Evidence")[0];
        assert!(evidence["content_hash"].is_string(), "{backend}");
        assert!(evidence.get("payload").is_none(), "{backend}: {evidence}");
        assert!(evidence.get("text").is_none(), "{backend}: {evidence}");

        let whole = ok(&["explain", ASSERTION, "--documents"]);
        assert_eq!(whole["format"], "ekr.explanation/2", "{backend}");
        let record = &links_of(&whole, "Proposal")[0]["record"];
        assert_eq!(record["transaction_id"], TRANSACTION, "{backend}: {record}");
        assert!(record["document_bytes"].is_string(), "{backend}: {record}");
        let receipt = &links_of(&whole, "Commit")[0]["receipt"];
        assert_eq!(receipt["result"]["revision"], 1, "{backend}: {receipt}");
        let evidence = links_of(&whole, "Evidence")[0];
        assert_eq!(evidence["text"], "Alice is CEO of Acme.", "{backend}");
        assert!(evidence["payload"].is_string(), "{backend}");
        // Without the flag the same links, minus what the flag adds.
        let mut stripped = whole.clone();
        for link in stripped["links"].as_array_mut().unwrap() {
            let added: &[&str] = match link["kind"].as_str().unwrap() {
                "Proposal" => &["record"],
                "Commit" => &["receipt"],
                "Evidence" => &["payload", "text"],
                _ => &[],
            };
            let fields = link.as_object_mut().unwrap();
            for name in added {
                assert!(fields.remove(*name).is_some(), "{backend}: {name}");
            }
        }
        assert_eq!(stripped, explained, "{backend}");

        let mut session = Child::start(world.command(&["session"]));
        for (argv, expected) in [
            (vec!["explain", ASSERTION], &explained),
            (vec!["explain", ASSERTION, "--documents"], &whole),
        ] {
            let (_, answer) = session.exchange(&json!({ "argv": argv }).to_string());
            let answer: Json = serde_json::from_str(&answer).unwrap();
            assert_eq!(answer["exit"], 0, "{backend} session {argv:?}: {answer}");
            assert_eq!(&answer["stdout"], expected, "{backend} session {argv:?}");
        }

        let mut server = Child::start(world.command(&["mcp"]));
        let (_, initialized) = server.exchange(
            &json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
                "protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": {"name": "explain-lanes", "version": "0"}}})
            .to_string(),
        );
        assert!(initialized.contains("\"result\""), "{initialized}");
        server
            .input
            .write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n")
            .unwrap();
        for (id, arguments, expected) in [
            (1, json!({"assertion": ASSERTION}), &explained),
            (
                2,
                json!({"assertion": ASSERTION, "documents": true}),
                &whole,
            ),
            (
                3,
                json!({"assertion": ASSERTION, "documents": false}),
                &explained,
            ),
        ] {
            let (_, answer) = server.exchange(
                &json!({"jsonrpc": "2.0", "id": id, "method": "tools/call",
                    "params": {"name": "explain", "arguments": arguments}})
                .to_string(),
            );
            let answer: Json = serde_json::from_str(&answer).unwrap();
            assert_eq!(answer["result"]["isError"], false, "{backend}: {answer}");
            assert_eq!(
                &answer["result"]["structuredContent"], expected,
                "{backend} mcp {arguments}"
            );
        }
    }
}
