//! Adversary, wave sdk-03, unit P: `task:commit-applies-once` with
//! `task:candidate-built-once-per-validation`.
//!
//! The unit hands the graph the commit decision applied to the replay that admits the
//! publication, keyed as the remembered root is, and builds one candidate view per validation.
//! These cases drive both through the paths the unit's own tests do not reach: every operation
//! kind the application handles (schema changes, `WidenEdgeType`, `AddAlias`, `AddEvidence`,
//! retraction) under profiles v1 to v3, a commit crossing the replay checkpoint and one from a
//! handle restored from it, a commit another writer overtakes between its read and its decision,
//! and a commit of a validation against an older revision. Each published root is held against a
//! full replay from the seed.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::validate::candidates_built;
use ekr_kernel::*;
use ekr_ontology::{EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TENANT: &str = "adversary-sdk03-p";

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

/// A monotonic clock shared by every handle of one case.
struct Clock(Cell<i64>);
impl Clock {
    fn new() -> Self {
        Self(Cell::new(1_000))
    }
    fn tick(&self) -> Timestamp {
        let now = self.0.get() + 1;
        self.0.set(now);
        Timestamp::from_millis(now)
    }
}

/// Two node types, `Subject` (with `label`) and `Other`; `REL` from `Subject` to `Subject`; one
/// subject node and one retained evidence entry.
struct World {
    document: SeedDocument,
    subject: TypeId,
    other: TypeId,
    rel: TypeId,
    label: PropertyId,
    evidence: EvidenceId,
    held: NodeId,
}
fn human_evidence(id: EvidenceId, payload: &[u8]) -> Evidence {
    Evidence {
        id,
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: ContentHash::of_bytes(payload),
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(5),
        confidence: Confidence::CERTAIN,
    }
}
fn world() -> World {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let (subject, other, rel, label) = (
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
    document
        .ontology
        .node_types
        .push(NodeType::new(other, "Other"));
    let mut edge_type = EdgeType::new(rel, "REL");
    edge_type.source_types = BTreeSet::from([subject]);
    edge_type.target_types = BTreeSet::from([subject]);
    edge_type.cardinality = ekr_ontology::Cardinality::Many;
    document.ontology.edge_types.push(edge_type);
    let node = Node::<Value>::new(NodeId::mint(), document.graph.root.id, subject, "held");
    let held = node.id;
    document.graph.nodes.insert(held, node);
    let bytes = b"seeded statement".to_vec();
    let entry = human_evidence(EvidenceId::mint(), &bytes);
    let evidence = entry.id;
    document.graph.evidence.insert(evidence, entry.clone());
    document
        .evidence_payloads
        .insert(entry.content_hash, bytes.into());
    World {
        document,
        subject,
        other,
        rel,
        label,
        evidence,
        held,
    }
}
impl World {
    fn root(&self) -> GraphRootId {
        self.document.graph.root.id
    }
    fn assertion(&self, id: AssertionId, node: NodeId, evidence: EvidenceId) -> GraphOperation {
        GraphOperation::AddAssertion(Box::new(Assertion {
            id,
            root_id: self.root(),
            subject: Subject::Node(node),
            predicate: Predicate::Property(self.label),
            object: Object::Value(Value::String(format!("claim {id}"))),
            evidence: BTreeSet::from([evidence]),
            proposed_by: context().operator,
            assessment: Assessment::Proposed,
            lifecycle: AssertionLifecycle::Active,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        }))
    }
    fn node(&self, id: NodeId, type_id: TypeId, aliases: Vec<String>) -> GraphOperation {
        GraphOperation::CreateNode(NodeDraft {
            id,
            root_id: self.root(),
            type_id,
            canonical_name: format!("node {id}"),
            properties: BTreeMap::new(),
            aliases,
        })
    }
    fn edge(&self, source: NodeId, target: NodeId) -> GraphOperation {
        GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id: self.root(),
            type_id: self.rel,
            source,
            target,
            properties: BTreeMap::new(),
        })
    }
    /// A new subject with an evidenced label: an ordinary data transaction.
    fn filler(&self) -> Vec<GraphOperation> {
        let node = NodeId::mint();
        vec![
            self.node(node, self.subject, Vec::new()),
            self.assertion(AssertionId::mint(), node, self.evidence),
        ]
    }
}
fn transaction(
    operations: Vec<GraphOperation>,
    schema_version: Option<SchemaVersionId>,
) -> GraphTransaction {
    let mut evidence = BTreeSet::new();
    for operation in &operations {
        if let GraphOperation::AddAssertion(assertion) = operation {
            evidence.extend(assertion.evidence.iter().copied());
        }
    }
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence,
        schema_version,
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
/// Proposes `tx` and validates it against `against` (the head when `None`).
fn validated(
    runtime: &Runtime,
    clock: &Clock,
    tx: &GraphTransaction,
    against: Option<u64>,
) -> ValidationCommandResult {
    runtime
        .propose(&encode(tx), context().operator, || clock.tick())
        .unwrap();
    let against = against.map_or_else(
        || runtime.head().unwrap().unwrap().revision,
        RevisionNumber::new,
    );
    runtime.validate(tx.id, against, || clock.tick()).unwrap()
}
/// Commits `id` and returns the result with the head graphs the command cloned and applied.
fn commit_counted(
    runtime: &Runtime,
    clock: &Clock,
    id: TransactionId,
) -> (CommitCommandResult, u64) {
    let before = graphs_applied();
    let result = runtime
        .commit(id, context().operator, || clock.tick())
        .unwrap();
    (result, graphs_applied() - before)
}
/// Proposes, validates against the head and commits `tx`, which every step must admit; returns
/// the published root and the count.
fn submit(runtime: &Runtime, clock: &Clock, tx: &GraphTransaction, at: &str) -> (Root, u64) {
    let verdict = validated(runtime, clock, tx, None);
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{at}: {verdict:?}"
    );
    let (result, applied) = commit_counted(runtime, clock, tx.id);
    let CommitCommandResult::Committed(receipt) = result else {
        panic!("{at}: {result:?}");
    };
    (receipt.result, applied)
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
    // A checkpointed open agrees with the full replay.
    assert_eq!(
        open(path, file, authority).read(None).unwrap().root,
        full.read(None).unwrap().root,
        "{at}: checkpointed and full reads"
    );
}

/// Every operation kind the application handles commits with one clone-and-apply of the head
/// graph, under every profile that admits it, across the replay checkpoint and from a handle
/// restored from it; and every root replays.
#[test]
fn every_operation_kind_commits_with_one_apply_and_every_root_replays() {
    for (profile, authority) in profiles() {
        let evolving = profile != "v1";
        for file in [true, false] {
            let at = how(profile, file);
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let clock = Clock::new();
            let w = world();
            let session = open(path, file, &authority);
            session.seed(w.document.clone(), || clock.tick()).unwrap();

            let first = NodeId::mint();
            let claim = AssertionId::mint();
            let payload = b"a statement the transaction brings".to_vec();
            let brought = human_evidence(EvidenceId::mint(), &payload);
            let mut steps: Vec<(&str, GraphTransaction)> = vec![
                (
                    "create with aliases and assert",
                    transaction(
                        vec![
                            w.node(first, w.subject, vec!["first".into()]),
                            w.assertion(claim, first, w.evidence),
                        ],
                        None,
                    ),
                ),
                (
                    "add evidence and cite it",
                    transaction(
                        vec![
                            GraphOperation::AddEvidence(Box::new(EvidenceAddition {
                                evidence: brought.clone(),
                                payload: payload.clone(),
                            })),
                            w.assertion(AssertionId::mint(), first, brought.id),
                        ],
                        None,
                    ),
                ),
                (
                    "add an alias",
                    transaction(
                        vec![GraphOperation::AddAlias(AliasAddition {
                            node: first,
                            alias: "premier".into(),
                        })],
                        None,
                    ),
                ),
                (
                    "update, relate and retract",
                    transaction(
                        vec![
                            GraphOperation::UpdateProperty(PropertyMutation {
                                node: first,
                                property: w.label,
                                values: vec![Value::String("relabelled".into())],
                            }),
                            w.edge(first, w.held),
                            GraphOperation::RetractAssertion(Retraction {
                                assertion: claim,
                                reason: RetractionReason::new("withdrawn"),
                            }),
                        ],
                        None,
                    ),
                ),
            ];
            let other = NodeId::mint();
            if evolving {
                let summary = PropertyId::mint();
                let mut observation = NodeType::new(TypeId::mint(), "Observation");
                observation.properties.insert(
                    summary,
                    PropertyDefinition::new(summary, "summary", ValueType::String),
                );
                let note = PropertyId::mint();
                steps.extend([
                    (
                        "define a node type",
                        transaction(
                            vec![GraphOperation::DefineNodeType(Box::new(observation))],
                            Some(SchemaVersionId::mint()),
                        ),
                    ),
                    (
                        "widen an edge type",
                        transaction(
                            vec![GraphOperation::WidenEdgeType(EdgeWidening {
                                edge_type: w.rel,
                                source_types: BTreeSet::from([w.subject]),
                                target_types: BTreeSet::from([w.subject, w.other]),
                            })],
                            Some(SchemaVersionId::mint()),
                        ),
                    ),
                    (
                        "relate across the widened type",
                        transaction(
                            vec![
                                w.node(other, w.other, vec!["other".into()]),
                                w.edge(first, other),
                            ],
                            None,
                        ),
                    ),
                    (
                        "add a property",
                        transaction(
                            vec![GraphOperation::ModifyProperty(PropertyModification {
                                owner: Some(w.subject),
                                property: PropertyDefinition::new(note, "note", ValueType::String),
                            })],
                            Some(SchemaVersionId::mint()),
                        ),
                    ),
                ]);
            }
            // Past the checkpoint cadence, whatever the profile admitted above.
            while steps.len() < 7 {
                steps.push(("filler", transaction(w.filler(), None)));
            }

            let mut published = Vec::new();
            for (what, tx) in &steps {
                let (root, applied) = submit(&session, &clock, tx, &format!("{at}: {what}"));
                assert_eq!(applied, 1, "{at}: {what}");
                published.push(root);
            }
            // A handle restored from the checkpoint the session wrote.
            let restored = open(path, file, &authority);
            restored.read(None).unwrap();
            let (root, applied) = submit(
                &restored,
                &clock,
                &transaction(w.filler(), None),
                &format!("{at}: restored"),
            );
            assert_eq!(applied, 1, "{at}: restored");
            published.push(root);
            replays(path, file, &authority, &published, &at);
        }
    }
}

/// Another writer commits between this handle's read of the state and its decision: the decision
/// applies the transaction to a head that is no longer the head, the publication is decided again
/// as stale, and the graph the first decision left is taken by nothing. The handle's next commit
/// applies once, and every root replays.
#[test]
fn a_commit_overtaken_between_its_read_and_its_decision_is_stale_and_later_commits_apply_once() {
    for (profile, authority) in profiles() {
        for file in [true, false] {
            let at = how(profile, file);
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let clock = Clock::new();
            let w = world();
            let mine = open(path, file, &authority);
            mine.seed(w.document.clone(), || clock.tick()).unwrap();
            let theirs = open(path, file, &authority);
            let (first, _) = submit(&mine, &clock, &transaction(w.filler(), None), &at);

            let overtaken = transaction(w.filler(), None);
            let winner = transaction(w.filler(), None);
            for tx in [&overtaken, &winner] {
                assert!(
                    matches!(
                        validated(&mine, &clock, tx, None),
                        ValidationCommandResult::Validated(_)
                    ),
                    "{at}"
                );
            }
            theirs.read(None).unwrap();
            let mut won = None;
            let (result, applied) = {
                let before = graphs_applied();
                let result = mine
                    .commit(overtaken.id, context().operator, || {
                        let (result, _) = commit_counted(&theirs, &clock, winner.id);
                        let CommitCommandResult::Committed(receipt) = result else {
                            panic!("{at}: the other writer's commit: {result:?}");
                        };
                        won = Some(receipt.result);
                        clock.tick()
                    })
                    .unwrap();
                (result, graphs_applied() - before)
            };
            assert!(
                matches!(result, CommitCommandResult::Stale(_)),
                "{at}: {result:?}"
            );
            // The other writer's commit ran inside this one's clock; this command's own
            // decision is the second apply and nothing admits it.
            assert!(applied >= 2, "{at}: {applied}");

            let (last, applied) = submit(&mine, &clock, &transaction(w.filler(), None), &at);
            assert_eq!(applied, 1, "{at}: the next commit");
            let published = [first, won.unwrap(), last];
            replays(path, file, &authority, &published, &at);
        }
    }
}

/// A validation against an older revision (`validate --against`) commits as stale, and neither
/// it nor the checkpoint the session crossed changes what the next commits apply or publish.
#[test]
fn a_validation_against_an_older_revision_commits_stale_and_the_next_commits_apply_once() {
    for (profile, authority) in profiles() {
        for file in [true, false] {
            let at = how(profile, file);
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let clock = Clock::new();
            let w = world();
            let session = open(path, file, &authority);
            session.seed(w.document.clone(), || clock.tick()).unwrap();
            let mut published: Vec<Root> = (0..6)
                .map(|_| submit(&session, &clock, &transaction(w.filler(), None), &at).0)
                .collect();
            // Validated at the head first; then a validation against revision 3 puts a state
            // holding revision 3's graph in the handle's cache before the head one commits.
            let current = transaction(w.filler(), None);
            assert!(
                matches!(
                    validated(&session, &clock, &current, None),
                    ValidationCommandResult::Validated(_)
                ),
                "{at}"
            );
            let old = transaction(w.filler(), None);
            let verdict = validated(&session, &clock, &old, Some(3));
            assert!(
                matches!(verdict, ValidationCommandResult::Validated(_)),
                "{at}: {verdict:?}"
            );
            let (result, applied) = commit_counted(&session, &clock, current.id);
            let CommitCommandResult::Committed(receipt) = result else {
                panic!("{at}: {result:?}");
            };
            assert_eq!(
                applied, 1,
                "{at}: a commit after a validation against revision 3"
            );
            published.push(receipt.result);
            let (result, _) = commit_counted(&session, &clock, old.id);
            assert!(
                matches!(result, CommitCommandResult::Stale(_)),
                "{at}: {result:?}"
            );
            for _ in 0..2 {
                let (root, applied) = submit(&session, &clock, &transaction(w.filler(), None), &at);
                assert_eq!(applied, 1, "{at}: after the stale commit");
                published.push(root);
            }
            replays(path, file, &authority, &published, &at);
        }
    }
}

/// The validate command validates the proposal when it decides the publication and again in the
/// replay that admits it, so it builds a candidate view twice, where the task's context names the
/// session `validate` command's cost; a commit from a handle restored from a checkpoint, whose
/// state holds no sealed validation, does the same.
#[test]
#[ignore = "finding (pre-existing, outside the unit's literal acceptance): a validate command \
            runs the pipeline twice, once to decide and once in the admitting replay"]
fn a_validate_command_builds_its_candidate_view_once() {
    let (_, authority) = &profiles()[0];
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let clock = Clock::new();
        let w = world();
        let session = open(directory.path(), file, authority);
        session.seed(w.document.clone(), || clock.tick()).unwrap();
        let tx = transaction(w.filler(), None);
        session
            .propose(&encode(&tx), context().operator, || clock.tick())
            .unwrap();
        let before = candidates_built();
        let verdict = session
            .validate(tx.id, RevisionNumber::SEED, || clock.tick())
            .unwrap();
        let built = candidates_built() - before;
        assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
        assert_eq!(
            built, 1,
            "file={file}: candidate views one validate command built"
        );
    }
}
