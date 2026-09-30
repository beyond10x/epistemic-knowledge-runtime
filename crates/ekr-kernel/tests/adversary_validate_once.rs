//! Adversary, wave ops-05, unit W: `task:validate-builds-one-view-per-command`.
//!
//! The unit leaves a validate decision's verdict in a thread-local for the replay that admits the
//! publication, and keeps each revision's per-edge assertion index with its graph. These cases
//! drive what the unit's own cases do not reach:
//!
//! - a session's verdicts, over seeded multi-revision histories with edges, assertions about
//!   edges, deletions, retractions and validations against older revisions, against a fresh
//!   pipeline run on the replayed graph of the named revision, under profiles v1 to v3 on both
//!   providers, with every root held against a full replay from the seed;
//! - a verdict a validate command decided and did not take, released when the command ends;
//! - another writer's validation of a different document against the same revision landing
//!   between the decision and the admitting replay;
//! - a revision's edge index released with its graph.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::validate::{candidates_built, edge_indexes_built, HeldIdentities};
use ekr_kernel::*;
use ekr_ontology::{EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};

const TENANT: &str = "adversary-validate-once";

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}
fn anchor(profile: ValidationProfileV1) -> AuthorityStateV1 {
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
        validation_profile: profile,
    }
}
fn profiles() -> [(&'static str, AuthorityStateV1); 3] {
    let v = context().validator;
    [
        ("v1", anchor(ValidationProfileV1::deterministic(v))),
        ("v2", anchor(ValidationProfileV1::schema_evolving(v))),
        ("v3", anchor(ValidationProfileV1::identity_keeping(v))),
    ]
}
fn open(path: &Path, file: bool, authority: &AuthorityStateV1) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), authority.clone())
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), authority.clone())
    }
    .unwrap()
}
fn how(profile: &str, file: bool) -> String {
    format!("{profile} {}", if file { "file" } else { "sqlite" })
}

/// A monotonic clock shared by every handle and thread of one case.
struct Clock(AtomicI64);
impl Clock {
    fn new() -> Self {
        Self(AtomicI64::new(1_000))
    }
    fn tick(&self) -> Timestamp {
        Timestamp::from_millis(self.0.fetch_add(1, Ordering::SeqCst) + 1)
    }
}

const K_NODE: u16 = 0x0a01;
const K_EDGE: u16 = 0x0a02;
const K_ASSERT: u16 = 0x0a03;
const K_TX: u16 = 0x0a04;
const NODES: u64 = 8;
const SEEDED_NODES: u64 = 3;
const EDGES: u64 = 8;
const ASSERTIONS: u64 = 12;

fn uuid(kind: u16, n: u64) -> String {
    format!("00000000-{kind:04x}-4000-8000-{n:012x}")
}
fn node(n: u64) -> NodeId {
    uuid(K_NODE, n).parse().unwrap()
}
fn edge(n: u64) -> EdgeId {
    uuid(K_EDGE, n).parse().unwrap()
}
fn assertion(n: u64) -> AssertionId {
    uuid(K_ASSERT, n).parse().unwrap()
}
fn transaction_id(n: u64) -> TransactionId {
    uuid(K_TX, n).parse().unwrap()
}

/// SplitMix64: the same draws on every machine and every Rust release.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn chance(&mut self, percent: u64) -> bool {
        self.below(100) < percent
    }
}

/// `Subject` with a `label`; `REL` (many) and `ONE` (one) from `Subject` to `Subject`; three
/// seeded subjects and one retained evidence entry.
struct World {
    document: SeedDocument,
    subject: TypeId,
    rel: TypeId,
    one: TypeId,
    label: PropertyId,
    evidence: EvidenceId,
}
fn world() -> World {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let (subject, rel, one, label) = (
        TypeId::mint(),
        TypeId::mint(),
        TypeId::mint(),
        PropertyId::mint(),
    );
    let mut declared = NodeType::new(subject, "Subject");
    declared.properties.insert(
        label,
        PropertyDefinition::new(label, "label", ValueType::String),
    );
    document.ontology.node_types.push(declared);
    for (id, name, cardinality) in [
        (rel, "REL", ekr_ontology::Cardinality::Many),
        (one, "ONE", ekr_ontology::Cardinality::One),
    ] {
        let mut edge_type = EdgeType::new(id, name);
        edge_type.source_types = BTreeSet::from([subject]);
        edge_type.target_types = BTreeSet::from([subject]);
        edge_type.cardinality = cardinality;
        document.ontology.edge_types.push(edge_type);
    }
    for n in 0..SEEDED_NODES {
        let seeded = Node::<Value>::new(node(n), document.graph.root.id, subject, "seeded");
        document.graph.nodes.insert(seeded.id, seeded);
    }
    let bytes = b"seeded statement".to_vec();
    let entry = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: ContentHash::of_bytes(&bytes),
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(5),
        confidence: Confidence::CERTAIN,
    };
    let evidence = entry.id;
    document.graph.evidence.insert(evidence, entry.clone());
    document
        .evidence_payloads
        .insert(entry.content_hash, bytes.into());
    World {
        document,
        subject,
        rel,
        one,
        label,
        evidence,
    }
}
impl World {
    fn root(&self) -> GraphRootId {
        self.document.graph.root.id
    }
    fn about(&self, id: AssertionId, subject: Subject<NodeId, EdgeId>) -> GraphOperation {
        GraphOperation::AddAssertion(Box::new(Assertion {
            id,
            root_id: self.root(),
            subject,
            predicate: Predicate::Property(self.label),
            object: Object::Value(Value::String(format!("claim {id}"))),
            evidence: BTreeSet::from([self.evidence]),
            proposed_by: context().operator,
            assessment: Assessment::Proposed,
            lifecycle: AssertionLifecycle::Active,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        }))
    }
    fn create_edge(
        &self,
        id: EdgeId,
        type_id: TypeId,
        source: NodeId,
        target: NodeId,
    ) -> GraphOperation {
        GraphOperation::CreateEdge(EdgeDraft {
            id,
            root_id: self.root(),
            type_id,
            source,
            target,
            properties: BTreeMap::new(),
        })
    }
    /// One operation drawn from `rng` against `graph`, the head: mostly one that names what the
    /// head holds, weighted toward what the edge index decides — edges created and deleted, and
    /// assertions about edges, including deleting an edge canonical state asserts about.
    fn operation(&self, rng: &mut Rng, graph: &CanonicalGraph, fresh: &mut u64) -> GraphOperation {
        let mut mint = || {
            *fresh += 1;
            *fresh
        };
        let nodes: Vec<NodeId> = graph.nodes.keys().copied().collect();
        let edges: Vec<EdgeId> = graph.edges.keys().copied().collect();
        let asserted: Vec<EdgeId> = graph
            .assertions
            .values()
            .filter_map(|assertion| match assertion.subject {
                Subject::Edge(edge) => Some(edge.id()),
                _ => None,
            })
            .collect();
        let assertions: Vec<AssertionId> = graph.assertions.keys().copied().collect();
        let pick_node = |rng: &mut Rng| pick(rng, &nodes).unwrap_or_else(|| node(rng.below(NODES)));
        match rng.below(20) {
            0..=2 => {
                let id = if rng.chance(90) {
                    node(100 + mint())
                } else {
                    pick_node(rng)
                };
                GraphOperation::CreateNode(NodeDraft {
                    id,
                    root_id: self.root(),
                    type_id: self.subject,
                    canonical_name: format!("node {id}"),
                    properties: BTreeMap::new(),
                    aliases: Vec::new(),
                })
            }
            3..=7 => {
                let id = match pick(rng, &edges) {
                    Some(held) if rng.chance(10) => held,
                    _ => edge(100 + mint()),
                };
                let type_id = if rng.chance(80) { self.rel } else { self.one };
                let (source, target) = (pick_node(rng), pick_node(rng));
                self.create_edge(id, type_id, source, target)
            }
            8..=11 => GraphOperation::DeleteEdge(
                match rng.below(10) {
                    0..=4 => pick(rng, &asserted),
                    5..=8 => pick(rng, &edges),
                    _ => None,
                }
                .unwrap_or_else(|| edge(rng.below(EDGES))),
            ),
            12..=16 => {
                let subject = match rng.below(10) {
                    0..=6 => {
                        Subject::Edge(pick(rng, &edges).unwrap_or_else(|| edge(rng.below(EDGES))))
                    }
                    7..=8 => Subject::Node(pick_node(rng)),
                    _ => Subject::Edge(edge(rng.below(EDGES))),
                };
                let id = if rng.chance(95) {
                    assertion(100 + mint())
                } else {
                    assertion(rng.below(ASSERTIONS))
                };
                self.about(id, subject)
            }
            17..=18 => GraphOperation::RetractAssertion(Retraction {
                assertion: pick(rng, &assertions)
                    .unwrap_or_else(|| assertion(rng.below(ASSERTIONS))),
                reason: RetractionReason::new("withdrawn"),
            }),
            _ => GraphOperation::UpdateProperty(PropertyMutation {
                node: pick_node(rng),
                property: self.label,
                values: vec![Value::String("relabelled".into())],
            }),
        }
    }
    /// A new subject with an evidenced label: every profile accepts it.
    fn filler(&self, id: TransactionId) -> GraphTransaction {
        let fresh = NodeId::mint();
        transaction(
            id,
            vec![
                GraphOperation::CreateNode(NodeDraft {
                    id: fresh,
                    root_id: self.root(),
                    type_id: self.subject,
                    canonical_name: format!("node {fresh}"),
                    properties: BTreeMap::new(),
                    aliases: Vec::new(),
                }),
                self.about(AssertionId::mint(), Subject::Node(fresh)),
            ],
        )
    }
}
fn pick<T: Copy>(rng: &mut Rng, from: &[T]) -> Option<T> {
    if from.is_empty() {
        None
    } else {
        Some(from[rng.below(from.len() as u64) as usize])
    }
}
fn transaction(id: TransactionId, operations: Vec<GraphOperation>) -> GraphTransaction {
    let mut evidence = BTreeSet::new();
    for operation in &operations {
        if let GraphOperation::AddAssertion(assertion) = operation {
            evidence.extend(assertion.evidence.iter().copied());
        }
    }
    GraphTransaction {
        id,
        proposer: context().operator,
        operations,
        evidence,
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

/// What a verdict says, without the identities a publication mints: accepted, or every issue in
/// the order raised.
type Said = Result<(), Vec<(TransactionId, String, String, String)>>;
fn said_by_command(result: &ValidationCommandResult) -> Said {
    match result {
        ValidationCommandResult::Validated(_) => Ok(()),
        ValidationCommandResult::Rejected(record) => Err(record
            .issues
            .iter()
            .map(|issue| {
                (
                    issue.transaction_id,
                    format!("{:?}", issue.validator),
                    issue.code.clone(),
                    issue.message.clone(),
                )
            })
            .collect()),
    }
}
/// What a fresh pipeline of `profile` says about `tx` against the last of `graphs`, the graphs
/// of revisions seed through the basis: its own candidate view and its own edge index.
fn said_by_pipeline(profile: &str, graphs: &[CanonicalGraph], tx: &GraphTransaction) -> Said {
    let validator = context().validator;
    let lineage: BTreeSet<SchemaVersionId> = graphs
        .iter()
        .flat_map(|graph| {
            let version = graph.ontology.version();
            [Some(version.id), version.parent]
        })
        .flatten()
        .collect();
    let pipeline = match profile {
        "v1" => Pipeline::deterministic(validator),
        "v2" => Pipeline::schema_evolving(validator, lineage),
        _ => Pipeline::identity_keeping(validator, lineage, HeldIdentities::of(graphs)),
    };
    let basis = graphs.last().unwrap();
    match pipeline.validate(&GraphSnapshot::of(basis), tx) {
        Ok(_) => Ok(()),
        Err(issues) => Err(issues
            .into_iter()
            .map(|issue| {
                (
                    issue.transaction_id,
                    format!("{:?}", issue.validator),
                    issue.code,
                    issue.message,
                )
            })
            .collect()),
    }
}

/// A fresh handle replaying from the seed reaches the head and every published root.
fn replays(path: &Path, file: bool, authority: &AuthorityStateV1, published: &[Root], at: &str) {
    let mut full = open(path, file, authority);
    full.set_full_replay(true);
    assert_eq!(
        full.head().unwrap(),
        published.last().copied(),
        "{at}: full-replay head"
    );
    for root in published {
        let graph = full.replay(root.revision).unwrap();
        assert_eq!(
            (
                ekr_store::knowledge_root(&graph),
                ekr_store::evidence_root(&graph),
                ContentHash::of(&graph.ontology),
            ),
            (root.knowledge_root, root.evidence_root, root.ontology_root),
            "{at}: revision {}",
            root.revision
        );
    }
}

/// Over seeded histories — edges created and deleted, assertions about edges added and
/// retracted, validations against the head and against older revisions, commits and stale
/// commits — every verdict a session's validate command publishes is the one a fresh pipeline
/// reaches on the replayed graph of the revision it names, each command builds one candidate
/// view and at most one edge index, and every published root replays from the seed.
#[test]
fn a_session_says_what_a_fresh_pipeline_says_over_seeded_histories() {
    const STEPS: u64 = 36;
    let mut reached: BTreeMap<&'static str, usize> = BTreeMap::new();
    for (profile, authority) in profiles() {
        for file in [true, false] {
            let at = how(profile, file);
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let clock = Clock::new();
            let w = world();
            let session = open(path, file, &authority);
            session.seed(w.document.clone(), || clock.tick()).unwrap();
            let reference = open(path, file, &authority);
            let mut graphs = vec![reference.replay(RevisionNumber::SEED).unwrap()];
            let mut published: Vec<Root> = Vec::new();
            let mut rng = Rng(0x5eed_0000 + u64::from(file));
            let mut older: Vec<TransactionId> = Vec::new();
            let mut fresh = 0;
            for step in 0..STEPS {
                let operations = (0..1 + rng.below(3))
                    .map(|_| w.operation(&mut rng, graphs.last().unwrap(), &mut fresh))
                    .collect();
                let tx = transaction(transaction_id(step), operations);
                session
                    .propose(&encode(&tx), context().operator, || clock.tick())
                    .unwrap();
                let head = graphs.len() as u64 - 1;
                let against = if head == 0 || rng.chance(70) {
                    head
                } else {
                    rng.below(head)
                };
                let (views, indexes) = (candidates_built(), edge_indexes_built());
                let result = session
                    .validate(tx.id, RevisionNumber::new(against), || clock.tick())
                    .unwrap();
                let (views, indexes) = (candidates_built() - views, edge_indexes_built() - indexes);
                let expected = said_by_pipeline(profile, &graphs[..=against as usize], &tx);
                assert_eq!(
                    said_by_command(&result),
                    expected,
                    "{at} step {step} against {against}"
                );
                // A validation against a revision whose graph the session released rebuilds it by
                // a replay that revalidates; that cost is its own case below.
                if against == head {
                    assert_eq!(views, 1, "{at} step {step}: candidate views");
                    assert!(indexes <= 1, "{at} step {step}: edge indexes {indexes}");
                }
                match (&result, against == head) {
                    (ValidationCommandResult::Rejected(_), _) => {
                        *reached.entry("rejected").or_default() += 1;
                    }
                    (ValidationCommandResult::Validated(_), false) => {
                        *reached
                            .entry("validated against an older revision")
                            .or_default() += 1;
                        older.push(tx.id);
                    }
                    (ValidationCommandResult::Validated(_), true) => {
                        *reached.entry("validated against the head").or_default() += 1;
                        if rng.chance(75) {
                            let committed = session
                                .commit(tx.id, context().operator, || clock.tick())
                                .unwrap();
                            let CommitCommandResult::Committed(receipt) = committed else {
                                panic!("{at} step {step}: {committed:?}");
                            };
                            published.push(receipt.result);
                            graphs.push(reference.replay(receipt.result.revision).unwrap());
                        }
                    }
                }
                if let Some(id) = older.pop() {
                    if rng.chance(50) {
                        let stale = session
                            .commit(id, context().operator, || clock.tick())
                            .unwrap();
                        assert!(
                            matches!(stale, CommitCommandResult::Stale(_)),
                            "{at} step {step}: {stale:?}"
                        );
                        *reached.entry("stale").or_default() += 1;
                    } else {
                        older.push(id);
                    }
                }
            }
            if !published.is_empty() {
                replays(path, file, &authority, &published, &at);
            }
        }
    }
    for kind in [
        "rejected",
        "validated against the head",
        "validated against an older revision",
        "stale",
    ] {
        assert!(
            reached.get(kind).copied().unwrap_or(0) >= 3,
            "the generator reached {kind} too rarely: {reached:?}"
        );
    }
}

/// A validate command decides its verdict and is then refused before anything admits it (its
/// clock reads earlier than the proposal). Another writer, on another thread, then publishes a
/// validation of the same proposal against the same revision; this handle's next read replays it
/// from the state its refused command left, whose revision already holds its edge index. That
/// replay validates: the refused command's verdict was released when the command ended, and was
/// not taken by a replay outside it.
#[test]
fn a_verdict_a_validate_command_decided_and_did_not_take_is_released_when_the_command_ends() {
    for (profile, authority) in profiles() {
        for file in [true, false] {
            let at = how(profile, file);
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let clock = Clock::new();
            let w = world();
            let mine = open(path, file, &authority);
            mine.seed(w.document.clone(), || clock.tick()).unwrap();
            let tx = w.filler(TransactionId::mint());
            mine.propose(&encode(&tx), context().operator, || clock.tick())
                .unwrap();

            let views = candidates_built();
            let refused = mine.validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(0));
            assert!(refused.is_err(), "{at}: {refused:?}");
            assert_eq!(
                candidates_built() - views,
                1,
                "{at}: the refused command decided its verdict"
            );

            std::thread::scope(|scope| {
                scope
                    .spawn(|| {
                        let theirs = open(path, file, &authority);
                        let verdict = theirs
                            .validate(tx.id, RevisionNumber::SEED, || clock.tick())
                            .unwrap();
                        assert!(
                            matches!(verdict, ValidationCommandResult::Validated(_)),
                            "{at}: the other writer: {verdict:?}"
                        );
                    })
                    .join()
                    .unwrap();
            });

            let (views, indexes) = (candidates_built(), edge_indexes_built());
            mine.read(None).unwrap();
            assert_eq!(
                edge_indexes_built() - indexes,
                0,
                "{at}: the read continued from the state the refused command left"
            );
            assert_eq!(
                candidates_built() - views,
                1,
                "{at}: the read's replay of the other writer's validation validated it"
            );
        }
    }
}

/// Between this handle's decision and the replay that admits its publication, another writer on
/// another thread validates a different proposal against the same revision, so this handle's
/// publication is retried over a stream that holds the other validation. Both are validated, the
/// one this handle commits publishes the root a full replay from the seed reaches, and no record
/// disagrees with what that replay derives.
#[test]
fn a_validation_another_writer_lands_between_decision_and_admission_is_not_given_this_verdict() {
    for (profile, authority) in profiles() {
        for file in [true, false] {
            let at = how(profile, file);
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let clock = Clock::new();
            let w = world();
            let mine = open(path, file, &authority);
            mine.seed(w.document.clone(), || clock.tick()).unwrap();
            let (first, other) = (
                w.filler(TransactionId::mint()),
                w.filler(TransactionId::mint()),
            );
            for tx in [&first, &other] {
                mine.propose(&encode(tx), context().operator, || clock.tick())
                    .unwrap();
            }
            let verdict = mine
                .validate(first.id, RevisionNumber::SEED, || {
                    std::thread::scope(|scope| {
                        scope
                            .spawn(|| {
                                let theirs = open(path, file, &authority);
                                let verdict = theirs
                                    .validate(other.id, RevisionNumber::SEED, || clock.tick())
                                    .unwrap();
                                assert!(
                                    matches!(verdict, ValidationCommandResult::Validated(_)),
                                    "{at}: the other writer: {verdict:?}"
                                );
                            })
                            .join()
                            .unwrap();
                    });
                    clock.tick()
                })
                .unwrap();
            assert!(
                matches!(verdict, ValidationCommandResult::Validated(_)),
                "{at}: {verdict:?}"
            );
            let committed = mine
                .commit(first.id, context().operator, || clock.tick())
                .unwrap();
            let CommitCommandResult::Committed(receipt) = committed else {
                panic!("{at}: {committed:?}");
            };
            replays(path, file, &authority, &[receipt.result], &at);
        }
    }
}

/// A session of `commits` commits, each validated against the head.
fn committed_session(
    path: &Path,
    file: bool,
    authority: &AuthorityStateV1,
    w: &World,
    clock: &Clock,
    commits: u64,
) -> Runtime {
    let session = open(path, file, authority);
    session.seed(w.document.clone(), || clock.tick()).unwrap();
    for number in 0..commits {
        let tx = w.filler(TransactionId::mint());
        session
            .propose(&encode(&tx), context().operator, || clock.tick())
            .unwrap();
        let verdict = session
            .validate(tx.id, RevisionNumber::new(number), || clock.tick())
            .unwrap();
        assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
        let committed = session
            .commit(tx.id, context().operator, || clock.tick())
            .unwrap();
        assert!(matches!(committed, CommitCommandResult::Committed(_)));
    }
    session
}
/// Proposes a filler and validates it against `against`: the candidate views and edge indexes
/// the validate command alone built.
fn validate_counted(session: &Runtime, w: &World, clock: &Clock, against: u64) -> (u64, u64) {
    let tx = w.filler(TransactionId::mint());
    session
        .propose(&encode(&tx), context().operator, || clock.tick())
        .unwrap();
    let (views, indexes) = (candidates_built(), edge_indexes_built());
    let verdict = session
        .validate(tx.id, RevisionNumber::new(against), || clock.tick())
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    (candidates_built() - views, edge_indexes_built() - indexes)
}

/// A revision's edge index is released with its graph. Revision 7 of an eight-commit session no
/// longer holds its graph, so a validation against it reconstructs the graph, and every candidate
/// view that command builds reads a graph whose index it builds: one index per view. An index kept
/// with a revision that holds no graph would give the decision's view an index it did not build.
#[test]
fn a_revisions_edge_index_is_released_with_its_graph() {
    for (profile, authority) in profiles() {
        for file in [true, false] {
            let at = how(profile, file);
            let directory = tempfile::tempdir().unwrap();
            let clock = Clock::new();
            let w = world();
            let session = committed_session(directory.path(), file, &authority, &w, &clock, 8);
            let (views, indexes) = validate_counted(&session, &w, &clock, 7);
            assert!(views >= 2, "{at}: revision 7 was reconstructed: {views}");
            assert_eq!(indexes, views, "{at}: one index per view");
        }
    }
}

/// The unit's acceptance, "one candidate view per `validate` command", and its changelog entry,
/// "a validate command builds one candidate view", for `validate --against` an older revision
/// whose graph the session no longer holds. The command reconstructs that graph by a replay from
/// the seed that revalidates every retained validation on the way, then decides: against revision
/// 7 of an eight-commit session, one command builds nine candidate views and nine edge indexes,
/// and the count grows with the history. The edge index is built once per revision only while the
/// revision holds its graph.
#[test]
#[ignore = "rebuilding a released revision revalidates its history; \
            task:rebuilt-revision-reuses-retained-verdicts"]
fn a_validate_command_against_an_older_released_revision_builds_one_candidate_view() {
    for (profile, authority) in profiles() {
        for file in [true, false] {
            let at = how(profile, file);
            let directory = tempfile::tempdir().unwrap();
            let clock = Clock::new();
            let w = world();
            let session = committed_session(directory.path(), file, &authority, &w, &clock, 8);
            let (views, indexes) = validate_counted(&session, &w, &clock, 7);
            assert_eq!((views, indexes), (1, 1), "{at}: against revision 7 of 8");
        }
    }
}

/// The acceptance again, under contention the design names as ordinary (§ 91.6: unrelated
/// proposal records permit a bounded retry of the same occurrence). Another writer proposes an
/// unrelated transaction between this handle's decision and its publication; the first admission
/// attempt takes the decided verdict and is refused as a conflict, and the retry's admitting
/// replay validates the same document against the same revision again: two candidate views.
#[test]
fn a_validate_command_retried_after_an_unrelated_proposal_builds_one_candidate_view() {
    for (profile, authority) in profiles() {
        for file in [true, false] {
            let at = how(profile, file);
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let clock = Clock::new();
            let w = world();
            let mine = open(path, file, &authority);
            mine.seed(w.document.clone(), || clock.tick()).unwrap();
            let (first, unrelated) = (
                w.filler(TransactionId::mint()),
                w.filler(TransactionId::mint()),
            );
            mine.propose(&encode(&first), context().operator, || clock.tick())
                .unwrap();
            let views = candidates_built();
            let verdict = mine
                .validate(first.id, RevisionNumber::SEED, || {
                    std::thread::scope(|scope| {
                        scope
                            .spawn(|| {
                                open(path, file, &authority)
                                    .propose(&encode(&unrelated), context().operator, || {
                                        clock.tick()
                                    })
                                    .unwrap();
                            })
                            .join()
                            .unwrap();
                    });
                    clock.tick()
                })
                .unwrap();
            assert!(
                matches!(verdict, ValidationCommandResult::Validated(_)),
                "{at}: {verdict:?}"
            );
            assert_eq!(candidates_built() - views, 1, "{at}: candidate views");
        }
    }
}
