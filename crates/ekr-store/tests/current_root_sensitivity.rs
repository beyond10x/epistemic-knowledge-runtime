//! Field-complete sensitivity of the two sub-roots this crate computes (§ 91.2): every field of
//! every node, edge and assertion reaches `knowledge_root`; every field of every evidence record
//! reaches `evidence_root`; and nothing either function does not read moves it.
//!
//! That the kernel's revision roots are these two functions over the admitted graph is held by
//! `crates/ekr-kernel/tests/current_root_sensitivity.rs`, on both providers.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, IssueId, PropertyId,
    RevisionNumber, SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue,
    Confidence, Edge, Evidence, EvidenceSource, GraphRoot, Node, Object, Predicate, Space, Subject,
    TemporalRange, TransactionTime,
};
use ekr_ontology::{NodeType, Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{evidence_root, knowledge_root};

fn id<T: std::str::FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

const NODE: u64 = 0x10;
const EDGE: u64 = 0x12;
const ASSERTION: u64 = 0x14;
const EVIDENCE: u64 = 0x13;

fn graph() -> CanonicalGraph {
    let (root, kind): (GraphRootId, TypeId) = (id(0x02), id(0x05));
    let mut node = Node::new(id(NODE), root, kind, "first");
    node.aliases = vec!["one".into()];
    node.type_state = Some("open".into());
    node.properties = BTreeMap::from([(
        id::<PropertyId>(0x06),
        vec![CanonicalValue::String("alpha".into())],
    )]);
    let second = Node::new(id(0x11), root, kind, "second");
    let mut edge = Edge::new(
        id(EDGE),
        root,
        id(0x07),
        CanonicalRef::new(node.id),
        CanonicalRef::new(second.id),
    );
    edge.properties = BTreeMap::from([(id::<PropertyId>(0x08), vec![CanonicalValue::Integer(1)])]);
    let evidence = Evidence {
        id: id(EVIDENCE),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: ContentHash::of_bytes(b"synthetic statement"),
        extracted_by: id(0x03),
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::CERTAIN,
    };
    let assertion = Assertion {
        id: id(ASSERTION),
        root_id: root,
        subject: Subject::Node(CanonicalRef::new(node.id)),
        predicate: Predicate::Property(id(0x06)),
        object: Object::Value(CanonicalValue::String("alpha".into())),
        evidence: BTreeSet::from([CanonicalRef::new(evidence.id)]),
        proposed_by: id(0x03),
        assessment: Assessment::Accepted {
            validators: BTreeSet::from([id::<AgentId>(0x04)]),
        },
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::from_millis(1)),
        transaction_time: TransactionTime::since(Timestamp::from_millis(10)),
    };
    CanonicalGraph {
        root: GraphRoot {
            id: root,
            space: Space::Canonical,
            schema_version_id: id(0x01),
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::SEED,
        ontology: Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(id(0x01), Timestamp::EPOCH),
            node_types: vec![NodeType::new(kind, "Subject")],
            edge_types: Vec::new(),
        })
        .unwrap(),
        nodes: BTreeMap::from([(node.id, node), (second.id, second)]),
        edges: BTreeMap::from([(edge.id, edge)]),
        assertions: BTreeMap::from([(assertion.id, assertion)]),
        evidence: BTreeMap::from([(evidence.id, evidence)]),
    }
}

type Change = Box<dyn Fn(&mut CanonicalGraph)>;

fn node(g: &mut CanonicalGraph) -> &mut Node {
    g.nodes.get_mut(&id(NODE)).unwrap()
}
fn edge(g: &mut CanonicalGraph) -> &mut Edge {
    g.edges.get_mut(&id(EDGE)).unwrap()
}
fn assertion(g: &mut CanonicalGraph) -> &mut Assertion {
    g.assertions.get_mut(&id(ASSERTION)).unwrap()
}
fn evidence(g: &mut CanonicalGraph) -> &mut Evidence {
    g.evidence.get_mut(&id(EVIDENCE)).unwrap()
}

/// Every node, edge and assertion field, and each collection's membership.
fn knowledge_changes() -> Vec<(&'static str, Change)> {
    vec![
        (
            "nodes: member",
            Box::new(|g| {
                g.nodes.remove(&id(0x11));
            }),
        ),
        ("node.id", Box::new(|g| node(g).id = id(0x7f))),
        ("node.root_id", Box::new(|g| node(g).root_id = id(0x7f))),
        ("node.type_id", Box::new(|g| node(g).type_id = id(0x7f))),
        (
            "node.canonical_name",
            Box::new(|g| node(g).canonical_name.push('x')),
        ),
        ("node.aliases", Box::new(|g| node(g).aliases.clear())),
        ("node.type_state", Box::new(|g| node(g).type_state = None)),
        (
            "node.properties: value",
            Box::new(|g| {
                node(g)
                    .properties
                    .values_mut()
                    .next()
                    .unwrap()
                    .push(CanonicalValue::String("alpha".into()));
            }),
        ),
        (
            "node.properties: key",
            Box::new(|g| {
                let values = node(g).properties.remove(&id(0x06)).unwrap();
                node(g).properties.insert(id(0x7e), values);
            }),
        ),
        ("edges: member", Box::new(|g| g.edges.clear())),
        ("edge.id", Box::new(|g| edge(g).id = id(0x7f))),
        ("edge.root_id", Box::new(|g| edge(g).root_id = id(0x7f))),
        ("edge.type_id", Box::new(|g| edge(g).type_id = id(0x7f))),
        (
            "edge.source",
            Box::new(|g| edge(g).source = CanonicalRef::new(id(0x11))),
        ),
        (
            "edge.target",
            Box::new(|g| edge(g).target = CanonicalRef::new(id(NODE))),
        ),
        ("edge.properties", Box::new(|g| edge(g).properties.clear())),
        ("assertions: member", Box::new(|g| g.assertions.clear())),
        ("assertion.id", Box::new(|g| assertion(g).id = id(0x7f))),
        (
            "assertion.root_id",
            Box::new(|g| assertion(g).root_id = id(0x7f)),
        ),
        (
            "assertion.subject",
            Box::new(|g| {
                assertion(g).subject = Subject::Edge(CanonicalRef::new(id::<EdgeId>(EDGE)))
            }),
        ),
        (
            "assertion.predicate",
            Box::new(|g| assertion(g).predicate = Predicate::Relation(id(0x07))),
        ),
        (
            "assertion.object",
            Box::new(|g| assertion(g).object = Object::Type(id(0x05))),
        ),
        (
            "assertion.evidence",
            Box::new(|g| assertion(g).evidence.clear()),
        ),
        (
            "assertion.proposed_by",
            Box::new(|g| assertion(g).proposed_by = id(0x7f)),
        ),
        (
            "assertion.assessment: kind",
            Box::new(|g| {
                assertion(g).assessment = Assessment::Rejected {
                    issues: vec![id::<IssueId>(0x7d)],
                }
            }),
        ),
        (
            "assertion.assessment: validators",
            Box::new(|g| {
                assertion(g).assessment = Assessment::Accepted {
                    validators: BTreeSet::from([id::<AgentId>(0x7c)]),
                }
            }),
        ),
        (
            "assertion.lifecycle",
            Box::new(|g| {
                assertion(g).lifecycle = AssertionLifecycle::Superseded {
                    by: CanonicalRef::new(id::<AssertionId>(0x7b)),
                    at_revision: RevisionNumber::new(1),
                    effective_from: Timestamp::from_millis(5),
                }
            }),
        ),
        (
            "assertion.valid_time",
            Box::new(|g| assertion(g).valid_time.to = Some(Timestamp::from_millis(5))),
        ),
        (
            "assertion.transaction_time",
            Box::new(|g| {
                assertion(g).transaction_time.recorded_to = Some(Timestamp::from_millis(20))
            }),
        ),
    ]
}

/// Every evidence field and the collection's membership.
fn evidence_changes() -> Vec<(&'static str, Change)> {
    vec![
        ("evidence: member", Box::new(|g| g.evidence.clear())),
        (
            "evidence.id",
            Box::new(|g| evidence(g).id = id::<EvidenceId>(0x7f)),
        ),
        (
            "evidence.source",
            Box::new(|g| {
                evidence(g).source = EvidenceSource::HumanStatement { identity: None };
            }),
        ),
        (
            "evidence.content_hash",
            Box::new(|g| evidence(g).content_hash = ContentHash::of_bytes(b"other")),
        ),
        (
            "evidence.extracted_by",
            Box::new(|g| evidence(g).extracted_by = id(0x7f)),
        ),
        (
            "evidence.observed_at",
            Box::new(|g| evidence(g).observed_at = Timestamp::from_millis(1)),
        ),
        (
            "evidence.confidence",
            Box::new(|g| evidence(g).confidence = Confidence::from_basis_points(1).unwrap()),
        ),
    ]
}

/// Fields of the graph neither root function reads.
fn neither_changes() -> Vec<(&'static str, Change)> {
    vec![
        ("root.id", Box::new(|g| g.root.id = id::<GraphRootId>(0x7f))),
        ("root.space", Box::new(|g| g.root.space = Space::Transient)),
        (
            "root.schema_version_id",
            Box::new(|g| g.root.schema_version_id = id::<SchemaVersionId>(0x7f)),
        ),
        ("root.parent", Box::new(|g| g.root.parent = Some(id(0x7f)))),
        (
            "root.created_at",
            Box::new(|g| g.root.created_at = Timestamp::from_millis(1)),
        ),
        (
            "revision",
            Box::new(|g| g.revision = RevisionNumber::new(9)),
        ),
        (
            "ontology",
            Box::new(|g| {
                g.ontology = Ontology::load(OntologyDocument {
                    version: SchemaVersion::seed(id(0x7f), Timestamp::EPOCH),
                    node_types: Vec::new(),
                    edge_types: Vec::new(),
                })
                .unwrap();
            }),
        ),
    ]
}

/// For each change: which of the two roots moved.
fn moved(changes: Vec<(&'static str, Change)>) -> Vec<(&'static str, bool, bool)> {
    let base = graph();
    let (knowledge, evidence) = (knowledge_root(&base), evidence_root(&base));
    changes
        .into_iter()
        .map(|(name, change)| {
            let mut changed = graph();
            change(&mut changed);
            (
                name,
                knowledge_root(&changed) != knowledge,
                evidence_root(&changed) != evidence,
            )
        })
        .collect()
}

#[test]
fn every_node_edge_and_assertion_field_reaches_knowledge_root_and_not_evidence_root() {
    let wrong: Vec<_> = moved(knowledge_changes())
        .into_iter()
        .filter(|(_, knowledge, evidence)| !knowledge || *evidence)
        .collect();
    assert_eq!(wrong, Vec::new());
}

#[test]
fn every_evidence_field_reaches_evidence_root_and_not_knowledge_root() {
    let wrong: Vec<_> = moved(evidence_changes())
        .into_iter()
        .filter(|(_, knowledge, evidence)| *knowledge || !evidence)
        .collect();
    assert_eq!(wrong, Vec::new());
}

#[test]
fn graph_fields_neither_root_reads_move_neither_root() {
    let wrong: Vec<_> = moved(neither_changes())
        .into_iter()
        .filter(|(_, knowledge, evidence)| *knowledge || *evidence)
        .collect();
    assert_eq!(wrong, Vec::new());
}

#[test]
fn the_two_roots_are_distinct_value_addresses() {
    let base = graph();
    assert_ne!(knowledge_root(&base), evidence_root(&base));
    assert_eq!(evidence_root(&base), ContentHash::of(&base.evidence));
    let empty = CanonicalGraph {
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
        ..base
    };
    assert_ne!(knowledge_root(&empty), evidence_root(&empty));
}
