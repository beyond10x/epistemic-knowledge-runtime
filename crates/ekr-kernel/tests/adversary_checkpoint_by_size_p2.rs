//! Adversary pass on `task:checkpoint-cadence-by-size` (design § 99.5), on both providers.
//!
//! - A handle at rest writes the checkpoint of the newest head *it* reached. When another process
//!   has since committed and checkpointed a later head, the store's retained checkpoint must not
//!   move back behind that one.
//! - The two § 99.5.1 item 2 conditions besides "head past the retained one" — an authority that
//!   knows of no retained checkpoint, and a validation after it against an earlier revision —
//!   each make a handle at rest write one.
//! - Documents of exactly `REPLAY_CHECKPOINT_BYTES` ("at least", § 99.5.1 item 1) make the
//!   commit that reaches them write a checkpoint.
//! - After 0 to 12 commits, by one-shot handles or by one handle then at rest, under profiles v1,
//!   v2 and v3, a fresh open replays nothing from the seed and answers as a full replay does, and
//!   under v3 a deleted edge's id stays held.
//! - A `/3` receipt whose `created` does not name what its transaction created changes no answer
//!   and frees no identity.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{Cardinality, EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use ekr_store::{RetainedHistory, RetainedObject, RevisionLog, StoredObject};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::Arc;

const TENANT: &str = "bysize";
/// The seed's edge type, from `Subject` to `Subject`.
const RELATES: &str = "00000000-0000-4000-8000-000000000008";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Profile {
    V1,
    V2,
    V3,
}

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}
fn anchor(profile: Profile) -> AuthorityStateV1 {
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
        validation_profile: match profile {
            Profile::V1 => ValidationProfileV1::deterministic(c.validator),
            Profile::V2 => ValidationProfileV1::schema_evolving(c.validator),
            Profile::V3 => ValidationProfileV1::identity_keeping(c.validator),
        },
    }
}
fn open(path: &Path, file: bool, profile: Profile) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), anchor(profile))
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), anchor(profile))
    }
    .unwrap()
}
fn open_in_full(path: &Path, file: bool, profile: Profile) -> Runtime {
    let mut runtime = open(path, file, profile);
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
fn transaction(
    operations: Vec<GraphOperation>,
    evidence: BTreeSet<EvidenceId>,
) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence,
        schema_version: None,
    }
}
/// `nodes` new `Subject` nodes filed under the seed's root, each with one evidenced assertion.
fn nodes_tx(seed: &SeedDocument, n: u64, nodes: u64) -> GraphTransaction {
    let ty = &seed.ontology.node_types[0];
    let label = *ty.properties.keys().next().unwrap();
    let evidence = *seed.graph.evidence.keys().next().unwrap();
    let root = seed.graph.root.id;
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
    transaction(operations, BTreeSet::from([evidence]))
}
fn edge_tx(seed: &SeedDocument, id: EdgeId, source: NodeId, target: NodeId) -> GraphTransaction {
    transaction(
        vec![GraphOperation::CreateEdge(EdgeDraft {
            id,
            root_id: seed.graph.root.id,
            type_id: RELATES.parse().unwrap(),
            source,
            target,
            properties: BTreeMap::new(),
        })],
        BTreeSet::new(),
    )
}
fn wire(tx: &GraphTransaction, format: &'static str) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format,
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}
fn encode(tx: &GraphTransaction) -> Vec<u8> {
    wire(
        tx,
        if tx.operations.len() > 256 {
            "ekr.transaction-document/2"
        } else {
            "ekr.transaction-document/1"
        },
    )
}
fn ts(n: u64, step: i64) -> Timestamp {
    Timestamp::from_millis(i64::try_from(n * 100).unwrap() + step)
}
/// Proposes `bytes` (the document of `tx`), validates it against `against` and commits it, each
/// through `runtime`, at times ordered by `n`.
fn run_bytes(
    runtime: &Runtime,
    tx: TransactionId,
    bytes: &[u8],
    against: u64,
    n: u64,
) -> CommitCommandResult {
    runtime
        .propose(bytes, context().operator, || ts(n, 0))
        .unwrap();
    let verdict = runtime
        .validate(tx, RevisionNumber::new(against), || ts(n, 1))
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    runtime.commit(tx, context().operator, || ts(n, 2)).unwrap()
}
fn run(runtime: &Runtime, tx: &GraphTransaction, against: u64, n: u64) -> CommitCommandResult {
    run_bytes(runtime, tx.id, &encode(tx), against, n)
}
fn committed(result: &CommitCommandResult) -> bool {
    matches!(result, CommitCommandResult::Committed(_))
}
type Answers = (
    Option<Root>,
    CanonicalGraph,
    Arc<BTreeMap<TransactionId, TransactionRecord>>,
);
fn answers(runtime: &Runtime) -> Answers {
    (
        runtime.head().unwrap(),
        runtime.snapshot().unwrap(),
        runtime.transactions().unwrap(),
    )
}
/// Every `ekr.store.CheckpointWritten` pointer the log holds.
fn pointers(runtime: &Runtime) -> Vec<serde_json::Value> {
    runtime
        .published_events()
        .unwrap()
        .into_iter()
        .filter(|event| event.name == "ekr.store.CheckpointWritten")
        .map(|event| event.data)
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
fn native(
    path: &Path,
    file: bool,
) -> (tokio::runtime::Runtime, Box<dyn eventlog_core::EventStore>) {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let provider: Box<dyn eventlog_core::EventStore> = executor.block_on(async {
        if file {
            Box::new(eventlog_file::FileEventStore::open(path).await.unwrap())
                as Box<dyn eventlog_core::EventStore>
        } else {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(
                    &path.join("state.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap(),
            )
        }
    });
    (executor, provider)
}
/// The bytes of the checkpoint `pointer` names, if the provider still holds them.
fn checkpoint_bytes(path: &Path, file: bool, pointer: &serde_json::Value) -> Option<Vec<u8>> {
    let (executor, provider) = native(path, file);
    let digest = format!(
        "ekr.private.checkpoint.{}",
        pointer["checkpoint_hash"].as_str().unwrap()
    );
    let tenant = eventlog_core::TenantId::new(TENANT).unwrap();
    executor
        .block_on(provider.get_blob(&tenant, &digest))
        .unwrap()
}
/// The head revision of the checkpoint the newest pointer names.
fn newest_checkpoint_revision(path: &Path, file: bool, profile: Profile) -> u64 {
    let newest = pointers(&open(path, file, profile)).pop().unwrap();
    let bytes = checkpoint_bytes(path, file, &newest).expect("the newest checkpoint is retained");
    let checkpoint: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    checkpoint["revision"].as_u64().unwrap()
}
fn seeded(path: &Path, file: bool, profile: Profile) -> SeedDocument {
    let seed = seed();
    open(path, file, profile)
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    seed
}

/// Two processes on one store: a session commits revision 1 and then reads nothing more, while
/// another process commits revisions 2 to `REPLAY_CHECKPOINT_COMMITS`, the last of which writes
/// the store's checkpoint of that head. When the session's input then ends, what it writes at
/// rest is the checkpoint of revision 1, the newest head *it* reached, and the store takes it for
/// the newest and deletes the later one: every later open continues from revision 1 again.
#[test]
fn a_handle_at_rest_does_not_move_the_retained_checkpoint_back_behind_another_writers() {
    let n = REPLAY_CHECKPOINT_COMMITS;
    let mut after_rest = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = seeded(path, file, Profile::V1);
        let session = open(path, file, Profile::V1);
        assert!(committed(&run(&session, &nodes_tx(&seed, 1, 3), 0, 1)));
        for revision in 2..=n {
            let other = open(path, file, Profile::V1);
            assert!(committed(&run(
                &other,
                &nodes_tx(&seed, revision, 3),
                revision - 1,
                revision
            )));
        }
        assert_eq!(
            newest_checkpoint_revision(path, file, Profile::V1),
            n,
            "file={file}: the other process's commit of revision {n} wrote the store's checkpoint"
        );
        session.retain_checkpoint_at_rest();
        after_rest.push((file, newest_checkpoint_revision(path, file, Profile::V1)));
    }
    assert_eq!(
        after_rest,
        [(false, n), (true, n)],
        "(file, the store's checkpoint revision after the session came to rest): the session \
         moved it back from revision {n}"
    );
}

/// A handle that admitted no checkpoint — here one opened for full replay — writes the
/// checkpoint of the head it reached when it comes to rest (design § 99.5.1 item 2: "an
/// authority that knows of no retained checkpoint").
#[test]
fn a_handle_that_admitted_no_checkpoint_leaves_one_at_rest() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = seeded(path, file, Profile::V1);
        for revision in 1..=2 {
            let one_shot = open(path, file, Profile::V1);
            assert!(committed(&run(
                &one_shot,
                &nodes_tx(&seed, revision, 2),
                revision - 1,
                revision
            )));
        }
        assert_eq!(newest_checkpoint_revision(path, file, Profile::V1), 0);
        let full = open_in_full(path, file, Profile::V1);
        full.snapshot().unwrap();
        full.retain_checkpoint_at_rest();
        assert_eq!(
            newest_checkpoint_revision(path, file, Profile::V1),
            2,
            "file={file}: a handle that knows of no retained checkpoint writes one at rest"
        );
    }
}

/// A validation made after the retained checkpoint against a revision before its head is one an
/// open continuing from that checkpoint must replay from the seed to re-derive. A handle at rest
/// whose newest head is the checkpoint's own writes one covering it (design § 99.5.1 item 2), so
/// that the next open replays nothing from the seed.
#[test]
fn a_handle_at_rest_after_an_early_basis_validation_leaves_a_checkpoint_covering_it() {
    let n = REPLAY_CHECKPOINT_COMMITS;
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = seeded(path, file, Profile::V1);
        for revision in 1..=n {
            let one_shot = open(path, file, Profile::V1);
            assert!(committed(&run(
                &one_shot,
                &nodes_tx(&seed, revision, 2),
                revision - 1,
                revision
            )));
        }
        assert_eq!(newest_checkpoint_revision(path, file, Profile::V1), n);
        let session = open(path, file, Profile::V1);
        let early = nodes_tx(&seed, n + 1, 2);
        session
            .propose(&encode(&early), context().operator, || ts(n + 1, 0))
            .unwrap();
        let verdict = session
            .validate(early.id, RevisionNumber::new(n - 1), || ts(n + 1, 1))
            .unwrap();
        assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
        session.retain_checkpoint_at_rest();
        let newest = pointers(&open(path, file, Profile::V1)).pop().unwrap();
        assert_eq!(
            newest["covered"].as_u64(),
            Some(revision_occurrences(&open(path, file, Profile::V1))),
            "file={file}: the checkpoint written at rest covers the early-basis validation"
        );
        let fresh = open(path, file, Profile::V1);
        let answered = answers(&fresh);
        assert_eq!(fresh.seed_replays(), 0, "file={file}");
        assert_eq!(
            answered,
            answers(&open_in_full(path, file, Profile::V1)),
            "file={file}"
        );
    }
}

/// `document`'s bytes padded to exactly `length`: nodes under the seed's root whose names carry
/// the bytes, the last one's name lengthened until the `/2` document is `length` long.
fn document_of_length(seed: &SeedDocument, n: u64, length: usize) -> (TransactionId, Vec<u8>) {
    // Well under the 65,536-byte string limit, so that the last name can take up the rest.
    const NAME: usize = 20_000;
    let ty = &seed.ontology.node_types[0];
    let node = |k: usize, name: usize| {
        GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id: seed.graph.root.id,
            type_id: ty.id,
            canonical_name: format!("{n}.{k} {}", "x".repeat(name)),
            properties: BTreeMap::new(),
            aliases: Vec::new(),
        })
    };
    let count = (length - 4_096) / (NAME + 300);
    let mut tx = transaction((0..count).map(|k| node(k, NAME)).collect(), BTreeSet::new());
    let mut last = NAME;
    for _ in 0..8 {
        let bytes = wire(&tx, "ekr.transaction-document/2");
        if bytes.len() == length {
            return (tx.id, bytes);
        }
        last = (last + length).checked_sub(bytes.len()).unwrap();
        let GraphOperation::CreateNode(draft) = tx.operations.last_mut().unwrap() else {
            unreachable!()
        };
        draft.canonical_name = format!("{n}.{} {}", count - 1, "x".repeat(last));
    }
    panic!("no document of exactly {length} bytes");
}

/// § 99.5.1 item 1: a commit's checkpoint is due when the documents committed since the retained
/// one "hold together at least `REPLAY_CHECKPOINT_BYTES`". Two `/2` documents of the input cap
/// each, 8 MiB, hold exactly 16 MiB: the commit of the second writes one.
#[test]
fn documents_of_exactly_the_byte_bound_write_a_checkpoint() {
    const CAP: usize = 8_388_608;
    assert_eq!(2 * CAP as u64, REPLAY_CHECKPOINT_BYTES);
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = seeded(path, file, Profile::V1);
        let session = open(path, file, Profile::V1);
        for revision in 1..=2 {
            let (tx, bytes) = document_of_length(&seed, revision, CAP);
            assert_eq!(bytes.len(), CAP);
            assert!(committed(&run_bytes(
                &session,
                tx,
                &bytes,
                revision - 1,
                revision
            )));
            assert_eq!(
                newest_checkpoint_revision(path, file, Profile::V1),
                if revision == 2 { 2 } else { 0 },
                "file={file}: {} bytes of documents since the seed's checkpoint",
                revision as usize * CAP
            );
        }
    }
}

/// The transaction of revision `k` of [`reopen_sequence`]: nodes, except that revision 3 creates
/// an edge between two nodes of revision 1 and revision 4 deletes it.
fn revision_tx(
    seed: &SeedDocument,
    k: u64,
    first: &mut Vec<NodeId>,
    edge: EdgeId,
) -> GraphTransaction {
    match k {
        3 => edge_tx(seed, edge, first[0], first[1]),
        4 => transaction(vec![GraphOperation::DeleteEdge(edge)], BTreeSet::new()),
        _ => {
            let tx = nodes_tx(seed, k, 2);
            if k == 1 {
                first.extend(tx.operations.iter().filter_map(|op| match op {
                    GraphOperation::CreateNode(draft) => Some(draft.id),
                    _ => None,
                }));
            }
            tx
        }
    }
}

/// Under v3 a transaction re-creating the deleted edge's id is refused
/// `identity-previously-held`, through the checkpoint path.
fn reuse_is_refused(
    path: &Path,
    file: bool,
    seed: &SeedDocument,
    edge: EdgeId,
    first: &[NodeId],
    head: u64,
    how: &str,
) {
    let fresh = open(path, file, Profile::V3);
    let tx = edge_tx(seed, edge, first[1], first[0]);
    fresh
        .propose(&encode(&tx), context().operator, || ts(head + 1, 0))
        .unwrap();
    let verdict = fresh
        .validate(tx.id, RevisionNumber::new(head), || ts(head + 1, 1))
        .unwrap();
    let ValidationCommandResult::Rejected(record) = verdict else {
        panic!("{how}: the deleted edge's id was reused: {verdict:?}");
    };
    assert!(
        record
            .issues
            .iter()
            .any(|issue| issue.code == "identity-previously-held"),
        "{how}: {:?}",
        record.issues
    );
}

/// A fresh open replays nothing from the seed and answers as a full replay does.
fn reopens_as_a_full_replay(path: &Path, file: bool, profile: Profile, how: &str) {
    let fresh = open(path, file, profile);
    let answered = answers(&fresh);
    assert_eq!(
        fresh.seed_replays(),
        0,
        "{how}: a fresh open replayed from the seed"
    );
    assert_eq!(
        answered,
        answers(&open_in_full(path, file, profile)),
        "{how}"
    );
}

/// Seeds under `profile`, then commits revisions 1 to 12 in runs of 0, 1, 2, 3, 4 and 2 commits,
/// so that the head after a run is 0, 1, 3, 6, 10 and 12. With `session`, each run is one handle
/// that then comes to rest; otherwise every command is a fresh handle. After every commit, and
/// every rest, a fresh open is held to a full replay; under v3 the edge revision 3 created and
/// revision 4 deleted stays held at the end.
fn reopen_sequence(profile: Profile, file: bool, session: bool) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    let seed = seeded(path, file, profile);
    let edge = EdgeId::mint();
    let mut first = Vec::new();
    let mut head = 0;
    for run_of in [0, 1, 2, 3, 4, 2] {
        let held = open(path, file, profile);
        for _ in 0..run_of {
            head += 1;
            let how = format!("{profile:?} file={file} session={session} head={head}");
            let tx = revision_tx(&seed, head, &mut first, edge);
            let result = if session {
                run(&held, &tx, head - 1, head)
            } else {
                run(&open(path, file, profile), &tx, head - 1, head)
            };
            assert!(committed(&result), "{how}");
            if !session {
                reopens_as_a_full_replay(path, file, profile, &how);
            }
        }
        if session {
            let how = format!("{profile:?} file={file} session at rest at {head}");
            assert_eq!(
                held.seed_replays(),
                0,
                "{how}: the session replayed from the seed"
            );
            held.retain_checkpoint_at_rest();
            assert_eq!(
                newest_checkpoint_revision(path, file, profile),
                head,
                "{how}: at rest the session leaves the checkpoint of its head"
            );
            reopens_as_a_full_replay(path, file, profile, &how);
        }
    }
    if profile == Profile::V3 {
        let how = format!("{profile:?} file={file} session={session}");
        reuse_is_refused(path, file, &seed, edge, &first, head, &how);
        reopens_as_a_full_replay(path, file, profile, &how);
    }
}

#[test]
fn after_0_to_12_commits_a_fresh_open_answers_as_a_full_replay_under_every_profile() {
    for profile in [Profile::V1, Profile::V2, Profile::V3] {
        for file in [false, true] {
            reopen_sequence(profile, file, false);
        }
    }
}

#[test]
fn after_sessions_of_0_to_4_commits_at_rest_a_fresh_open_answers_as_a_full_replay() {
    for profile in [Profile::V1, Profile::V2, Profile::V3] {
        for file in [false, true] {
            reopen_sequence(profile, file, true);
        }
    }
}

/// The retained history of the SQLite store at `path`.
fn captured_history(path: &Path, profile: Profile) -> RetainedHistory {
    let mut history = None;
    let _kernel = Commit::over_with_authority(context(), anchor(profile), |authority| {
        let store =
            ekr_store::SqliteStore::sqlite(&path.join("state.db"), TENANT, None)?.under(authority);
        history = Some(store.history()?);
        Ok(store)
    })
    .unwrap();
    history.unwrap()
}
/// Writes objects and revision occurrences straight through the native provider, bypassing
/// every kernel and store check.
fn install_history(path: &Path, file: bool, history: &RetainedHistory) {
    use eventlog_core::{CommandMeta, Expected, NewEvent, StreamId, TenantId};
    let (executor, provider) = native(path, file);
    executor.block_on(async {
        let tenant = TenantId::new(TENANT).unwrap();
        let meta = |key: String| CommandMeta {
            idempotency_key: key.clone(),
            request_hash: key.clone(),
            subject: "fixture".into(),
            actor: "fixture".into(),
            request_id: key.clone(),
            trace_id: key,
            causation_id: None,
            causation_depth: 0,
            occurred_at: ::time::OffsetDateTime::UNIX_EPOCH,
            claim: None,
        };
        for (hash, object) in &history.objects {
            provider
                .put_blob(&tenant, &hash.to_hex(), &object.bytes)
                .await
                .unwrap();
            let stream = StreamId::new(tenant.clone(), "ekr.store.object", hash.to_hex()).unwrap();
            let event = NewEvent::new(
                "ekr.store.ObjectStored",
                2,
                serde_json::to_value(&object.metadata).unwrap(),
            )
            .unwrap();
            provider
                .append(
                    &stream,
                    Expected::NoStream,
                    &[event],
                    &meta(format!("object-{hash}")),
                )
                .await
                .unwrap();
        }
        let stream = StreamId::new(tenant, "ekr.revision", "canonical").unwrap();
        let events = history
            .occurrences
            .iter()
            .map(|o| {
                NewEvent::new(o.event.name(), 2, serde_json::to_value(&o.event).unwrap()).unwrap()
            })
            .collect::<Vec<_>>();
        provider
            .append(
                &stream,
                Expected::NoStream,
                &events,
                &meta("history".into()),
            )
            .await
            .unwrap();
    });
}

/// A v3 lineage whose last commit created edge `E`, with that commit's `/3` receipt re-written
/// to name, in `created`, none of the edges it created and a node it did not create: the lie
/// is carried by the receipt the commit's event names, so the prefix digest binds it. Whatever an
/// open then does with it — refuse the store, or read it — it must not change an answer, and
/// `E`, deleted by the next commit, must stay held against reuse on the checkpoint path.
#[test]
fn a_receipt_whose_created_list_lies_changes_no_answer_and_frees_no_identity() {
    let profile = Profile::V3;
    let source = tempfile::tempdir().unwrap();
    let seed = seeded(source.path(), false, profile);
    let edge = EdgeId::mint();
    let mut first = Vec::new();
    for revision in 1..=2 {
        let tx = if revision == 1 {
            revision_tx(&seed, 1, &mut first, edge)
        } else {
            edge_tx(&seed, edge, first[0], first[1])
        };
        assert!(committed(&run(
            &open(source.path(), false, profile),
            &tx,
            revision - 1,
            revision
        )));
    }
    let mut forged = captured_history(source.path(), profile);
    let at = forged.occurrences.len() - 1;
    let old = forged.occurrences[at].event.record_hash;
    let mut receipt = CommitReceiptV1::from_bytes(&forged.objects[&old].bytes).unwrap();
    assert_eq!(receipt.format, CommitReceiptV1::FORMAT);
    let created = receipt.created.as_mut().unwrap();
    assert!(created.edges.remove(&edge));
    created.nodes.insert(NodeId::mint());
    let bytes = receipt.to_bytes().unwrap();
    let hash = ContentHash::of_bytes(&bytes);
    let metadata = StoredObject {
        content_hash: hash,
        byte_len: bytes.len() as u64,
        ..forged.objects[&old].metadata.clone()
    };
    forged.objects.remove(&old);
    forged.objects.insert(
        hash,
        RetainedObject {
            metadata,
            bytes: Arc::new(bytes),
        },
    );
    forged.occurrences[at].event.record_hash = hash;

    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        install_history(path, file, &forged);
        let full = match open_in_full(path, file, profile).head() {
            Err(refused) => {
                // A refusal of the lie is an answer: no open reads it.
                assert_eq!(
                    open(path, file, profile).head().err(),
                    Some(refused),
                    "file={file}"
                );
                continue;
            }
            Ok(head) => head,
        };
        assert_eq!(open(path, file, profile).head().unwrap(), full);
        // Revision 3 deletes the edge; the commit writes the checkpoint of revision 3.
        let delete = transaction(vec![GraphOperation::DeleteEdge(edge)], BTreeSet::new());
        assert!(committed(&run(&open(path, file, profile), &delete, 2, 3)));
        assert_eq!(newest_checkpoint_revision(path, file, profile), 3);
        let fresh = open(path, file, profile);
        let answered = answers(&fresh);
        assert_eq!(
            answered,
            answers(&open_in_full(path, file, profile)),
            "file={file}"
        );
        reuse_is_refused(path, file, &seed, edge, &first, 3, &format!("file={file}"));
        let fresh = open(path, file, profile);
        let answered = answers(&fresh);
        assert_eq!(
            answered,
            answers(&open_in_full(path, file, profile)),
            "file={file}"
        );
        eprintln!(
            "file={file}: a fresh open of the store holding the lie replayed from the seed {} time(s)",
            fresh.seed_replays()
        );
    }
}
