//! Read-only conflict analysis; ordinary replay continues to use its recorded rules.
use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionNumber, SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue,
    Confidence, Edge, Evidence, EvidenceSource, GraphRoot, Node, Object, Predicate,
    RetractionReason, Space, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::disputes::preview_contradictions;
use ekr_ontology::{
    Cardinality, EdgeType, NodeType, Ontology, OntologyDocument, PropertyDefinition, SchemaVersion,
    ValueType,
};
use std::collections::BTreeSet;

struct Fixture {
    graph: CanonicalGraph,
    nodes: [NodeId; 3],
    edge: EdgeId,
    property: PropertyId,
    reference: PropertyId,
    relation: TypeId,
    proposer: AgentId,
    validator: AgentId,
    evidence: EvidenceId,
}
impl Fixture {
    fn new(cardinality: Cardinality) -> Self {
        let root = GraphRootId::mint();
        let schema = SchemaVersionId::mint();
        let parent = TypeId::mint();
        let child = TypeId::mint();
        let relation = TypeId::mint();
        let property = PropertyId::mint();
        let reference = PropertyId::mint();
        let nodes = [NodeId::mint(), NodeId::mint(), NodeId::mint()];
        let edge = EdgeId::mint();
        let proposer = AgentId::mint();
        let validator = AgentId::mint();
        let evidence = EvidenceId::mint();
        let mut parent_type = NodeType::new(parent, "Record");
        let mut property_type = PropertyDefinition::new(property, "health", ValueType::String);
        property_type.cardinality = cardinality;
        parent_type
            .properties
            .insert(property, property_type.clone());
        parent_type.properties.insert(
            reference,
            PropertyDefinition::new(
                reference,
                "owner",
                ValueType::NodeRef {
                    allowed_types: [parent].into_iter().collect(),
                },
            ),
        );
        let mut child_type = NodeType::new(child, "Project");
        child_type.parents.insert(parent);
        let mut edge_type = EdgeType::new(relation, "owned_by");
        edge_type.cardinality = cardinality;
        edge_type.source_types.insert(parent);
        edge_type.target_types.insert(parent);
        edge_type.properties.insert(property, property_type);
        let ontology = Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(schema, Timestamp::EPOCH),
            node_types: vec![parent_type, child_type],
            edge_types: vec![edge_type],
        })
        .unwrap();
        let graph = CanonicalGraph {
            root: GraphRoot {
                id: root,
                space: Space::Canonical,
                schema_version_id: schema,
                parent: None,
                created_at: Timestamp::EPOCH,
            },
            revision: RevisionNumber::new(1),
            ontology,
            nodes: nodes
                .into_iter()
                .map(|id| (id, Node::new(id, root, child, "Fixture project")))
                .collect(),
            edges: [(
                edge,
                Edge::new(
                    edge,
                    root,
                    relation,
                    CanonicalRef::new(nodes[0]),
                    CanonicalRef::new(nodes[1]),
                ),
            )]
            .into_iter()
            .collect(),
            evidence: [(
                evidence,
                Evidence {
                    id: evidence,
                    source: EvidenceSource::Document {
                        document_id: "fixture".into(),
                        section: None,
                    },
                    content_hash: ContentHash::of_bytes(b"fixture"),
                    extracted_by: proposer,
                    observed_at: Timestamp::EPOCH,
                    confidence: Confidence::CERTAIN,
                },
            )]
            .into_iter()
            .collect(),
            assertions: Default::default(),
            attachments: Default::default(),
        };
        Self {
            graph,
            nodes,
            edge,
            property,
            reference,
            relation,
            proposer,
            validator,
            evidence,
        }
    }
    fn claim(
        &mut self,
        subject: Subject,
        predicate: Predicate,
        object: Object,
        valid_time: TemporalRange,
    ) -> AssertionId {
        let id = AssertionId::mint();
        self.graph.assertions.insert(
            id,
            Assertion {
                id,
                root_id: self.graph.root.id,
                subject,
                predicate,
                object,
                evidence: [CanonicalRef::new(self.evidence)].into_iter().collect(),
                proposed_by: self.proposer,
                assessment: Assessment::Accepted {
                    validators: [self.validator].into_iter().collect(),
                },
                lifecycle: AssertionLifecycle::Active,
                valid_time,
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            },
        );
        id
    }
    fn property(&mut self, value: &str, time: TemporalRange) -> AssertionId {
        self.claim(
            Subject::Node(CanonicalRef::new(self.nodes[0])),
            Predicate::Property(self.property),
            Object::Value(CanonicalValue::String(value.into())),
            time,
        )
    }
}
fn range(from: Option<i64>, to: Option<i64>) -> TemporalRange {
    TemporalRange::new(
        from.map(Timestamp::from_millis),
        to.map(Timestamp::from_millis),
    )
    .unwrap()
}
fn pairs(graph: &CanonicalGraph) -> Vec<(AssertionId, AssertionId)> {
    preview_contradictions(graph)
        .iter()
        .map(|c| (c.left.0.parse().unwrap(), c.right.0.parse().unwrap()))
        .collect()
}
#[test]
fn overlapping_one_claims_return_every_pair_in_identity_order_without_mutation() {
    let mut f = Fixture::new(Cardinality::One);
    let mut ids = [
        f.property("red", range(None, Some(20))),
        f.property("amber", range(Some(10), None)),
        f.property("green", range(Some(15), Some(18))),
    ];
    let before = ContentHash::of(&f.graph.assertions);
    ids.sort();
    assert_eq!(
        pairs(&f.graph),
        vec![(ids[0], ids[1]), (ids[0], ids[2]), (ids[1], ids[2])]
    );
    assert_eq!(ContentHash::of(&f.graph.assertions), before);
    let report = serde_json::to_value(preview_contradictions(&f.graph)).unwrap();
    assert_eq!(
        report[0]["subject"],
        serde_json::json!({"kind":"Node","id":f.nodes[0].to_string()})
    );
    assert_eq!(
        report[0]["predicate"],
        serde_json::json!({"kind":"Property","id":f.property.to_string()})
    );
    assert_eq!(pairs(&f.graph), pairs(&f.graph.clone()));
}
#[test]
fn equal_disjoint_empty_and_many_claims_do_not_conflict() {
    for (cardinality, left, right, equal) in [
        (Cardinality::One, range(None, None), range(None, None), true),
        (
            Cardinality::One,
            range(None, Some(10)),
            range(Some(10), None),
            false,
        ),
        (
            Cardinality::One,
            range(Some(20), Some(30)),
            range(Some(0), Some(10)),
            false,
        ),
        (
            Cardinality::One,
            range(Some(10), Some(10)),
            range(None, None),
            false,
        ),
        (
            Cardinality::One,
            range(None, None),
            range(Some(10), Some(10)),
            false,
        ),
        (
            Cardinality::Many,
            range(None, None),
            range(None, None),
            false,
        ),
    ] {
        let mut f = Fixture::new(cardinality);
        f.property("amber", left);
        f.property(if equal { "amber" } else { "red" }, right);
        assert!(
            pairs(&f.graph).is_empty(),
            "{cardinality:?} {left:?} {right:?}"
        );
    }
}
#[test]
fn relation_claims_use_declared_cardinality_and_distinct_targets() {
    for cardinality in [Cardinality::One, Cardinality::Many] {
        let mut f = Fixture::new(cardinality);
        let subject = Subject::Node(CanonicalRef::new(f.nodes[0]));
        let predicate = Predicate::Relation(f.relation);
        let a = f.claim(
            subject,
            predicate,
            Object::Node(CanonicalRef::new(f.nodes[1])),
            range(None, None),
        );
        let b = f.claim(
            subject,
            predicate,
            Object::Node(CanonicalRef::new(f.nodes[2])),
            range(None, None),
        );
        assert_eq!(
            pairs(&f.graph).len(),
            usize::from(cardinality == Cardinality::One)
        );
        if cardinality == Cardinality::One {
            let report = serde_json::to_value(preview_contradictions(&f.graph)).unwrap();
            assert_eq!(report[0]["predicate"]["kind"], "Relation");
            assert_eq!(pairs(&f.graph), vec![(a.min(b), a.max(b))]);
        }
    }
}
#[test]
fn edge_properties_conflict_but_different_subjects_and_predicates_do_not() {
    let mut f = Fixture::new(Cardinality::One);
    let subject = Subject::Edge(CanonicalRef::new(f.edge));
    let predicate = Predicate::Property(f.property);
    let a = f.claim(
        subject,
        predicate,
        Object::Value(CanonicalValue::String("red".into())),
        range(None, None),
    );
    let b = f.claim(
        subject,
        predicate,
        Object::Value(CanonicalValue::String("amber".into())),
        range(None, None),
    );
    f.property("green", range(None, None));
    f.claim(
        Subject::Node(CanonicalRef::new(f.nodes[1])),
        predicate,
        Object::Value(CanonicalValue::String("red".into())),
        range(None, None),
    );
    f.claim(
        Subject::Node(CanonicalRef::new(f.nodes[0])),
        Predicate::Property(f.reference),
        Object::Node(CanonicalRef::new(f.nodes[1])),
        range(None, None),
    );
    assert_eq!(pairs(&f.graph), vec![(a.min(b), a.max(b))]);
    let report = serde_json::to_value(preview_contradictions(&f.graph)).unwrap();
    assert_eq!(report[0]["subject"]["kind"], "Edge");
}
#[test]
fn equivalent_node_reference_property_encodings_are_equal() {
    let mut f = Fixture::new(Cardinality::One);
    let subject = Subject::Node(CanonicalRef::new(f.nodes[0]));
    let predicate = Predicate::Property(f.reference);
    let a = f.claim(
        subject,
        predicate,
        Object::Node(CanonicalRef::new(f.nodes[1])),
        range(None, None),
    );
    let b = f.claim(
        subject,
        predicate,
        Object::Value(CanonicalValue::NodeRef(CanonicalRef::new(f.nodes[1]))),
        range(None, None),
    );
    assert!(pairs(&f.graph).is_empty());
    let c = f.claim(
        subject,
        predicate,
        Object::Node(CanonicalRef::new(f.nodes[2])),
        range(None, None),
    );
    let expected: BTreeSet<_> = [(a.min(c), a.max(c)), (b.min(c), b.max(c))]
        .into_iter()
        .collect();
    assert_eq!(
        pairs(&f.graph).into_iter().collect::<BTreeSet<_>>(),
        expected
    );
}
#[test]
fn only_active_accepted_or_disputed_claims_participate() {
    let mut f = Fixture::new(Cardinality::One);
    let a = f.property("red", range(None, None));
    let b = f.property("amber", range(None, None));
    for assessment in [
        Assessment::Proposed,
        Assessment::Validating {
            completed: 1,
            required: 7,
        },
        Assessment::Rejected { issues: vec![] },
    ] {
        f.graph.assertions.get_mut(&b).unwrap().assessment = assessment;
        assert!(pairs(&f.graph).is_empty());
    }
    f.graph.assertions.get_mut(&b).unwrap().assessment = Assessment::Disputed {
        competing_assertions: vec![CanonicalRef::new(a)],
    };
    assert_eq!(pairs(&f.graph).len(), 1);
    for lifecycle in [
        AssertionLifecycle::Retracted {
            at_revision: RevisionNumber::new(2),
            reason: RetractionReason::new("incorrect"),
        },
        AssertionLifecycle::Superseded {
            by: CanonicalRef::new(a),
            at_revision: RevisionNumber::new(2),
            effective_from: Timestamp::EPOCH,
        },
    ] {
        f.graph.assertions.get_mut(&b).unwrap().lifecycle = lifecycle;
        assert!(pairs(&f.graph).is_empty());
    }
    let claim = f.graph.assertions.get_mut(&b).unwrap();
    claim.lifecycle = AssertionLifecycle::Active;
    claim.transaction_time =
        TransactionTime::new(Timestamp::EPOCH, Some(Timestamp::from_millis(10))).unwrap();
    assert!(pairs(&f.graph).is_empty());
}

#[test]
fn temporal_extremes_match_point_membership_without_sentinel_collisions() {
    let ranges = [
        range(None, None),
        range(None, Some(i64::MIN)),
        range(Some(i64::MIN), Some(i64::MIN + 1)),
        range(None, Some(0)),
        range(Some(0), Some(0)),
        range(Some(0), None),
        range(None, Some(i64::MAX)),
        range(Some(i64::MAX), None),
    ];
    let witnesses = [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX];
    for left in ranges {
        for right in ranges {
            let contains = |r: TemporalRange, t: i64| {
                r.from.is_none_or(|from| from.millis() <= t)
                    && r.to.is_none_or(|to| t < to.millis())
            };
            let overlap = witnesses
                .into_iter()
                .any(|time| contains(left, time) && contains(right, time));
            let mut f = Fixture::new(Cardinality::One);
            f.property("red", left);
            f.property("amber", right);
            assert_eq!(!pairs(&f.graph).is_empty(), overlap, "{left:?} {right:?}");
        }
    }
}
