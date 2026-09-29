//! Replay checkpoints and compact records, design §§ 96 and 99, on both providers.
//!
//! The seed and every `REPLAY_CHECKPOINT_COMMITS`-th commit leave a checkpoint of the head they
//! published, and so does a commit that brings the operations committed since the retained one
//! to `REPLAY_CHECKPOINT_OPERATIONS`, or one made by a handle that restored none or whose last
//! checkpoint write the store did not keep; every other
//! commit appends only a pointer, and a verb that moves no head appends nothing. After any number
//! of commits a fresh open continues from the checkpoint and answers exactly as a full replay from
//! the seed does; a checkpoint that does not verify in any field it carries is ignored, and a
//! commit made after one replays in full; a validation against an earlier revision, whose graph a
//! checkpoint does not hold, replays in full; and every record and preparation a command retains
//! spells its byte strings as base64 and holds each staged object once.
//!
//! The forgery cases run under validation profile v1 and again under v3, whose checkpoint also
//! carries every node and edge identity the lineage held (`task:deleted-edge-id-is-reusable`).
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{Cardinality, EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use ekr_store::RevisionLog;
use serde::Serialize;
use std::borrow::Borrow;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TENANT: &str = "checkpoint";
/// The seed's edge type, from `Subject` to `Subject`.
const RELATES: &str = "00000000-0000-4000-8000-000000000008";

thread_local! {
    /// Whether the stores this test thread opens run validation profile v3 rather than v1. Each
    /// test runs on its own thread, so a case that sets it affects only itself.
    static KEEPS_IDENTITIES: Cell<bool> = const { Cell::new(false) };
}

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}
fn anchor() -> AuthorityStateV1 {
    let c = context();
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [(c.operator, "operator"), (c.validator, "validator")]
            .into_iter()
            .map(|(id, name)| {
                (
                    id,
                    Agent {
                        id,
                        name: name.into(),
                        capabilities: BTreeSet::new(),
                    },
                )
            })
            .collect(),
        validation_profile: if KEEPS_IDENTITIES.get() {
            ValidationProfileV1::identity_keeping(c.validator)
        } else {
            ValidationProfileV1::deterministic(c.validator)
        },
    }
}
fn open(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), anchor())
    }
    .unwrap()
}
fn open_in_full(path: &Path, file: bool) -> Runtime {
    let mut runtime = open(path, file);
    runtime.set_full_replay(true);
    runtime
}
fn seed() -> SeedDocument {
    let mut seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let type_id = "00000000-0000-4000-8000-000000000005".parse().unwrap();
    let label = "00000000-0000-4000-8000-000000000006".parse().unwrap();
    let mut declared = NodeType::new(type_id, "Subject");
    declared.properties.insert(
        label,
        PropertyDefinition::new(label, "label", ValueType::String),
    );
    seed.ontology.node_types.push(declared);
    let mut relates = EdgeType::new(RELATES.parse().unwrap(), "relates");
    relates.source_types = [type_id].into_iter().collect();
    relates.target_types = [type_id].into_iter().collect();
    relates.cardinality = Cardinality::Many;
    seed.ontology.edge_types.push(relates);
    let bytes = b"synthetic human evidence".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let evidence = Evidence {
        id: "00000000-0000-4000-8000-000000000007".parse().unwrap(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::from_basis_points(10000).unwrap(),
    };
    seed.graph.evidence.insert(evidence.id, evidence);
    seed.evidence_payloads.insert(hash, bytes.into());
    seed
}
/// `nodes` new nodes filed under `root`, each with one evidenced assertion.
fn document_under(
    seed: &SeedDocument,
    root: GraphRootId,
    n: u64,
    nodes: u64,
) -> (TransactionId, Vec<u8>) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let ty = &seed.ontology.node_types[0];
    let label = *ty.properties.keys().next().unwrap();
    let evidence = *seed.graph.evidence.keys().next().unwrap();
    let mut operations = Vec::new();
    for k in 0..nodes {
        let id = NodeId::mint();
        operations.push(GraphOperation::CreateNode(NodeDraft {
            id,
            root_id: root,
            type_id: ty.id,
            canonical_name: format!("subject {n}.{k}"),
            properties: BTreeMap::new(),
            aliases: Vec::new(),
        }));
        operations.push(GraphOperation::AddAssertion(Box::new(Assertion {
            id: AssertionId::mint(),
            root_id: root,
            subject: Subject::Node(id),
            predicate: Predicate::Property(label),
            object: Object::Value(Value::String(format!("label {n}.{k}"))),
            evidence: BTreeSet::from([evidence]),
            proposed_by: context().operator,
            assessment: Assessment::Proposed,
            lifecycle: AssertionLifecycle::Active,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        })));
    }
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: BTreeSet::from([evidence]),
        schema_version: None,
    };
    // A `/1` document holds at most 256 operations; a larger one is written in `/2`.
    let format = if tx.operations.len() > 256 {
        "ekr.transaction-document/2"
    } else {
        "ekr.transaction-document/1"
    };
    let bytes = serde_yaml_ng::to_string(&Wire {
        format,
        transaction: &tx,
    })
    .unwrap()
    .into_bytes();
    (tx.id, bytes)
}
/// Seeds and commits `commits` transactions, each command through a fresh runtime as one `ekr`
/// invocation does. Returns the seed.
fn build(path: &Path, file: bool, commits: u64) -> SeedDocument {
    let seed = seed();
    open(path, file)
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    for revision in 1..=commits {
        commit(path, file, &seed, revision, revision - 1);
    }
    seed
}
fn commit(
    path: &Path,
    file: bool,
    seed: &SeedDocument,
    n: u64,
    against: u64,
) -> CommitCommandResult {
    commit_under(path, file, seed, seed.graph.root.id, n, against)
}
/// [`commit`] of a document filed under `root`.
fn commit_under(
    path: &Path,
    file: bool,
    seed: &SeedDocument,
    root: GraphRootId,
    n: u64,
    against: u64,
) -> CommitCommandResult {
    commit_by(|| open(path, file), seed, root, n, against, 3)
}
/// Proposes, validates against `against` and commits a document of `nodes` new nodes filed under
/// `root`, each command through the runtime `runtime` returns.
fn commit_by<R: Borrow<Runtime>>(
    runtime: impl Fn() -> R,
    seed: &SeedDocument,
    root: GraphRootId,
    n: u64,
    against: u64,
    nodes: u64,
) -> CommitCommandResult {
    let at = i64::try_from(n * 100).unwrap();
    let (tx, bytes) = document_under(seed, root, n, nodes);
    runtime()
        .borrow()
        .propose(&bytes, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    let verdict = runtime()
        .borrow()
        .validate(tx, RevisionNumber::new(against), || {
            Timestamp::from_millis(at + 1)
        })
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    runtime()
        .borrow()
        .commit(tx, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap()
}
/// Commits revisions `from + 1` onwards, each against the one before, up to the first revision
/// at or after `from + 1` that is a multiple of [`REPLAY_CHECKPOINT_COMMITS`]: the commit that
/// writes the next checkpoint when the retained one is the seed's or one such commit's. Returns
/// the head revision.
fn commit_to_checkpoint(path: &Path, file: bool, seed: &SeedDocument, from: u64) -> u64 {
    let to = (from + 1).div_ceil(ekr_kernel::REPLAY_CHECKPOINT_COMMITS)
        * ekr_kernel::REPLAY_CHECKPOINT_COMMITS;
    for revision in from + 1..=to {
        commit(path, file, seed, revision, revision - 1);
    }
    to
}
/// Commits revision `n`, a transaction of exactly `operations`, validated against `n - 1`.
fn commit_operations(path: &Path, file: bool, n: u64, operations: Vec<GraphOperation>) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: BTreeSet::new(),
        schema_version: None,
    };
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: &tx,
    })
    .unwrap()
    .into_bytes();
    let at = i64::try_from(n * 100).unwrap();
    open(path, file)
        .propose(&bytes, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    let verdict = open(path, file)
        .validate(tx.id, RevisionNumber::new(n - 1), || {
            Timestamp::from_millis(at + 1)
        })
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let result = open(path, file)
        .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap();
    assert!(
        matches!(result, CommitCommandResult::Committed(_)),
        "{result:?}"
    );
}
/// [`build`] of two revisions, then revision 3 creating an edge between two nodes of revision 1
/// and revision 4 deleting it again. Returns the seed and the deleted edge's id.
fn build_with_a_deleted_edge(path: &Path, file: bool) -> (SeedDocument, EdgeId) {
    let seed = build(path, file, 2);
    let nodes: Vec<NodeId> = open(path, file)
        .replay(RevisionNumber::new(1))
        .unwrap()
        .nodes
        .into_keys()
        .collect();
    let deleted = EdgeId::mint();
    commit_operations(
        path,
        file,
        3,
        vec![GraphOperation::CreateEdge(EdgeDraft {
            id: deleted,
            root_id: seed.graph.root.id,
            type_id: RELATES.parse().unwrap(),
            source: nodes[0],
            target: nodes[1],
            properties: BTreeMap::new(),
        })],
    );
    commit_operations(path, file, 4, vec![GraphOperation::DeleteEdge(deleted)]);
    (seed, deleted)
}
type Answers = (
    Option<Root>,
    CanonicalGraph,
    BTreeMap<TransactionId, TransactionRecord>,
);
fn answers(runtime: &Runtime) -> Answers {
    (
        runtime.head().unwrap(),
        runtime.snapshot().unwrap(),
        runtime.transactions().unwrap(),
    )
}
/// What a verified read of the head answers for each of the seed's evidence payloads.
fn seed_evidence(runtime: &Runtime, seed: &SeedDocument) -> Vec<Option<Vec<u8>>> {
    let read = runtime.read(None).unwrap();
    seed.evidence_payloads
        .keys()
        .map(|hash| read.content(hash).map(<[u8]>::to_vec))
        .collect()
}
/// Every `ekr.store.CheckpointWritten` pointer the log holds.
fn pointers(runtime: &Runtime) -> Vec<serde_json::Value> {
    runtime
        .published_events()
        .unwrap()
        .into_iter()
        .filter(|event| event.name == "ekr.store.CheckpointWritten")
        .map(|event| {
            assert_eq!(event.stream_type, "ekr.checkpoint");
            event.data
        })
        .collect()
}
fn revision_occurrences(runtime: &Runtime) -> u64 {
    runtime
        .published_events()
        .unwrap()
        .iter()
        .filter(|event| event.stream_type == "ekr.revision")
        .count() as u64
}
/// The bytes of the checkpoint the newest pointer names, read from the provider itself.
fn checkpoint_bytes(path: &Path, file: bool, pointer: &serde_json::Value) -> Vec<u8> {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let digest = format!(
        "ekr.private.checkpoint.{}",
        pointer["checkpoint_hash"].as_str().unwrap()
    );
    let tenant = eventlog_core::TenantId::new(TENANT).unwrap();
    executor
        .block_on(async {
            use eventlog_core::EventStore;
            if file {
                let provider = eventlog_file::FileEventStore::open(path).await.unwrap();
                provider.get_blob(&tenant, &digest).await.unwrap()
            } else {
                let provider = eventlog_sqlite::SqliteEventStore::open(
                    &path.join("state.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap();
                provider.get_blob(&tenant, &digest).await.unwrap()
            }
        })
        .expect("the newest checkpoint is retained")
}
/// Retains `bytes` as the newest checkpoint through the store's own writer, as a store handle
/// with no kernel authority: the store neither checks nor could check what a checkpoint says.
fn install(path: &Path, file: bool, covered: u64, binding: ContentHash, bytes: &[u8]) {
    if file {
        ekr_store::FileStore::file_existing(path, TENANT, None)
            .unwrap()
            .write_checkpoint(covered, binding, Some(bytes))
            .unwrap();
    } else {
        ekr_store::SqliteStore::sqlite_existing(&path.join("state.db"), TENANT, None)
            .unwrap()
            .write_checkpoint(covered, binding, Some(bytes))
            .unwrap();
    }
}

/// The position, among `written` pointers, of each that names a checkpoint blob the pointer
/// before it does not: the pointers that wrote a checkpoint.
fn checkpoints_written(written: &[serde_json::Value]) -> Vec<u64> {
    (0..written.len())
        .filter(|&i| i == 0 || written[i]["checkpoint_hash"] != written[i - 1]["checkpoint_hash"])
        .map(|i| i as u64)
        .collect()
}
/// The head revision of the checkpoint `pointer` names.
fn checkpoint_revision(path: &Path, file: bool, pointer: &serde_json::Value) -> u64 {
    let checkpoint: serde_json::Value =
        serde_json::from_slice(&checkpoint_bytes(path, file, pointer)).unwrap();
    checkpoint["revision"].as_u64().unwrap()
}
/// How many checkpoint blobs the file provider holds.
fn retained_checkpoints(path: &Path) -> usize {
    std::fs::read_dir(path.join("blobs"))
        .unwrap()
        .filter(|entry| {
            std::fs::read(entry.as_ref().unwrap().path())
                .unwrap()
                .starts_with(br#"{"format":"ekr.replay-checkpoint/1""#)
        })
        .count()
}
/// The pointers, blobs and answers a store holds after the seed and `2n + 1` commits, `n` being
/// [`REPLAY_CHECKPOINT_COMMITS`]: a pointer from the seed and from each commit and from nothing
/// else, a checkpoint from the seed and the commits of revisions `n` and `2n`, and one blob.
fn assert_cadence(path: &Path, file: bool, how: &str) {
    let n = ekr_kernel::REPLAY_CHECKPOINT_COMMITS;
    let runtime = open(path, file);
    let written = pointers(&runtime);
    assert_eq!(
        written.len() as u64,
        1 + 2 * n + 1,
        "file={file}, {how}: the seed and each commit leave one pointer, and nothing else does"
    );
    // With one pointer from the seed and one from each commit, the pointer at position `k` is
    // the one revision `k` left.
    assert_eq!(
        checkpoints_written(&written),
        [0, n, 2 * n],
        "file={file}, {how}: the seed and every {n}th commit write a checkpoint"
    );
    let newest = written.last().unwrap();
    assert_eq!(
        newest["covered"].as_u64(),
        Some(revision_occurrences(&runtime)),
        "file={file}, {how}: the newest pointer covers the whole revision stream"
    );
    assert_eq!(checkpoint_revision(path, file, newest), 2 * n, "{how}");
    if file {
        assert_eq!(
            retained_checkpoints(path),
            1,
            "{how}: a newer checkpoint replaces the one before it"
        );
    }
}

/// After every commit, whether it wrote a checkpoint or not, a fresh open continues from the
/// retained checkpoint, replaying only what follows it, and answers as a full replay does.
#[test]
fn a_fresh_open_continues_from_the_checkpoint_with_the_answers_of_a_full_replay() {
    let n = ekr_kernel::REPLAY_CHECKPOINT_COMMITS;
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = build(path, file, 0);
        for revision in 1..=2 * n + 1 {
            commit(path, file, &seed, revision, revision - 1);
            let checkpointed = open(path, file);
            let answered = answers(&checkpointed);
            assert_eq!(
                answered,
                answers(&open_in_full(path, file)),
                "file={file}: after the commit of revision {revision}"
            );
            assert_eq!(answered.0.unwrap().revision, RevisionNumber::new(revision));
            assert_eq!(
                checkpointed.seed_replays(),
                0,
                "file={file}: revision {revision} is reached from the checkpoint, not the seed"
            );
        }
        assert_cadence(path, file, "one runtime per command");
    }
}

/// A runtime that serves every command, as `ekr session` holds one, writes checkpoints at the
/// same revisions as a runtime per command does.
#[test]
fn one_runtime_serving_every_command_writes_checkpoints_at_the_same_revisions() {
    let n = ekr_kernel::REPLAY_CHECKPOINT_COMMITS;
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = build(path, file, 0);
        let runtime = open(path, file);
        for revision in 1..=2 * n + 1 {
            let result = commit_by(
                || &runtime,
                &seed,
                seed.graph.root.id,
                revision,
                revision - 1,
                3,
            );
            assert!(matches!(result, CommitCommandResult::Committed(_)));
        }
        assert_eq!(
            answers(&runtime),
            answers(&open_in_full(path, file)),
            "file={file}"
        );
        drop(runtime);
        assert_eq!(
            answers(&open(path, file)),
            answers(&open_in_full(path, file)),
            "file={file}"
        );
        assert_cadence(path, file, "one runtime for every command");
    }
}

/// A proposal, a validation, a rejection and a stale commit move no head and write no checkpoint,
/// and append no pointer; a commit that moves the head appends one.
#[test]
fn a_verb_that_moves_no_head_appends_no_pointer() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = build(path, file, 1);
        let count = || pointers(&open(path, file)).len();
        let before = count();
        let operator = context().operator;
        let root = seed.graph.root.id;
        let (first, first_bytes) = document_under(&seed, root, 2, 3);
        let (second, second_bytes) = document_under(&seed, root, 3, 3);
        #[derive(Serialize)]
        struct Wire<'a> {
            format: &'static str,
            transaction: &'a GraphTransaction,
        }
        // A node of the edge type: refused by the type check.
        let refused = GraphTransaction {
            id: TransactionId::mint(),
            proposer: operator,
            operations: vec![GraphOperation::CreateNode(NodeDraft {
                id: NodeId::mint(),
                root_id: root,
                type_id: RELATES.parse().unwrap(),
                canonical_name: "not a subject".into(),
                properties: BTreeMap::new(),
                aliases: Vec::new(),
            })],
            evidence: BTreeSet::new(),
            schema_version: None,
        };
        let refused_bytes = serde_yaml_ng::to_string(&Wire {
            format: "ekr.transaction-document/1",
            transaction: &refused,
        })
        .unwrap()
        .into_bytes();
        for (bytes, at) in [
            (&first_bytes, 200),
            (&second_bytes, 201),
            (&refused_bytes, 202),
        ] {
            open(path, file)
                .propose(bytes, operator, || Timestamp::from_millis(at))
                .unwrap();
            assert_eq!(
                count(),
                before,
                "file={file}: a proposal appends no pointer"
            );
        }
        for (tx, at) in [(first, 210), (second, 211)] {
            let verdict = open(path, file)
                .validate(tx, RevisionNumber::new(1), || Timestamp::from_millis(at))
                .unwrap();
            assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
            assert_eq!(
                count(),
                before,
                "file={file}: a validation appends no pointer"
            );
        }
        let verdict = open(path, file)
            .validate(refused.id, RevisionNumber::new(1), || {
                Timestamp::from_millis(212)
            })
            .unwrap();
        assert!(
            matches!(verdict, ValidationCommandResult::Rejected(_)),
            "{verdict:?}"
        );
        assert_eq!(
            count(),
            before,
            "file={file}: a rejection appends no pointer"
        );
        let committed = open(path, file)
            .commit(first, operator, || Timestamp::from_millis(220))
            .unwrap();
        assert!(matches!(committed, CommitCommandResult::Committed(_)));
        assert_eq!(
            count(),
            before + 1,
            "file={file}: a commit that moves the head appends one"
        );
        let stale = open(path, file)
            .commit(second, operator, || Timestamp::from_millis(221))
            .unwrap();
        assert!(matches!(stale, CommitCommandResult::Stale(_)), "{stale:?}");
        assert_eq!(
            count(),
            before + 1,
            "file={file}: a stale commit appends no pointer"
        );
        assert_eq!(
            answers(&open(path, file)),
            answers(&open_in_full(path, file)),
            "file={file}"
        );
    }
}

/// A commit whose operations, with those committed since the retained checkpoint, reach
/// [`REPLAY_CHECKPOINT_OPERATIONS`] writes a checkpoint however few commits came before it.
#[test]
fn a_commit_that_reaches_the_operation_bound_writes_a_checkpoint() {
    const { assert!(ekr_kernel::REPLAY_CHECKPOINT_COMMITS > 3) };
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = build(path, file, 1);
        let seeded = pointers(&open(path, file)).pop().unwrap();
        assert_eq!(checkpoint_revision(path, file, &seeded), 0);
        // Each node is two operations, its creation and its assertion; revision 1 committed six.
        let nodes = (ekr_kernel::REPLAY_CHECKPOINT_OPERATIONS - 6).div_ceil(2);
        let under = nodes - 1;
        let result = commit_by(|| open(path, file), &seed, seed.graph.root.id, 2, 1, under);
        assert!(matches!(result, CommitCommandResult::Committed(_)));
        let newest = pointers(&open(path, file)).pop().unwrap();
        assert_eq!(
            newest["checkpoint_hash"],
            seeded["checkpoint_hash"],
            "file={file}: {} operations since the checkpoint write none",
            6 + 2 * under
        );
        let result = commit_by(|| open(path, file), &seed, seed.graph.root.id, 3, 2, 1);
        assert!(matches!(result, CommitCommandResult::Committed(_)));
        let newest = pointers(&open(path, file)).pop().unwrap();
        assert_eq!(
            checkpoint_revision(path, file, &newest),
            3,
            "file={file}: {} operations since the checkpoint write one",
            6 + 2 * under + 2
        );
        assert_eq!(
            answers(&open(path, file)),
            answers(&open_in_full(path, file)),
            "file={file}"
        );
    }
}

/// A handle that restored no checkpoint — one opened for full replay, or one whose checkpoint
/// did not verify — knows of none to continue from, and writes one at its next commit.
#[test]
fn a_handle_that_restored_no_checkpoint_writes_one_at_its_next_commit() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = build(path, file, 1);
        let result = commit_by(
            || open_in_full(path, file),
            &seed,
            seed.graph.root.id,
            2,
            1,
            3,
        );
        assert!(matches!(result, CommitCommandResult::Committed(_)));
        let newest = pointers(&open(path, file)).pop().unwrap();
        assert_eq!(checkpoint_revision(path, file, &newest), 2, "file={file}");
        assert_eq!(
            answers(&open(path, file)),
            answers(&open_in_full(path, file)),
            "file={file}"
        );
    }
}

/// Deletes the retained object at `hash` from the provider itself.
fn delete_object(path: &Path, file: bool, hash: ContentHash) {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let tenant = eventlog_core::TenantId::new(TENANT).unwrap();
    executor.block_on(async {
        use eventlog_core::EventStore;
        if file {
            let provider = eventlog_file::FileEventStore::open(path).await.unwrap();
            provider.delete_blob(&tenant, &hash.to_hex()).await.unwrap();
        } else {
            let provider = eventlog_sqlite::SqliteEventStore::open(
                &path.join("state.db").to_string_lossy(),
                "ekr",
            )
            .await
            .unwrap();
            provider.delete_blob(&tenant, &hash.to_hex()).await.unwrap();
        }
    });
}

/// `head` answers from the pointer alone, reading only the head revision's record, when the
/// newest pointer names every occurrence the stream holds: after a commit. After a proposal,
/// which appends no pointer, it replays from the checkpoint as every other read does.
#[test]
fn head_answers_from_the_pointer_alone_after_a_commit_and_replays_after_a_proposal() {
    for file in [false, true] {
        for propose_after in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let seed = build(path, file, 2);
            let head = open(path, file).head().unwrap();
            let records = open(path, file).transactions().unwrap();
            if propose_after {
                let (_, bytes) = document_under(&seed, seed.graph.root.id, 3, 1);
                open(path, file)
                    .propose(&bytes, context().operator, || Timestamp::from_millis(300))
                    .unwrap();
            }
            // Every replay needs the proposal record of revision 1's transaction.
            let first = records
                .values()
                .find(|record| {
                    record
                        .committed
                        .as_ref()
                        .is_some_and(|receipt| receipt.result.revision == RevisionNumber::new(1))
                })
                .unwrap();
            delete_object(path, file, first.proposal_record_hash);
            assert!(
                open_in_full(path, file).head().is_err(),
                "file={file}: a full replay needs every record"
            );
            if propose_after {
                assert!(
                    open(path, file).head().is_err(),
                    "file={file}: no pointer names the proposal, so head replays"
                );
            } else {
                assert_eq!(
                    open(path, file).head().unwrap(),
                    head,
                    "file={file}: the pointer names every occurrence"
                );
            }
        }
    }
}

type Forge = fn(&mut serde_json::Value);
/// Each forgery of a genuine checkpoint: the field it changes, what it changes it to, and how.
fn forgeries() -> Vec<(&'static str, &'static str, Forge)> {
    fn root(forged: &mut serde_json::Value) -> &mut serde_json::Value {
        &mut forged["graph"]["graph"]["root"]
    }
    vec![
        ("graph.graph.assertions", "a dropped assertion", |forged| {
            let assertions = forged["graph"]["graph"]["assertions"]
                .as_object_mut()
                .unwrap();
            let first = assertions.keys().next().unwrap().clone();
            assertions.remove(&first);
        }),
        ("authority", "another host", |forged| {
            forged["authority"] = serde_json::to_value(ContentHash::of_bytes(b"x")).unwrap();
        }),
        ("prefix", "another history", |forged| {
            forged["prefix"] = serde_json::to_value(ContentHash::of_bytes(b"y")).unwrap();
        }),
        ("revision", "an earlier head", |forged| {
            forged["revision"] = 1.into();
        }),
        ("format", "another format", |forged| {
            forged["format"] = "ekr.replay-checkpoint/0".into();
        }),
        ("covered", "a shorter coverage", |forged| {
            forged["covered"] = (forged["covered"].as_u64().unwrap() - 1).into();
        }),
        ("ontologies", "no schema versions", |forged| {
            forged["ontologies"] = serde_json::json!([]);
        }),
        ("graph.graph.root.id", "another graph root id", |forged| {
            root(forged)["id"] = serde_json::to_value(GraphRootId::mint()).unwrap();
        }),
        (
            "graph.graph.root.space",
            "a transient graph root",
            |forged| {
                root(forged)["space"] = serde_json::to_value(Space::Transient).unwrap();
            },
        ),
        (
            "graph.graph.root.schema_version_id",
            "another graph root schema version",
            |forged| {
                root(forged)["schema_version_id"] =
                    serde_json::to_value(SchemaVersionId::mint()).unwrap();
            },
        ),
        ("graph.graph.root.parent", "a graph root parent", |forged| {
            root(forged)["parent"] = serde_json::to_value(GraphRootId::mint()).unwrap();
        }),
        (
            "graph.graph.root.created_at",
            "another graph root creation time",
            |forged| {
                root(forged)["created_at"] =
                    serde_json::to_value(Timestamp::from_millis(123_456)).unwrap();
            },
        ),
        ("seed_payloads", "no seed payloads", |forged| {
            forged["seed_payloads"] = serde_json::json!([]);
        }),
        ("seed_payloads", "an extra seed payload", |forged| {
            forged["seed_payloads"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::to_value(ContentHash::of_bytes(b"z")).unwrap());
        }),
    ]
}
/// Each forgery of the identities a profile-v3 checkpoint says its lineage held.
fn held_forgeries() -> Vec<(&'static str, &'static str, Forge)> {
    vec![
        ("held", "a dropped deleted edge id", |forged| {
            for at in forged["held"].as_array_mut().unwrap() {
                at["edges"] = serde_json::json!([]);
            }
        }),
        ("held", "an id held from a later revision", |forged| {
            let at = forged["held"].as_array_mut().unwrap().last_mut().unwrap();
            at["from"] = (at["from"].as_u64().unwrap() + 1).into();
        }),
        ("held", "an extra held node id", |forged| {
            forged["held"][0]["nodes"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::to_value(NodeId::mint()).unwrap());
        }),
        ("held", "no held ids", |forged| {
            forged.as_object_mut().unwrap().remove("held");
        }),
    ]
}

#[test]
fn a_checkpoint_that_does_not_verify_is_ignored_and_the_history_replays_in_full() {
    forged_checkpoints_are_ignored(false);
}

/// The same under profile v3, whose checkpoint also carries the identities every revision held,
/// over a lineage that created an edge and deleted it: only its checkpoint names that edge's id.
#[test]
fn a_v3_checkpoint_that_does_not_verify_is_ignored_and_the_history_replays_in_full() {
    KEEPS_IDENTITIES.set(true);
    forged_checkpoints_are_ignored(true);
}

fn forged_checkpoints_are_ignored(v3: bool) {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        // Committed up to the next checkpoint, so that the genuine checkpoint is a commit's and
        // covers the whole stream; under v3, one written after the edge's deletion.
        let (seed, head) = if v3 {
            let (seed, deleted) = build_with_a_deleted_edge(path, file);
            let head = commit_to_checkpoint(path, file, &seed, 4);
            assert!(!open(path, file)
                .snapshot()
                .unwrap()
                .edges
                .contains_key(&deleted));
            (seed, head)
        } else {
            let seed = build(path, file, 2);
            let head = commit_to_checkpoint(path, file, &seed, 2);
            (seed, head)
        };
        let truth = answers(&open_in_full(path, file));
        let evidence = seed_evidence(&open_in_full(path, file), &seed);
        assert!(
            !evidence.is_empty() && evidence.iter().all(Option::is_some),
            "file={file}: a full replay reads every seed evidence payload"
        );
        let newest = pointers(&open(path, file)).pop().unwrap();
        let genuine: serde_json::Value =
            serde_json::from_slice(&checkpoint_bytes(path, file, &newest)).unwrap();
        assert_eq!(genuine["revision"].as_u64(), Some(head), "file={file}");
        assert_eq!(genuine["covered"], newest["covered"], "file={file}");
        // The genuine pointer's coverage and binding, so that only the checkpoint is forged.
        let covered = newest["covered"].as_u64().unwrap();
        let binding: ContentHash = serde_json::from_value(newest["binding"].clone()).unwrap();
        // The genuine checkpoint is taken: an open continues from it and replays nothing.
        let genuine_open = open(path, file);
        assert_eq!(answers(&genuine_open), truth, "file={file}");
        assert_eq!(genuine_open.seed_replays(), 0, "file={file}");
        let mut forgeries = forgeries();
        if v3 {
            forgeries.extend(held_forgeries());
        } else {
            // A v1 store keeps no identities, and its checkpoint says none: its bytes are the
            // ones written before profile v3 existed.
            assert!(genuine.get("held").is_none(), "file={file}: {genuine}");
            forgeries.push(("held", "held ids in a store that keeps none", |forged| {
                forged["held"] = serde_json::json!([
                    { "from": 1, "nodes": [NodeId::mint()], "edges": [] }
                ]);
            }));
        }
        // Every field a checkpoint carries, and every field of the graph root it carries, is
        // forged by at least one case: a field no case changes is a field nothing shows is bound.
        let forged_paths: BTreeSet<&str> = forgeries.iter().map(|(path, _, _)| *path).collect();
        let mut fields: Vec<String> = genuine
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| *key != "graph")
            .cloned()
            .collect();
        fields.extend(
            genuine["graph"]["graph"]["root"]
                .as_object()
                .unwrap()
                .keys()
                .map(|key| format!("graph.graph.root.{key}")),
        );
        fields.push("graph.graph.assertions".into());
        for field in &fields {
            assert!(
                forged_paths.contains(field.as_str()),
                "no forgery changes the checkpoint's {field}"
            );
        }
        for (field, name, forge) in forgeries {
            let mut forged = genuine.clone();
            forge(&mut forged);
            assert_ne!(forged, genuine, "the forgery with {name} changes {field}");
            let bytes = serde_json::to_vec(&forged).unwrap();
            install(path, file, covered, binding, &bytes);
            assert_eq!(
                pointers(&open(path, file)).pop().unwrap()["checkpoint_hash"],
                serde_json::to_value(ContentHash::of_bytes(&bytes)).unwrap(),
                "file={file}: the forgery with {name} is the newest checkpoint"
            );
            let forged_open = open(path, file);
            assert_eq!(
                answers(&forged_open),
                truth,
                "file={file}: a checkpoint with {name} is ignored"
            );
            assert!(
                forged_open.seed_replays() > 0,
                "file={file}: a checkpoint with {name} is ignored, and the open replays from the \
                 seed"
            );
            assert_eq!(
                seed_evidence(&open(path, file), &seed),
                evidence,
                "file={file}: after a checkpoint with {name}, a verified read holds the seed's \
                 evidence bytes"
            );
        }
        // A pointer whose binding is not this host's own, and one that claims more than the
        // stream holds, are not taken as a verification of the head.
        let original = serde_json::to_vec(&genuine).unwrap();
        for (claimed, binding) in [
            (covered, ContentHash::of_bytes(b"another binding")),
            (covered + 1, binding),
        ] {
            install(path, file, claimed, binding, &original);
            assert_eq!(answers(&open(path, file)), truth, "file={file}");
        }
    }
}

/// A checkpoint whose graph root is not the one the seed admitted must not become the root the
/// kernel's next decisions record: a transaction filed under the root a non-full open reports
/// commits, and a full replay then answers exactly as the non-full open does.
#[test]
fn a_commit_after_a_checkpoint_with_another_graph_root_replays_in_full() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = build(path, file, 2);
        let head = commit_to_checkpoint(path, file, &seed, 2);
        let newest = pointers(&open(path, file)).pop().unwrap();
        let mut forged: serde_json::Value =
            serde_json::from_slice(&checkpoint_bytes(path, file, &newest)).unwrap();
        assert_eq!(forged["revision"].as_u64(), Some(head), "file={file}");
        forged["graph"]["graph"]["root"]["id"] = serde_json::to_value(GraphRootId::mint()).unwrap();
        let covered = newest["covered"].as_u64().unwrap();
        let binding: ContentHash = serde_json::from_value(newest["binding"].clone()).unwrap();
        install(
            path,
            file,
            covered,
            binding,
            &serde_json::to_vec(&forged).unwrap(),
        );
        let reported = open(path, file).snapshot().unwrap().root.id;
        let result = commit_under(path, file, &seed, reported, head + 1, head);
        assert!(
            matches!(result, CommitCommandResult::Committed(_)),
            "file={file}: {result:?}"
        );
        // The committing handle admitted no checkpoint, so its commit wrote one.
        let newest = pointers(&open(path, file)).pop().unwrap();
        assert_eq!(
            checkpoint_revision(path, file, &newest),
            head + 1,
            "file={file}"
        );
        let replayed = open_in_full(path, file);
        let replayed = (
            replayed.head(),
            replayed.snapshot(),
            replayed.transactions(),
        );
        assert!(
            replayed.0.is_ok() && replayed.1.is_ok() && replayed.2.is_ok(),
            "file={file}: a full replay admits the history the kernel wrote: {replayed:?}"
        );
        assert_eq!(
            answers(&open(path, file)),
            (
                replayed.0.unwrap(),
                replayed.1.unwrap(),
                replayed.2.unwrap()
            ),
            "file={file}"
        );
        assert_eq!(
            answers(&open(path, file)).0.unwrap().revision,
            RevisionNumber::new(head + 1),
            "file={file}"
        );
    }
}

#[test]
fn validating_against_a_revision_the_checkpoint_holds_no_graph_of_replays_in_full() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = build(path, file, 2);
        let head = commit_to_checkpoint(path, file, &seed, 2);
        let newest = pointers(&open(path, file)).pop().unwrap();
        assert_eq!(checkpoint_revision(path, file, &newest), head);
        // Validated against revision 1 while the head, the checkpoint's, is later: the commit is
        // stale, and both the validation and the stale decision agree with a full replay.
        let result = commit(path, file, &seed, head + 1, 1);
        assert!(
            matches!(result, CommitCommandResult::Stale(_)),
            "file={file}: {result:?}"
        );
        assert_eq!(
            answers(&open(path, file)),
            answers(&open_in_full(path, file)),
            "file={file}"
        );
    }
}

/// Every retained proposal and commit receipt is the current format, its document bytes one
/// base64 string; every preparation holds each staged object once, as base64, and no separate
/// blob list; and a preparation is not much larger than the objects it stages.
#[test]
fn retained_records_and_preparations_hold_each_payload_once_as_base64() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    build(path, true, 2);
    let mut formats = BTreeMap::<String, usize>::new();
    for entry in std::fs::read_dir(path.join("blobs")).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let Some(format) = value["format"].as_str() else {
            continue;
        };
        *formats.entry(format.to_owned()).or_default() += 1;
        match format {
            "ekr.proposal-record/2" => assert!(value["document_bytes"].is_string()),
            "ekr.commit-receipt/2" => {
                assert!(value["proposal"]["document_bytes"].is_string());
            }
            "ekr.publication-preparation/2" => {
                assert!(value["native_request"].get("blobs").is_none());
                let objects = value["decision"]["objects"].as_object().unwrap();
                let mut staged = 0;
                for object in objects.values() {
                    staged += object["bytes"].as_str().unwrap().len();
                }
                assert!(
                    bytes.len() < staged + 8192,
                    "a preparation of {} bytes stages {staged} bytes of base64",
                    bytes.len()
                );
            }
            _ => {}
        }
    }
    for format in [
        "ekr.proposal-record/2",
        "ekr.commit-receipt/2",
        "ekr.publication-preparation/2",
    ] {
        assert!(formats.contains_key(format), "{format} in {formats:?}");
    }
    for format in [
        "ekr.proposal-record/1",
        "ekr.commit-receipt/1",
        "ekr.publication-preparation/1",
    ] {
        assert!(!formats.contains_key(format), "{format} in {formats:?}");
    }
}

/// A store handle whose checkpoint writes can be made to lose, as a pointer append loses to
/// another writer twice over: nothing is written and the store says so.
struct Losing<S> {
    inner: S,
    lose: std::rc::Rc<Cell<bool>>,
}
impl<S: RevisionLog> RevisionLog for Losing<S> {
    fn preparation(
        &self,
        key: &ekr_store::PublicationCommandKey,
    ) -> Result<Option<ekr_store::PublicationPreparationV1>, ekr_store::StoreError> {
        self.inner.preparation(key)
    }
    fn prepare(
        &self,
        key: &ekr_store::PublicationCommandKey,
        input: ContentHash,
        decision: &ekr_store::Publication,
        previous: Option<&ekr_store::PublicationPreparationV1>,
    ) -> Result<ekr_store::PublicationPreparationV1, ekr_store::StoreError> {
        self.inner.prepare(key, input, decision, previous)
    }
    fn resume(
        &self,
        prepared: &ekr_store::PublicationPreparationV1,
    ) -> Result<ekr_store::Appended, ekr_store::StoreError> {
        self.inner.resume(prepared)
    }
    fn history(&self) -> Result<ekr_store::RetainedHistory, ekr_store::StoreError> {
        self.inner.history()
    }
    fn history_at(
        &self,
        revision: RevisionNumber,
    ) -> Result<ekr_store::RetainedHistory, ekr_store::StoreError> {
        self.inner.history_at(revision)
    }
    fn publish(
        &self,
        publication: &ekr_store::Publication,
    ) -> Result<ekr_store::Appended, ekr_store::StoreError> {
        self.inner.publish(publication)
    }
    fn seed_bytes(&self) -> Result<Option<Vec<u8>>, ekr_store::StoreError> {
        self.inner.seed_bytes()
    }
    fn fold(&self) -> Result<CanonicalGraph, ekr_store::StoreError> {
        self.inner.fold()
    }
    fn head(&self) -> Result<Option<Root>, ekr_store::StoreError> {
        self.inner.head()
    }
    fn replay(&self, revision: RevisionNumber) -> Result<CanonicalGraph, ekr_store::StoreError> {
        self.inner.replay(revision)
    }
    fn write_checkpoint(
        &self,
        covered: u64,
        binding: ContentHash,
        checkpoint: Option<&[u8]>,
    ) -> Result<bool, ekr_store::StoreError> {
        if checkpoint.is_some() && self.lose.get() {
            return Ok(false);
        }
        self.inner.write_checkpoint(covered, binding, checkpoint)
    }
}
impl<S: ekr_store::Initialize> ekr_store::Initialize for Losing<S> {
    fn initialize(
        &self,
        publication: &ekr_store::Publication,
    ) -> Result<ekr_store::Appended, ekr_store::StoreError> {
        self.inner.initialize(publication)
    }
}
impl<S: ekr_store::ObjectStore> ekr_store::ObjectStore for Losing<S> {
    fn put(
        &self,
        class: ekr_store::StorageClass,
        bytes: &[u8],
        at: Timestamp,
    ) -> Result<ekr_store::StoredObject, ekr_store::StoreError> {
        self.inner.put(class, bytes, at)
    }
    fn get(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, ekr_store::StoreError> {
        self.inner.get(hash)
    }
}

/// Runs the commits of revisions `1..=to` through `kernel`, each against the one before, with the
/// checkpoint write of revision `lost` losing.
fn commit_through<S>(kernel: &Commit<S>, lose: &Cell<bool>, seed: &SeedDocument, to: u64, lost: u64)
where
    S: RevisionLog + ekr_store::ObjectStore,
{
    for n in 1..=to {
        let at = i64::try_from(n * 100).unwrap();
        let (tx, bytes) = document_under(seed, seed.graph.root.id, n, 1);
        kernel
            .propose(&bytes, context().operator, || Timestamp::from_millis(at))
            .unwrap();
        kernel
            .validate(tx, RevisionNumber::new(n - 1), || {
                Timestamp::from_millis(at + 1)
            })
            .unwrap();
        lose.set(n == lost);
        let result = kernel
            .commit(tx, context().operator, || Timestamp::from_millis(at + 2))
            .unwrap();
        lose.set(false);
        assert!(matches!(result, CommitCommandResult::Committed(_)));
    }
}

/// A checkpoint the store did not write — its pointer lost to another writer — is not taken for
/// the retained one: the handle's next commit writes it, so the head stays within the bound of the
/// newest checkpoint (design § 99.1).
#[test]
fn a_checkpoint_the_store_did_not_write_is_written_at_the_next_commit() {
    fn run<S>(
        seed: &SeedDocument,
        store: impl FnOnce(KernelAuthority) -> Result<S, ekr_store::StoreError>,
    ) where
        S: RevisionLog + ekr_store::ObjectStore + ekr_store::Initialize,
    {
        let n = ekr_kernel::REPLAY_CHECKPOINT_COMMITS;
        let lose = std::rc::Rc::new(Cell::new(false));
        let kernel = Commit::over_with_authority(context(), anchor(), |authority| {
            Ok(Losing {
                inner: store(authority)?,
                lose: lose.clone(),
            })
        })
        .unwrap();
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        // The commit of revision n is due and its checkpoint write loses.
        commit_through(&kernel, &lose, seed, n + 1, n);
    }
    let n = ekr_kernel::REPLAY_CHECKPOINT_COMMITS;
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = seed();
        if file {
            run(&seed, |authority| {
                Ok(ekr_store::FileStore::file(path, TENANT, None)?.under(authority))
            });
        } else {
            run(&seed, |authority| {
                Ok(
                    ekr_store::SqliteStore::sqlite(&path.join("state.db"), TENANT, None)?
                        .under(authority),
                )
            });
        }
        let newest = pointers(&open(path, file)).pop().unwrap();
        assert_eq!(
            checkpoint_revision(path, file, &newest),
            n + 1,
            "file={file}: the lost checkpoint of revision {n} is written by the next commit"
        );
        assert_eq!(
            answers(&open(path, file)),
            answers(&open_in_full(path, file)),
            "file={file}"
        );
    }
}
