//! Adversary pass, wave perf-02, unit M (`story:session-keeps-only-the-head-graph`).
//!
//! A session now keeps the head graph (and the retained checkpoint's) and rebuilds any other
//! revision's graph by a verified replay. These cases drive that from the public `Runtime` only,
//! on both providers:
//!
//! - a validation against revision N uses N's graph, not the head's: a transaction naming a node
//!   that revision 3 created is rejected against 0 and 2 and validated against 3 and 5, in a
//!   session that committed past them, after a checkpoint restore, and from a second handle;
//! - the store those decisions leave behind replays in full from the seed and migrates;
//! - a read of an earlier revision does not leave its graph held once the read returned;
//! - a long session holds a bounded number of graphs.
//!
//! Whether the kernel still holds a graph is observed through a `Weak` to the `Arc` a read
//! returned: the kernel shares its graph with the reader rather than copying it, so a live `Weak`
//! after every strong handle the test held is dropped means the kernel still holds that graph.

use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Weak};

const TENANT: &str = "adversary-m";

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
        validation_profile: ValidationProfileV1::deterministic(c.validator),
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
fn how(file: bool) -> &'static str {
    if file {
        "file"
    } else {
        "sqlite"
    }
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
fn label_of(seed: &SeedDocument) -> PropertyId {
    *seed.ontology.node_types[0]
        .properties
        .keys()
        .next()
        .unwrap()
}
fn evidence_of(seed: &SeedDocument) -> EvidenceId {
    *seed.graph.evidence.keys().next().unwrap()
}
fn assertion(seed: &SeedDocument, node: NodeId, text: String) -> GraphOperation {
    GraphOperation::AddAssertion(Box::new(Assertion {
        id: AssertionId::mint(),
        root_id: seed.graph.root.id,
        subject: Subject::Node(node),
        predicate: Predicate::Property(label_of(seed)),
        object: Object::Value(Value::String(text)),
        evidence: BTreeSet::from([evidence_of(seed)]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }))
}
fn document(seed: &SeedDocument, operations: Vec<GraphOperation>) -> (TransactionId, Vec<u8>) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: BTreeSet::from([evidence_of(seed)]),
        schema_version: None,
    };
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: &tx,
    })
    .unwrap()
    .into_bytes();
    (tx.id, bytes)
}
/// One new node with one evidenced assertion; returns the node.
fn new_node(seed: &SeedDocument, n: u64) -> (NodeId, (TransactionId, Vec<u8>)) {
    let id = NodeId::mint();
    let operations = vec![
        GraphOperation::CreateNode(NodeDraft {
            id,
            root_id: seed.graph.root.id,
            type_id: seed.ontology.node_types[0].id,
            canonical_name: format!("subject {n}"),
            properties: BTreeMap::new(),
            aliases: Vec::new(),
        }),
        assertion(seed, id, format!("label {n}")),
    ];
    (id, document(seed, operations))
}
/// A monotonic clock: every call is one millisecond after the previous one.
struct Clock(std::cell::Cell<i64>);
impl Clock {
    fn new() -> Self {
        Self(std::cell::Cell::new(1_000))
    }
    fn tick(&self) -> Timestamp {
        let now = self.0.get() + 1;
        self.0.set(now);
        Timestamp::from_millis(now)
    }
}
/// Commits revision `n` through `runtime`, a new node, validated against the head. Returns it.
fn commit_node(runtime: &Runtime, seed: &SeedDocument, clock: &Clock, n: u64) -> NodeId {
    let (node, (tx, bytes)) = new_node(seed, n);
    runtime
        .propose(&bytes, context().operator, || clock.tick())
        .unwrap();
    let head = runtime.head().unwrap().unwrap().revision;
    let verdict = runtime.validate(tx, head, || clock.tick()).unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let result = runtime
        .commit(tx, context().operator, || clock.tick())
        .unwrap();
    assert!(
        matches!(result, CommitCommandResult::Committed(_)),
        "{result:?}"
    );
    node
}
/// Proposes a transaction asserting a label on `node` and validates it against `against`.
fn validate_naming(
    runtime: &Runtime,
    seed: &SeedDocument,
    clock: &Clock,
    node: NodeId,
    against: u64,
) -> ValidationCommandResult {
    let (tx, bytes) = document(seed, vec![assertion(seed, node, format!("on {against}"))]);
    runtime
        .propose(&bytes, context().operator, || clock.tick())
        .unwrap();
    runtime
        .validate(tx, RevisionNumber::new(against), || clock.tick())
        .unwrap()
}
fn kind(verdict: &ValidationCommandResult) -> &'static str {
    match verdict {
        ValidationCommandResult::Validated(_) => "validated",
        ValidationCommandResult::Rejected(_) => "rejected",
    }
}
/// Seeds the store at `path` and commits six revisions through one session; returns the seed and
/// the node each revision created, by revision.
fn build(session: &Runtime, clock: &Clock) -> (SeedDocument, BTreeMap<u64, NodeId>) {
    let seed = seed();
    session.seed(seed.clone(), || clock.tick()).unwrap();
    let nodes = (1..=6)
        .map(|n| (n, commit_node(session, &seed, clock, n)))
        .collect();
    (seed, nodes)
}
/// Every verdict a transaction naming the node revision 3 created receives against revisions 0,
/// 2, 3 and 5, through `runtime`.
fn verdicts(
    runtime: &Runtime,
    seed: &SeedDocument,
    clock: &Clock,
    node: NodeId,
) -> Vec<(u64, &'static str)> {
    [0, 2, 3, 5]
        .into_iter()
        .map(|against| {
            (
                against,
                kind(&validate_naming(runtime, seed, clock, node, against)),
            )
        })
        .collect()
}
const EXPECTED: [(u64, &str); 4] = [
    (0, "rejected"),
    (2, "rejected"),
    (3, "validated"),
    (5, "validated"),
];
/// The store at `path` replays in full from the seed and answers what a checkpointed open does.
fn replays_in_full(path: &Path, file: bool) {
    let full = open_in_full(path, file).read(None).unwrap();
    let opened = open(path, file).read(None).unwrap();
    assert_eq!(full.root, opened.root, "{}", how(file));
    assert_eq!(full.transactions, opened.transactions, "{}", how(file));
    for number in 0..=full.root.revision.get() {
        let number = RevisionNumber::new(number);
        assert_eq!(
            open_in_full(path, file).schema_history(number).unwrap(),
            open(path, file).schema_history(number).unwrap(),
            "{}: revision {number}",
            how(file)
        );
    }
}

/// A validation against an earlier revision is decided on that revision's graph, whichever way
/// the handle reached its state: the session that committed past it, a fresh open restored from
/// the checkpoint, and a second handle over a store another handle moved.
#[test]
fn a_validation_against_an_earlier_revision_is_decided_on_that_revisions_graph() {
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let clock = Clock::new();
        let session = open(path, file);
        let (seed, nodes) = build(&session, &clock);
        let node = nodes[&3];
        assert_eq!(
            verdicts(&session, &seed, &clock, node),
            EXPECTED,
            "{}: the session that committed revisions 1 to 6",
            how(file)
        );
        let restored = open(path, file);
        assert_eq!(
            verdicts(&restored, &seed, &clock, node),
            EXPECTED,
            "{}: a fresh open, restored from the checkpoint",
            how(file)
        );
        // Another handle moves the store on; the session validates again against the same
        // revisions, and so does a reader that has read revision 2 before.
        let reader = open(path, file);
        let _ = reader.schema_history(RevisionNumber::new(2)).unwrap();
        commit_node(&open(path, file), &seed, &clock, 7);
        assert_eq!(
            verdicts(&session, &seed, &clock, node),
            EXPECTED,
            "{}: the session after another handle committed revision 7",
            how(file)
        );
        assert_eq!(
            verdicts(&reader, &seed, &clock, node),
            EXPECTED,
            "{}: a reader after another handle committed revision 7",
            how(file)
        );
        replays_in_full(path, file);
    }
}

/// The graph a read of an earlier revision returns is not held by the handle once the read
/// returned and the reader dropped it: only the head's and the retained checkpoint's are kept
/// (the `Revision` documentation: every other graph is released when the head moves past it;
/// the `keeping` hint lasts "while a read that asked for it runs").
#[test]
fn a_read_of_an_earlier_revision_leaves_its_graph_unheld_once_it_returned() {
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let clock = Clock::new();
        let (seed, _) = build(&open(path, file), &clock);
        // Revision 5 lies after the checkpoint at 4 and before the head at 6, so the replay that
        // verifies the history passes it and the read's hint applies to it.
        let reader = open(path, file);
        let read = reader.schema_history(RevisionNumber::new(5)).unwrap();
        let weak: Weak<CanonicalGraph> = Arc::downgrade(&read.graph);
        drop(read);
        let after_read = weak.strong_count();
        // Whatever it still held then, the reader's own next commit releases.
        commit_node(&reader, &seed, &clock, 7);
        let after_commit = weak.strong_count();
        assert_eq!(
            (after_read, after_commit),
            (0, 0),
            "{}: strong references the handle holds to revision 5's graph (after the read \
             returned, after the handle's next commit)",
            how(file)
        );
    }
}

/// A long session — commits, reads of older revisions, validations against older revisions,
/// and another handle's commits — holds at most a handful of graphs at any point: the head's,
/// the retained checkpoint's and the few a command in flight keeps.
#[test]
fn a_long_session_holds_a_bounded_number_of_graphs() {
    const BOUND: usize = 3;
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let clock = Clock::new();
        let session = open(path, file);
        let (seed, mut nodes) = build(&session, &clock);
        let mut graphs: BTreeMap<u64, Weak<CanonicalGraph>> = BTreeMap::new();
        let mut worst = (0, 0, Vec::new());
        for n in 7..=30 {
            nodes.insert(n, commit_node(&session, &seed, &clock, n));
            // The head read shares the head graph with the session: every revision's graph is
            // observed at the moment it was the head.
            let head = session.schema_history(RevisionNumber::new(n)).unwrap();
            graphs.insert(n, Arc::downgrade(&head.graph));
            drop(head);
            let old = n - 5;
            let past = session.schema_history(RevisionNumber::new(old)).unwrap();
            graphs
                .entry(old)
                .or_insert_with(|| Arc::downgrade(&past.graph));
            drop(past);
            if n % 3 == 0 {
                let verdict = validate_naming(&session, &seed, &clock, nodes[&(n - 2)], n - 4);
                assert_eq!(kind(&verdict), "rejected", "{}: {n}", how(file));
            }
            if n % 5 == 0 {
                commit_node(&open(path, file), &seed, &clock, 1000 + n);
            }
            let held: Vec<u64> = graphs
                .iter()
                .filter(|(_, weak)| weak.strong_count() > 0)
                .map(|(number, _)| *number)
                .collect();
            if held.len() > worst.1 {
                worst = (n, held.len(), held);
            }
        }
        assert!(
            worst.1 <= BOUND,
            "{}: after command round {} the session held {} graphs, of revisions {:?}",
            how(file),
            worst.0,
            worst.1,
            worst.2
        );
    }
}

/// A store holding a validation against the seed and one against revision 2 that were never
/// committed, a rejection against revision 1 and a stale commit migrates, and the destination
/// holds the same decisions.
#[test]
fn a_store_with_pending_validations_against_early_revisions_migrates() {
    for (file, into_file) in [(true, false), (false, true), (true, true), (false, false)] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let clock = Clock::new();
        let session = open(path, file);
        let (seed, nodes) = build(&session, &clock);
        // Pending: validated against the seed and against revision 2, never committed.
        let (_, (tx, bytes)) = new_node(&seed, 100);
        session
            .propose(&bytes, context().operator, || clock.tick())
            .unwrap();
        assert_eq!(
            kind(
                &session
                    .validate(tx, RevisionNumber::SEED, || clock.tick())
                    .unwrap()
            ),
            "validated"
        );
        assert_eq!(
            kind(&validate_naming(&session, &seed, &clock, nodes[&1], 2)),
            "validated"
        );
        assert_eq!(
            kind(&validate_naming(&session, &seed, &clock, nodes[&3], 1)),
            "rejected"
        );
        let (_, (stale, bytes)) = new_node(&seed, 101);
        session
            .propose(&bytes, context().operator, || clock.tick())
            .unwrap();
        session
            .validate(stale, RevisionNumber::new(4), || clock.tick())
            .unwrap();
        let result = session
            .commit(stale, context().operator, || clock.tick())
            .unwrap();
        assert!(
            matches!(result, CommitCommandResult::Stale(_)),
            "{result:?}"
        );
        let into = tempfile::tempdir().unwrap();
        let destination = open(into.path(), into_file);
        session.migrate_into(&destination).unwrap();
        let migrated = open_in_full(into.path(), into_file).read(None).unwrap();
        let source = open_in_full(path, file).read(None).unwrap();
        assert_eq!(migrated.root, source.root);
        let states = |read: &VerifiedRead| {
            read.transactions
                .iter()
                .map(|(id, record)| (*id, record.state()))
                .collect::<BTreeMap<_, _>>()
        };
        assert_eq!(states(&migrated), states(&source));
    }
}
