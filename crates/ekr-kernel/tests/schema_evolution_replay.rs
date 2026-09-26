//! Schema changes across replay, on both providers: `story:schema-evolution-transactions` part C.
//!
//! The second acceptance bullet: existing nodes, edges and assertions stay valid across the change,
//! and replay from the seed reproduces every root across schema versions, on the file and the
//! SQLite provider. Plus the profile rule: a store keeps the profile its anchor names at seed, and a
//! v1 store holding a retained rejection of a schema operation still replays.
//!
//! These run through `Runtime`, which is the kernel's authority over both providers. They sit in
//! `ekr-kernel` and not `ekr-store`: `ekr-store` sits below the kernel and has no dependency on it,
//! so a store test cannot reach the kernel's replay.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, NodeId, PropertyId, RevisionNumber,
    SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, Confidence, Evidence,
    EvidenceSource, Node, Object, Predicate, Root, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EdgeDraft, GraphOperation,
    GraphTransaction, NodeDraft, PropertyModification, PropertyMutation, Runtime, SeedDocument,
    TransactionState, ValidationCommandResult, ValidationProfileV1,
};
use ekr_ontology::{EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;

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

fn v1() -> AuthorityStateV1 {
    anchor(ValidationProfileV1::deterministic(context().validator))
}

fn v2() -> AuthorityStateV1 {
    anchor(ValidationProfileV1::schema_evolving(context().validator))
}

fn open(
    path: &std::path::Path,
    file: bool,
    authority: AuthorityStateV1,
) -> Result<Runtime, ekr_kernel::PersistenceError> {
    if file {
        Runtime::file(path, "test", context(), authority)
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), authority)
    }
}

/// The seed: one `Subject` type with a `label`, one subject node, and one assertion on it.
struct Seeded {
    document: SeedDocument,
    subject_type: TypeId,
    label: PropertyId,
    subject: NodeId,
    evidence: EvidenceId,
}

fn seeded() -> Seeded {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let (subject_type, label) = (TypeId::mint(), PropertyId::mint());
    let mut declared = NodeType::new(subject_type, "Subject");
    declared.properties.insert(
        label,
        PropertyDefinition::new(label, "label", ValueType::String),
    );
    document.ontology.node_types.push(declared);
    let mut node = Node::<Value>::new(NodeId::mint(), document.graph.root.id, subject_type, "seed");
    node.properties
        .insert(label, vec![Value::String("seeded".into())]);
    let bytes = b"synthetic operator statement".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::from_basis_points(10000).unwrap(),
    };
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id: document.graph.root.id,
        subject: Subject::Node(node.id),
        predicate: Predicate::Property(label),
        object: Object::Value(Value::String("seeded".into())),
        evidence: BTreeSet::from([evidence.id]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    let subject = node.id;
    document.graph.nodes.insert(node.id, node);
    document.graph.assertions.insert(assertion.id, assertion);
    document
        .graph
        .evidence
        .insert(evidence.id, evidence.clone());
    document.evidence_payloads.insert(hash, bytes);
    Seeded {
        document,
        subject_type,
        label,
        subject,
        evidence: evidence.id,
    }
}

fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}

fn transaction(
    operations: Vec<GraphOperation>,
    evidence: BTreeSet<EvidenceId>,
    schema_version: Option<SchemaVersionId>,
) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence,
        schema_version,
    }
}

/// Propose, validate against the head, and — if validated — commit, at `at`, `at + 1`, `at + 2`.
fn submit(kernel: &Runtime, tx: &GraphTransaction, at: i64) -> ValidationCommandResult {
    kernel
        .propose(&encode(tx), context().operator, || {
            Timestamp::from_millis(at)
        })
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    let verdict = kernel
        .validate(tx.id, head, || Timestamp::from_millis(at + 1))
        .unwrap();
    if matches!(verdict, ValidationCommandResult::Validated(_)) {
        let committed = kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
            .unwrap();
        assert!(
            matches!(committed, CommitCommandResult::Committed(_)),
            "{committed:?}"
        );
    }
    verdict
}

fn rejected_codes(verdict: &ValidationCommandResult) -> Vec<String> {
    match verdict {
        ValidationCommandResult::Rejected(record) => record
            .issues
            .iter()
            .map(|issue| issue.code.clone())
            .collect(),
        ValidationCommandResult::Validated(_) => panic!("expected a rejection"),
    }
}

/// Every revision's graph as this runtime reconstructs it, with the root its receipt holds; the
/// seed's root is the basis the first validation recorded.
fn every_revision(kernel: &Runtime) -> Vec<(Option<Root>, CanonicalGraph)> {
    let head = kernel.head().unwrap().unwrap();
    let records = kernel.transactions().unwrap();
    (0..=head.revision.get())
        .map(|number| {
            let graph = kernel.replay(RevisionNumber::new(number)).unwrap();
            let root = records
                .values()
                .filter_map(|record| record.committed.as_ref())
                .find(|receipt| receipt.result.revision == graph.revision)
                .map(|receipt| receipt.result);
            if let Some(root) = root {
                assert_eq!(root.ontology_root, ContentHash::of(&graph.ontology));
            }
            (root, graph)
        })
        .collect()
}

/// Seed, schema change, data using the new type, reopen: identical roots on both providers, and
/// the schema version carries lineage.
#[test]
fn replay_reproduces_every_root_across_schema_versions_on_both_providers() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let seed_version = seed.document.ontology.version.id;
        let kernel = open(directory.path(), file, v2()).unwrap();
        let initial = kernel
            .seed(seed.document.clone(), || Timestamp::from_millis(10))
            .unwrap();

        // Revision 1: a node type, an edge type, and a property added to the seed's type.
        let (observation, summary, observes, note) = (
            TypeId::mint(),
            PropertyId::mint(),
            TypeId::mint(),
            PropertyId::mint(),
        );
        let mut observation_type = NodeType::new(observation, "Observation");
        observation_type.properties.insert(
            summary,
            PropertyDefinition::new(summary, "summary", ValueType::String),
        );
        let mut observes_type = EdgeType::new(observes, "observes");
        observes_type.source_types = [observation].into_iter().collect();
        observes_type.target_types = [seed.subject_type].into_iter().collect();
        let version = SchemaVersionId::mint();
        let schema = transaction(
            vec![
                GraphOperation::DefineNodeType(Box::new(observation_type)),
                GraphOperation::DefineEdgeType(Box::new(observes_type)),
                GraphOperation::ModifyProperty(PropertyModification {
                    owner: Some(seed.subject_type),
                    property: PropertyDefinition::new(note, "note", ValueType::String),
                }),
            ],
            BTreeSet::new(),
            Some(version),
        );
        assert!(matches!(
            submit(&kernel, &schema, 20),
            ValidationCommandResult::Validated(_)
        ));
        let after_schema = kernel.snapshot().unwrap();
        let held = after_schema.ontology.version();
        assert_eq!(
            (held.id, held.number, held.parent),
            (version, 1, Some(seed_version))
        );
        assert_eq!(held.created_at, Timestamp::from_millis(22), "commit time");
        assert_eq!(after_schema.root.schema_version_id, seed_version);
        let head = kernel.head().unwrap().unwrap();
        assert_ne!(head.ontology_root, initial.result.ontology_root);
        // Existing nodes and assertions are unchanged by the change.
        let before = kernel.replay(RevisionNumber::SEED).unwrap();
        assert_eq!(after_schema.nodes, before.nodes);
        assert_eq!(after_schema.assertions, before.assertions);

        // Revision 2: data that uses the new types and the new property.
        let observed = NodeId::mint();
        let data = transaction(
            vec![
                GraphOperation::CreateNode(NodeDraft {
                    id: observed,
                    root_id: seed.document.graph.root.id,
                    type_id: observation,
                    canonical_name: "first observation".into(),
                    properties: BTreeMap::from([(
                        summary,
                        vec![Value::String("observed after the schema change".into())],
                    )]),
                }),
                GraphOperation::CreateEdge(EdgeDraft {
                    id: EdgeId::mint(),
                    root_id: seed.document.graph.root.id,
                    type_id: observes,
                    source: observed,
                    target: seed.subject,
                    properties: BTreeMap::new(),
                }),
                GraphOperation::UpdateProperty(PropertyMutation {
                    node: seed.subject,
                    property: note,
                    values: vec![Value::String("annotated".into())],
                }),
            ],
            BTreeSet::new(),
            None,
        );
        assert!(matches!(
            submit(&kernel, &data, 30),
            ValidationCommandResult::Validated(_)
        ));

        // A change canonical state now violates, refused and retained: `note` holds a string.
        let mut integer_note = PropertyDefinition::new(note, "note", ValueType::Integer);
        integer_note.required = false;
        let breaking = transaction(
            vec![GraphOperation::ModifyProperty(PropertyModification {
                owner: Some(seed.subject_type),
                property: integer_note,
            })],
            BTreeSet::new(),
            Some(SchemaVersionId::mint()),
        );
        assert_eq!(
            rejected_codes(&submit(&kernel, &breaking, 40)),
            vec!["value-kind-not-admitted"]
        );

        // Revision 3: a second version, so the seed's id is two versions back.
        let second = SchemaVersionId::mint();
        let widen = transaction(
            vec![GraphOperation::ModifyProperty(PropertyModification {
                owner: Some(observation),
                property: PropertyDefinition::new(PropertyId::mint(), "source", ValueType::String),
            })],
            BTreeSet::new(),
            Some(second),
        );
        assert!(matches!(
            submit(&kernel, &widen, 50),
            ValidationCommandResult::Validated(_)
        ));
        let third = kernel.snapshot().unwrap();
        assert_eq!(
            (
                third.ontology.version().number,
                third.ontology.version().parent
            ),
            (2, Some(version))
        );

        // The seed's id, two versions back, is on the lineage and is refused.
        let reused = transaction(
            vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
                TypeId::mint(),
                "Reuse",
            )))],
            BTreeSet::new(),
            Some(seed_version),
        );
        assert_eq!(
            rejected_codes(&submit(&kernel, &reused, 60)),
            vec!["schema-version-reused"]
        );
        let _ = seed.evidence;
        let _ = seed.label;

        let expected = every_revision(&kernel);
        let expected_head = kernel.head().unwrap();
        let expected_records = kernel.transactions().unwrap();
        assert_eq!(expected.len(), 4);
        drop(kernel);

        let reopened = open(directory.path(), file, v2()).unwrap();
        assert_eq!(reopened.head().unwrap(), expected_head);
        assert_eq!(every_revision(&reopened), expected);
        assert_eq!(reopened.transactions().unwrap(), expected_records);
    }
}

/// A v1 store refuses the three kinds with the retained P1 issue, retains the rejection, and
/// still replays it.
#[test]
fn a_v1_store_with_a_retained_schema_rejection_still_replays_on_both_providers() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let kernel = open(directory.path(), file, v1()).unwrap();
        kernel
            .seed(seed.document.clone(), || Timestamp::from_millis(10))
            .unwrap();
        for (at, version) in [(20, None), (30, Some(SchemaVersionId::mint()))] {
            let refused = transaction(
                vec![GraphOperation::ModifyProperty(PropertyModification {
                    owner: Some(seed.subject_type),
                    property: PropertyDefinition::new(
                        PropertyId::mint(),
                        "note",
                        ValueType::String,
                    ),
                })],
                BTreeSet::new(),
                version,
            );
            let verdict = submit(&kernel, &refused, at);
            assert_eq!(rejected_codes(&verdict), vec!["unsupported-operation"]);
        }
        let head = kernel.head().unwrap();
        let records = kernel.transactions().unwrap();
        assert!(records
            .values()
            .all(|record| record.state() == TransactionState::Rejected));
        drop(kernel);

        let reopened = open(directory.path(), file, v1()).unwrap();
        assert_eq!(reopened.head().unwrap(), head);
        assert_eq!(reopened.transactions().unwrap(), records);
    }
}

/// A store keeps the profile its anchor named at seed: a v1 store is not read under v2, nor a v2
/// store under v1.
#[test]
fn a_store_keeps_the_profile_it_was_seeded_under_on_both_providers() {
    for file in [false, true] {
        for (seeded_under, opened_under) in [(v1(), v2()), (v2(), v1())] {
            let directory = tempfile::tempdir().unwrap();
            let kernel = open(directory.path(), file, seeded_under).unwrap();
            kernel
                .seed(seeded().document, || Timestamp::from_millis(10))
                .unwrap();
            drop(kernel);
            let refused =
                open(directory.path(), file, opened_under).and_then(|kernel| kernel.head());
            assert!(refused.is_err(), "{refused:?}");
        }
    }
}

/// Each profile is one exact pair: a v2 ruleset with the v1 application, or the reverse, is no
/// profile at all.
#[test]
fn a_profile_mixing_the_two_rulesets_is_refused() {
    for (ruleset, application) in [
        ("ekr.p2-deterministic/1", "ekr.p1-apply/1"),
        ("ekr.p1-deterministic/1", "ekr.p2-apply/1"),
    ] {
        let mut profile = ValidationProfileV1::schema_evolving(context().validator);
        profile.ruleset = ruleset.into();
        profile.application = application.into();
        let directory = tempfile::tempdir().unwrap();
        assert!(open(directory.path(), true, anchor(profile)).is_err());
    }
    let v2 = ValidationProfileV1::schema_evolving(context().validator);
    assert_eq!(
        (v2.ruleset.as_str(), v2.application.as_str()),
        ("ekr.p2-deterministic/1", "ekr.p2-apply/1")
    );
    let _: AgentId = v2.validator;
}
