//! An identity names one record for the life of a store, across commit, checkpoint and replay, on
//! both providers: `task:deleted-edge-id-is-reusable`.
//!
//! The review's reproduction runs end to end: create edge E, delete it, then create E again
//! between other nodes. Under validation profile v3 (`ekr.p3-deterministic/1`) the third
//! transaction, and a node minted over E's id, are refused with `identity-previously-held`; a
//! reopened store continues from its replay checkpoint, which carries the identities every
//! revision held, and holds the rule without replaying from the seed; and a full replay re-derives
//! both refusals. Under v1 and v2 the reuse still commits, as it did before v3 existed, so a store
//! that already holds one keeps reopening. The validator-level cases are in `identity_history.rs`.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{EdgeId, NodeId, RevisionNumber, Timestamp, TransactionId, TypeId};
use ekr_graph::Node;
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EdgeDraft, GraphOperation,
    GraphTransaction, NodeDraft, Runtime, SeedDocument, TransactionState, ValidationCommandResult,
    ValidationProfileV1,
};
use ekr_ontology::{Cardinality, EdgeType, NodeType, Value};
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

fn v3() -> AuthorityStateV1 {
    anchor(ValidationProfileV1::identity_keeping(context().validator))
}

/// Only the v3 profile keeps identities against the whole lineage.
#[test]
fn only_the_v3_profile_keeps_identities() {
    let validator = context().validator;
    assert!(!ValidationProfileV1::deterministic(validator).keeps_identities());
    assert!(!ValidationProfileV1::schema_evolving(validator).keeps_identities());
    assert!(ValidationProfileV1::identity_keeping(validator).keeps_identities());
}

fn open(path: &std::path::Path, file: bool, authority: AuthorityStateV1) -> Runtime {
    if file {
        Runtime::file(path, "test", context(), authority)
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), authority)
    }
    .unwrap()
}

fn open_in_full(path: &std::path::Path, file: bool, authority: AuthorityStateV1) -> Runtime {
    let mut runtime = open(path, file, authority);
    runtime.set_full_replay(true);
    runtime
}

/// A seed with one node type, one many-valued edge type between it, and four nodes.
struct Seeded {
    document: SeedDocument,
    relates: TypeId,
    subject: TypeId,
    nodes: [NodeId; 4],
}

fn seeded() -> Seeded {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let (subject, relates) = (TypeId::mint(), TypeId::mint());
    document
        .ontology
        .node_types
        .push(NodeType::new(subject, "Subject"));
    let mut edge_type = EdgeType::new(relates, "relates");
    edge_type.source_types = [subject].into_iter().collect();
    edge_type.target_types = [subject].into_iter().collect();
    edge_type.cardinality = Cardinality::Many;
    document.ontology.edge_types.push(edge_type);
    let nodes = [
        NodeId::mint(),
        NodeId::mint(),
        NodeId::mint(),
        NodeId::mint(),
    ];
    for (index, id) in nodes.into_iter().enumerate() {
        document.graph.nodes.insert(
            id,
            Node::<Value>::new(id, document.graph.root.id, subject, format!("n{index}")),
        );
    }
    Seeded {
        document,
        relates,
        subject,
        nodes,
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

fn transaction(operations: Vec<GraphOperation>) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: BTreeSet::new(),
        schema_version: None,
    }
}

/// Propose at `at` and validate against `against` at `at + 1`.
fn propose_and_validate(
    kernel: &Runtime,
    tx: &GraphTransaction,
    against: RevisionNumber,
    at: i64,
) -> ValidationCommandResult {
    kernel
        .propose(&encode(tx), context().operator, || {
            Timestamp::from_millis(at)
        })
        .unwrap();
    kernel
        .validate(tx.id, against, || Timestamp::from_millis(at + 1))
        .unwrap()
}

/// Propose, validate against the head, and — if validated — commit, at `at`, `at + 1`, `at + 2`.
fn submit(kernel: &Runtime, tx: &GraphTransaction, at: i64) -> ValidationCommandResult {
    let head = kernel.head().unwrap().unwrap().revision;
    let verdict = propose_and_validate(kernel, tx, head, at);
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

impl Seeded {
    fn edge(&self, id: EdgeId, source: usize, target: usize) -> GraphOperation {
        GraphOperation::CreateEdge(EdgeDraft {
            id,
            root_id: self.document.graph.root.id,
            type_id: self.relates,
            source: self.nodes[source],
            target: self.nodes[target],
            properties: BTreeMap::new(),
        })
    }

    fn node(&self, id: NodeId) -> GraphOperation {
        GraphOperation::CreateNode(NodeDraft {
            id,
            root_id: self.document.graph.root.id,
            type_id: self.subject,
            canonical_name: "minted over an edge's id".into(),
            properties: BTreeMap::new(),
            aliases: Vec::new(),
        })
    }

    /// Revision 1 creates edge `id` from node 0 to node 1, revision 2 deletes it.
    fn create_then_delete(&self, kernel: &Runtime, id: EdgeId) {
        for (at, operations) in [
            (20, vec![self.edge(id, 0, 1)]),
            (30, vec![GraphOperation::DeleteEdge(id)]),
        ] {
            assert!(matches!(
                submit(kernel, &transaction(operations), at),
                ValidationCommandResult::Validated(_)
            ));
        }
    }
}

/// The review's reproduction under v3: the reuse, as an edge and as a node, is refused at
/// validation; a reopened store continues from its checkpoint, not from the seed, and still
/// refuses the reuse; and a full replay re-derives both refusals.
#[test]
fn v3_refuses_a_deleted_edges_id_and_keeps_refusing_after_a_checkpointed_reopen() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let kernel = open(directory.path(), file, v3());
        kernel
            .seed(seed.document.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let reused = EdgeId::mint();
        seed.create_then_delete(&kernel, reused);

        let as_edge = transaction(vec![seed.edge(reused, 2, 3)]);
        assert_eq!(
            rejected_codes(&submit(&kernel, &as_edge, 40)),
            vec!["identity-previously-held"],
            "file: {file}"
        );
        let as_node = transaction(vec![seed.node(NodeId::from_uuid(reused.to_uuid()))]);
        assert_eq!(
            rejected_codes(&submit(&kernel, &as_node, 50)),
            vec!["identity-previously-held"],
            "file: {file}"
        );
        // A seed node's id is held from revision 0, and is refused as an edge's.
        let over_seed_node = transaction(vec![seed.edge(
            EdgeId::from_uuid(seed.nodes[0].to_uuid()),
            2,
            3,
        )]);
        assert_eq!(
            rejected_codes(&submit(&kernel, &over_seed_node, 55)),
            vec!["identity-already-exists"],
            "file: {file}"
        );
        // And a fresh id for the same relationship is an ordinary edge.
        assert!(matches!(
            submit(
                &kernel,
                &transaction(vec![seed.edge(EdgeId::mint(), 2, 3)]),
                60
            ),
            ValidationCommandResult::Validated(_)
        ));

        let head = kernel.head().unwrap();
        let records = kernel.transactions().unwrap();
        for id in [as_edge.id, as_node.id, over_seed_node.id] {
            assert_eq!(records[&id].state(), TransactionState::Rejected);
        }
        drop(kernel);

        let reopened = open(directory.path(), file, v3());
        assert_eq!(reopened.head().unwrap(), head, "file: {file}");
        assert_eq!(reopened.transactions().unwrap(), records, "file: {file}");
        assert!(!reopened.snapshot().unwrap().edges.contains_key(&reused));
        assert_eq!(
            reopened.seed_replays(),
            0,
            "file: {file}: a reopened v3 store continues from its checkpoint"
        );
        // The rule holds on the state the checkpoint restored, still without a seed replay.
        assert_eq!(
            rejected_codes(&submit(
                &reopened,
                &transaction(vec![seed.edge(reused, 3, 2)]),
                70
            )),
            vec!["identity-previously-held"],
            "file: {file}"
        );
        assert_eq!(reopened.seed_replays(), 0, "file: {file}");
        let head = reopened.head().unwrap();
        let records = reopened.transactions().unwrap();
        drop(reopened);

        // A full replay re-derives every refusal, including the one made after the reopen.
        let replayed = open_in_full(directory.path(), file, v3());
        assert_eq!(replayed.head().unwrap(), head, "file: {file}");
        assert_eq!(replayed.transactions().unwrap(), records, "file: {file}");
        assert!(replayed.seed_replays() > 0, "file: {file}");
    }
}

/// Validating against revision N reads the identities held up to N: an id first held after N is
/// not yet held there, and is held at every revision after.
#[test]
fn v3_holds_an_id_from_the_revision_that_first_held_it() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let kernel = open(directory.path(), file, v3());
        kernel
            .seed(seed.document.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let reused = EdgeId::mint();
        seed.create_then_delete(&kernel, reused);

        let at_seed = transaction(vec![seed.edge(reused, 2, 3)]);
        assert!(
            matches!(
                propose_and_validate(&kernel, &at_seed, RevisionNumber::SEED, 40),
                ValidationCommandResult::Validated(_)
            ),
            "file: {file}: the seed revision never held the id"
        );
        for (at, against, code) in [
            (50, 1, "identity-already-exists"),
            (60, 2, "identity-previously-held"),
        ] {
            let tx = transaction(vec![seed.edge(reused, 2, 3)]);
            assert_eq!(
                rejected_codes(&propose_and_validate(
                    &kernel,
                    &tx,
                    RevisionNumber::new(against),
                    at
                )),
                vec![code],
                "file: {file}: against revision {against}"
            );
        }
        let records = kernel.transactions().unwrap();
        drop(kernel);
        let replayed = open_in_full(directory.path(), file, v3());
        assert_eq!(replayed.transactions().unwrap(), records, "file: {file}");
    }
}

/// Under v1 and v2 the same reuse still commits, as it did before v3: one id names two
/// relationships at two revisions, and the store reopens from its checkpoint and in full.
#[test]
fn v1_and_v2_still_admit_the_reuse_and_replay_it_on_both_providers() {
    for (profile, authority) in [("v1", v1 as fn() -> AuthorityStateV1), ("v2", v2)] {
        for file in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let seed = seeded();
            let kernel = open(directory.path(), file, authority());
            kernel
                .seed(seed.document.clone(), || Timestamp::from_millis(10))
                .unwrap();
            let reused = EdgeId::mint();
            seed.create_then_delete(&kernel, reused);
            assert!(matches!(
                submit(&kernel, &transaction(vec![seed.edge(reused, 2, 3)]), 40),
                ValidationCommandResult::Validated(_)
            ));
            let endpoints = |kernel: &Runtime, revision: u64| {
                let graph = kernel.replay(RevisionNumber::new(revision)).unwrap();
                let edge = &graph.edges[&reused];
                (edge.source.id(), edge.target.id())
            };
            assert_eq!(endpoints(&kernel, 1), (seed.nodes[0], seed.nodes[1]));
            assert_eq!(endpoints(&kernel, 3), (seed.nodes[2], seed.nodes[3]));

            let head = kernel.head().unwrap();
            let records = kernel.transactions().unwrap();
            drop(kernel);
            let reopened = open(directory.path(), file, authority());
            assert_eq!(reopened.head().unwrap(), head, "{profile} file: {file}");
            assert_eq!(
                reopened.transactions().unwrap(),
                records,
                "{profile} file: {file}"
            );
            assert_eq!(reopened.seed_replays(), 0, "{profile} file: {file}");
            let replayed = open_in_full(directory.path(), file, authority());
            assert_eq!(replayed.head().unwrap(), head, "{profile} file: {file}");
            assert_eq!(
                replayed.transactions().unwrap(),
                records,
                "{profile} file: {file}"
            );
        }
    }
}
