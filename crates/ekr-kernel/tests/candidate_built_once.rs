//! Validation builds its candidate view once (`task:candidate-built-once-per-validation`).
//!
//! The candidate view is the node and edge index of canonical state with the operation set
//! applied, which every validator that resolves an identity reads. Building it copies every node
//! and edge of the graph, so the pipeline builds it once per validation and hands that one to every
//! validator, whatever the profile and whatever the verdict. The count is
//! [`ekr_kernel::validate::candidates_built`]: every candidate view built on this thread.
//!
//! That the refusals are byte-identical is `adversary_perf_01_validation_scans.rs`'s differential.
use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, EdgeId, GraphRootId, NodeId, PropertyId, RevisionNumber, SchemaVersionId,
    Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    AssertionLifecycle, Assessment, CanonicalGraph, CanonicalValue, GraphRoot, GraphSnapshot, Node,
    Object, Predicate, Space, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::validate::{candidates_built, HeldIdentities};
use ekr_kernel::{GraphOperation, GraphTransaction, NodeDraft, Pipeline, PropertyMutation};
use ekr_ontology::{
    NodeType, Ontology, OntologyDocument, PropertyDefinition, SchemaVersion, Value, ValueType,
};

fn id<T: std::str::FromStr>(kind: u16, n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-{kind:04x}-4000-8000-{n:012x}")
        .parse()
        .unwrap()
}
fn schema() -> SchemaVersionId {
    id(1, 1)
}
fn decision() -> TypeId {
    id(2, 1)
}
fn title() -> PropertyId {
    id(3, 1)
}
fn root() -> GraphRootId {
    id(4, 1)
}
fn agent(n: u64) -> AgentId {
    id(5, n)
}
/// Canonical state with a few decisions in it, so a candidate view has something to copy.
fn graph() -> CanonicalGraph {
    let mut declared = NodeType::new(decision(), "Decision");
    declared.properties.insert(
        title(),
        PropertyDefinition::new(title(), "title", ValueType::String),
    );
    let mut graph = CanonicalGraph {
        root: GraphRoot {
            id: root(),
            space: Space::Canonical,
            schema_version_id: schema(),
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::new(3),
        ontology: Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(schema(), Timestamp::EPOCH),
            node_types: vec![declared],
            edge_types: Vec::new(),
        })
        .unwrap(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };
    for n in 0..8 {
        let node: NodeId = id(6, n);
        let mut held = Node::new(node, root(), decision(), format!("decision {n}"));
        held.properties
            .insert(title(), vec![CanonicalValue::String(format!("title {n}"))]);
        graph.nodes.insert(node, held);
    }
    graph
}
/// A transaction every profile accepts: one new decision and a title change on a held one.
fn accepted() -> GraphTransaction {
    GraphTransaction {
        id: id::<TransactionId>(7, 1),
        proposer: agent(0),
        operations: vec![
            GraphOperation::CreateNode(NodeDraft {
                id: id(6, 100),
                root_id: root(),
                type_id: decision(),
                canonical_name: "a new decision".into(),
                properties: BTreeMap::new(),
                aliases: Vec::new(),
            }),
            GraphOperation::UpdateProperty(PropertyMutation {
                node: id(6, 1),
                property: title(),
                values: vec![Value::String("retitled".into())],
            }),
        ],
        evidence: BTreeSet::new(),
        schema_version: None,
    }
}
/// A transaction every validator that reads the candidate has something to refuse in: a node
/// nothing holds, a type the ontology does not declare, an edge that is not there and an
/// assertion about it.
fn refused() -> GraphTransaction {
    GraphTransaction {
        id: id::<TransactionId>(7, 2),
        proposer: agent(0),
        operations: vec![
            GraphOperation::CreateNode(NodeDraft {
                id: id(6, 200),
                root_id: root(),
                type_id: id(2, 99),
                canonical_name: "undeclared".into(),
                properties: BTreeMap::new(),
                aliases: Vec::new(),
            }),
            GraphOperation::UpdateProperty(PropertyMutation {
                node: id(6, 300),
                property: title(),
                values: vec![Value::Integer(1)],
            }),
            GraphOperation::DeleteEdge(id::<EdgeId>(8, 1)),
            GraphOperation::AddAssertion(Box::new(ekr_graph::Assertion {
                id: id::<AssertionId>(9, 1),
                root_id: root(),
                subject: Subject::Edge(id(8, 1)),
                predicate: Predicate::Property(title()),
                object: Object::Value(Value::String("about an edge".into())),
                evidence: BTreeSet::new(),
                proposed_by: agent(0),
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time: TemporalRange::UNBOUNDED,
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            })),
        ],
        evidence: BTreeSet::new(),
        schema_version: None,
    }
}
fn pipelines() -> Vec<(&'static str, Pipeline)> {
    let lineage: BTreeSet<SchemaVersionId> = [schema()].into_iter().collect();
    vec![
        ("v1", Pipeline::deterministic(agent(1))),
        ("v2", Pipeline::schema_evolving(agent(1), lineage.clone())),
        (
            "v3",
            Pipeline::identity_keeping(agent(1), lineage, HeldIdentities::default()),
        ),
    ]
}

/// The reference validator reads canonical state's assertions about edges through an index by
/// edge, and still refuses each one about an edge that will not be there in the order canonical
/// state holds the assertions — here the reverse of their edges' order, and including assertions
/// about an edge canonical state never held, which the differential's generator does not build.
/// The expectation is the scan the index replaced, over every assertion in id order.
#[test]
fn assertions_about_missing_edges_are_refused_in_the_order_canonical_state_holds_them() {
    let mut graph = graph();
    let (source, target): (NodeId, NodeId) = (id(6, 0), id(6, 1));
    for e in 0..4 {
        let edge: EdgeId = id(8, e);
        graph.edges.insert(
            edge,
            ekr_graph::Edge::new(
                edge,
                root(),
                id(2, 50),
                ekr_graph::CanonicalRef::new(source),
                ekr_graph::CanonicalRef::new(target),
            ),
        );
    }
    // Assertion `a` is about edge `9 - a`, so assertion order and edge order disagree; edges 4 to
    // 9 were never held, and assertions 6 and 7 are about edges 3 and 2, which the transaction
    // deletes.
    for a in 0..10 {
        let assertion: AssertionId = id(9, a);
        graph.assertions.insert(
            assertion,
            ekr_graph::Assertion {
                id: assertion,
                root_id: root(),
                subject: Subject::Edge(ekr_graph::CanonicalRef::new(id(8, (9 - a) % 10))),
                predicate: Predicate::Property(title()),
                object: Object::Value(CanonicalValue::String(format!("claim {a}"))),
                evidence: BTreeSet::new(),
                proposed_by: agent(0),
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time: TemporalRange::UNBOUNDED,
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            },
        );
    }
    let tx = GraphTransaction {
        id: id::<TransactionId>(7, 3),
        proposer: agent(0),
        operations: vec![
            GraphOperation::DeleteEdge(id(8, 3)),
            GraphOperation::DeleteEdge(id(8, 2)),
        ],
        evidence: BTreeSet::new(),
        schema_version: None,
    };
    let deleted: BTreeSet<EdgeId> = [id(8, 3), id(8, 2)].into_iter().collect();
    let expected: Vec<String> = graph
        .assertions
        .values()
        .filter_map(|assertion| match assertion.subject {
            Subject::Edge(edge)
                if deleted.contains(&edge.id()) || !graph.edges.contains_key(&edge.id()) =>
            {
                Some(format!("edge {} is not in the graph", edge.id()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(expected.len(), 8, "{expected:?}");
    let snapshot = GraphSnapshot::of(&graph);
    let refused: Vec<String> =
        ekr_kernel::Validator::validate(&ekr_kernel::Reference, &snapshot, &tx)
            .unwrap_err()
            .into_iter()
            .filter(|issue| issue.code == "unresolved-edge")
            .map(|issue| issue.message)
            .collect();
    assert_eq!(refused, expected);
}

#[test]
fn every_profile_builds_one_candidate_per_validation_whatever_the_verdict() {
    let graph = graph();
    let snapshot = GraphSnapshot::of(&graph);
    for (profile, pipeline) in pipelines() {
        for (tx, accepts) in [(accepted(), true), (refused(), false)] {
            let before = candidates_built();
            let verdict = pipeline.validate(&snapshot, &tx);
            let built = candidates_built() - before;
            assert_eq!(verdict.is_ok(), accepts, "{profile}: {verdict:?}");
            assert_eq!(built, 1, "{profile}, accepts={accepts}");
        }
    }
}
