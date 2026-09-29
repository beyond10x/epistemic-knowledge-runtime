//! `story:edge-type-endpoints-widen`: a schema change widens an edge type's `source_types` and
//! `target_types`, on both providers, under validation profiles v2 and v3.
//!
//! A store that relates `Person` to `Document` by `AUTHORED` learns that people author work items
//! too. Without a widening the only way to record that is a second edge type
//! (`AUTHORED_WORKITEM`), which splits one relation in two. With one, a schema-only transaction
//! makes `AUTHORED` run from `Person` to `Document` or `WorkItem`; the edge the seed holds stays
//! valid, the next transaction creates `Person → WorkItem` edges, and replay reproduces every
//! root. An end is never narrowed, and a widening that names what is not there, removes an end or
//! adds nothing is refused by name.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    ContentHash, EdgeId, EvidenceId, NodeId, RevisionNumber, SchemaVersionId, Timestamp,
    TransactionId, TypeId,
};
use ekr_graph::{CanonicalGraph, Edge, Node, Root};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EdgeDraft, EdgeWidening,
    GraphOperation, GraphTransaction, NodeDraft, Runtime, SeedDocument, ValidationCommandResult,
    ValidationProfileV1, ValidatorName,
};
use ekr_ontology::{EdgeType, NodeType, Value};
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

/// The two profiles that admit schema changes.
fn evolving() -> [(&'static str, AuthorityStateV1); 2] {
    [
        (
            "v2",
            anchor(ValidationProfileV1::schema_evolving(context().validator)),
        ),
        (
            "v3",
            anchor(ValidationProfileV1::identity_keeping(context().validator)),
        ),
    ]
}

fn open(path: &std::path::Path, file: bool, authority: AuthorityStateV1) -> Runtime {
    if file {
        Runtime::file(path, "test", context(), authority)
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), authority)
    }
    .unwrap()
}

/// `Person`, `Document` and `WorkItem`; `AUTHORED` from `Person` to `Document`; two people, a
/// document, two work items; and one `AUTHORED` edge, Ada to the design note.
struct Authored {
    document: SeedDocument,
    person: TypeId,
    document_type: TypeId,
    work_item: TypeId,
    authored: TypeId,
    ada: NodeId,
    grace: NodeId,
    first_item: NodeId,
    second_item: NodeId,
    held_edge: EdgeId,
}

fn authored() -> Authored {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let (person, document_type, work_item, authored) = (
        TypeId::mint(),
        TypeId::mint(),
        TypeId::mint(),
        TypeId::mint(),
    );
    document.ontology.node_types.extend([
        NodeType::new(person, "Person"),
        NodeType::new(document_type, "Document"),
        NodeType::new(work_item, "WorkItem"),
    ]);
    let mut edge_type = EdgeType::new(authored, "AUTHORED");
    edge_type.source_types = BTreeSet::from([person]);
    edge_type.target_types = BTreeSet::from([document_type]);
    edge_type.cardinality = ekr_ontology::Cardinality::Many;
    document.ontology.edge_types.push(edge_type);

    let root = document.graph.root.id;
    let mut node = |type_id: TypeId, name: &str| {
        let node = Node::<Value>::new(NodeId::mint(), root, type_id, name);
        let id = node.id;
        document.graph.nodes.insert(id, node);
        id
    };
    let (ada, grace) = (node(person, "Ada"), node(person, "Grace"));
    let design = node(document_type, "design note");
    let (first_item, second_item) = (
        node(work_item, "first item"),
        node(work_item, "second item"),
    );
    let held = Edge::<Value>::new(EdgeId::mint(), root, authored, ada, design);
    let held_edge = held.id;
    document.graph.edges.insert(held.id, held);
    Authored {
        document,
        person,
        document_type,
        work_item,
        authored,
        ada,
        grace,
        first_item,
        second_item,
        held_edge,
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

fn transaction(
    operations: Vec<GraphOperation>,
    schema_version: Option<SchemaVersionId>,
) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: BTreeSet::<EvidenceId>::new(),
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

/// Each issue of a rejection as `(validator, code)`.
fn refused(verdict: &ValidationCommandResult) -> Vec<(ValidatorName, String)> {
    match verdict {
        ValidationCommandResult::Rejected(record) => record
            .issues
            .iter()
            .map(|issue| (issue.validator, issue.code.clone()))
            .collect(),
        ValidationCommandResult::Validated(_) => panic!("expected a rejection"),
    }
}

fn widen(edge_type: TypeId, source_types: &[TypeId], target_types: &[TypeId]) -> GraphOperation {
    GraphOperation::WidenEdgeType(EdgeWidening {
        edge_type,
        source_types: source_types.iter().copied().collect(),
        target_types: target_types.iter().copied().collect(),
    })
}

fn authored_edge(seed: &Authored, source: NodeId, target: NodeId) -> GraphOperation {
    GraphOperation::CreateEdge(EdgeDraft {
        id: EdgeId::mint(),
        root_id: seed.document.graph.root.id,
        type_id: seed.authored,
        source,
        target,
        properties: BTreeMap::new(),
    })
}

/// Every revision's graph as this runtime replays it, with the root its commit receipt holds.
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

/// The first acceptance statement, and the replay half of the second: AUTHORED widens from
/// Person→Document to Person→{Document, WorkItem} in one schema-only transaction, which is a new
/// schema version whose parent is the seed's; the edge the seed holds is untouched; the next
/// transaction creates Person→WorkItem AUTHORED edges, which the seed's schema refused; and a
/// reopened store replays every root and every version.
#[test]
fn authored_widens_to_work_items_and_the_next_transaction_creates_person_to_work_item_edges() {
    for (profile, authority) in evolving() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            let seed = authored();
            let seed_version = seed.document.ontology.version.id;
            let kernel = open(directory.path(), file, authority.clone());
            kernel
                .seed(seed.document.clone(), || Timestamp::from_millis(10))
                .unwrap();

            // Under the seed's schema a work item is no end of AUTHORED.
            let early = transaction(vec![authored_edge(&seed, seed.ada, seed.first_item)], None);
            assert!(
                refused(&submit(&kernel, &early, 20))
                    .iter()
                    .any(|(_, code)| code == "edge-endpoint-type"),
                "{at}"
            );

            let version = SchemaVersionId::mint();
            let widening = transaction(
                vec![widen(
                    seed.authored,
                    &[seed.person],
                    &[seed.document_type, seed.work_item],
                )],
                Some(version),
            );
            let verdict = submit(&kernel, &widening, 30);
            assert!(
                matches!(verdict, ValidationCommandResult::Validated(_)),
                "{at}: {verdict:?}"
            );
            let widened = kernel.snapshot().unwrap();
            assert_eq!(widened.revision, RevisionNumber::new(1), "{at}");
            let held = widened.ontology.version();
            assert_eq!(
                (held.id, held.number, held.parent),
                (version, 1, Some(seed_version)),
                "{at}"
            );
            let declared = widened.ontology.edge_type(seed.authored).unwrap();
            assert_eq!(declared.source_types, BTreeSet::from([seed.person]), "{at}");
            assert_eq!(
                declared.target_types,
                BTreeSet::from([seed.document_type, seed.work_item]),
                "{at}"
            );
            // The edge the seed holds is untouched, and so is every other record.
            let before = kernel.replay(RevisionNumber::SEED).unwrap();
            assert_eq!(widened.edges, before.edges, "{at}");
            assert!(widened.edges.contains_key(&seed.held_edge), "{at}");
            assert_eq!(widened.nodes, before.nodes, "{at}");

            let later = transaction(
                vec![
                    authored_edge(&seed, seed.ada, seed.first_item),
                    authored_edge(&seed, seed.grace, seed.second_item),
                ],
                None,
            );
            let verdict = submit(&kernel, &later, 40);
            assert!(
                matches!(verdict, ValidationCommandResult::Validated(_)),
                "{at}: {verdict:?}"
            );
            let used = kernel.snapshot().unwrap();
            assert_eq!(used.revision, RevisionNumber::new(2), "{at}");
            assert_eq!(
                used.ontology.version().id,
                version,
                "{at}: data keeps the version"
            );
            let to_work_items = used
                .edges
                .values()
                .filter(|edge| {
                    edge.type_id == seed.authored
                        && used.nodes[&edge.target.id()].type_id == seed.work_item
                })
                .count();
            assert_eq!(to_work_items, 2, "{at}");

            let expected = every_revision(&kernel);
            let expected_head = kernel.head().unwrap();
            let expected_records = kernel.transactions().unwrap();
            assert_eq!(expected.len(), 3, "{at}");
            assert_eq!(expected[0].1.ontology.version().id, seed_version, "{at}");
            assert_eq!(expected[1].1.ontology.version().id, version, "{at}");
            drop(kernel);

            let reopened = open(directory.path(), file, authority.clone());
            assert_eq!(reopened.head().unwrap(), expected_head, "{at}");
            assert_eq!(every_revision(&reopened), expected, "{at}");
            assert_eq!(reopened.transactions().unwrap(), expected_records, "{at}");
        }
    }
}

/// The third acceptance statement: removing an end, naming an unknown type — an edge type or a
/// node type — and a widening that adds nothing are each refused, by `OntologyConstraint`, with
/// one issue under its own code; the head does not move.
#[test]
fn a_removed_end_an_unknown_type_and_a_widening_without_effect_are_each_refused_by_name() {
    for (profile, authority) in evolving() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            let seed = authored();
            let kernel = open(directory.path(), file, authority.clone());
            kernel
                .seed(seed.document.clone(), || Timestamp::from_millis(10))
                .unwrap();
            let nowhere = TypeId::mint();
            for (step, (operation, code)) in [
                (
                    widen(seed.authored, &[seed.person], &[seed.work_item]),
                    "edge-endpoint-removed",
                ),
                (
                    widen(nowhere, &[seed.person], &[seed.document_type]),
                    "unknown-edge-type",
                ),
                (
                    widen(
                        seed.authored,
                        &[seed.person],
                        &[seed.document_type, nowhere],
                    ),
                    "unknown-endpoint-type",
                ),
                (
                    widen(seed.authored, &[seed.person], &[seed.document_type]),
                    "schema-change-without-effect",
                ),
            ]
            .into_iter()
            .enumerate()
            {
                let tx = transaction(vec![operation], Some(SchemaVersionId::mint()));
                let at_ms = 20 + 10 * i64::try_from(step).unwrap();
                assert_eq!(
                    refused(&submit(&kernel, &tx, at_ms)),
                    vec![(ValidatorName::OntologyConstraint, code.to_owned())],
                    "{at}: {code}"
                );
            }
            assert_eq!(
                kernel.head().unwrap().unwrap().revision,
                RevisionNumber::SEED,
                "{at}"
            );
        }
    }
}

/// A widening is a schema change like the other three: it travels alone and names its version,
/// two of one edge type in one unordered transaction compete, and profile v1 refuses it as it
/// refuses the others.
#[test]
fn a_widening_obeys_the_rules_every_schema_change_obeys() {
    let seed = authored();
    let widening = widen(
        seed.authored,
        &[seed.person],
        &[seed.document_type, seed.work_item],
    );
    let (_, v2) = evolving().into_iter().next().unwrap();
    let directory = tempfile::tempdir().unwrap();
    let kernel = open(directory.path(), true, v2);
    kernel
        .seed(seed.document.clone(), || Timestamp::from_millis(10))
        .unwrap();

    let unversioned = transaction(vec![widening.clone()], None);
    assert_eq!(
        refused(&submit(&kernel, &unversioned, 20)),
        vec![(
            ValidatorName::Structural,
            "schema-version-missing".to_owned()
        )]
    );

    let mixed = transaction(
        vec![
            widening.clone(),
            GraphOperation::CreateNode(NodeDraft {
                id: NodeId::mint(),
                root_id: seed.document.graph.root.id,
                type_id: seed.work_item,
                canonical_name: "third item".into(),
                properties: BTreeMap::new(),
                aliases: Vec::new(),
            }),
        ],
        Some(SchemaVersionId::mint()),
    );
    assert!(refused(&submit(&kernel, &mixed, 30)).contains(&(
        ValidatorName::Structural,
        "mixed-schema-transaction".to_owned()
    )));

    let twice = transaction(
        vec![
            widening.clone(),
            widen(
                seed.authored,
                &[seed.person, seed.work_item],
                &[seed.document_type],
            ),
        ],
        Some(SchemaVersionId::mint()),
    );
    assert_eq!(
        refused(&submit(&kernel, &twice, 40)),
        vec![(ValidatorName::Structural, "conflicting-write".to_owned())]
    );

    let v1 = anchor(ValidationProfileV1::deterministic(context().validator));
    let directory = tempfile::tempdir().unwrap();
    let kernel = open(directory.path(), false, v1);
    kernel
        .seed(seed.document.clone(), || Timestamp::from_millis(10))
        .unwrap();
    let versioned = transaction(vec![widening], Some(SchemaVersionId::mint()));
    let verdict = submit(&kernel, &versioned, 20);
    assert_eq!(
        refused(&verdict),
        vec![(
            ValidatorName::Structural,
            "unsupported-operation".to_owned()
        )]
    );
    let ValidationCommandResult::Rejected(record) = verdict else {
        unreachable!()
    };
    assert_eq!(
        record.issues[0].message,
        "WidenEdgeType is not supported in P1"
    );
}
