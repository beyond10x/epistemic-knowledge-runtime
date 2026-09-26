//! Additional adversarial combinations for the P1 membrane repair.

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue, Confidence, Edge,
    Evidence, EvidenceSource, GraphRoot, GraphSnapshot, Node, Object, Predicate, Space, Subject,
    TemporalRange, TransactionTime,
};
use ekr_kernel::{
    EdgeDraft, GraphOperation, GraphTransaction, NodeDraft, Pipeline, PropertyMutation,
    ValidatorName,
};
use ekr_ontology::{
    Cardinality, EdgeType, Lifecycle, NodeType, Ontology, OntologyDocument, OperationDefinition,
    PropertyDefinition, SchemaVersion, Transition, Value, ValueType,
};
use std::collections::BTreeMap;

/// of retained evidence, at revision 7.
struct World {
    graph: CanonicalGraph,
    root_id: GraphRootId,
    decision: TypeId,
    depends_on: TypeId,
    title: PropertyId,
    tags: PropertyId,
    supersedes: PropertyId,
    open: NodeId,
    decided: NodeId,
    existing_edge: EdgeId,
    held_assertion: AssertionId,
    retained_evidence: EvidenceId,
    proposer: AgentId,
    reviewer: AgentId,
}

impl World {
    /// The canonical state at revision 7, and the identities the cases name.
    fn new() -> Self {
        let root_id = GraphRootId::mint();
        let schema = SchemaVersionId::mint();
        let (decision, depends_on) = (TypeId::mint(), TypeId::mint());
        let (title, tags, supersedes) =
            (PropertyId::mint(), PropertyId::mint(), PropertyId::mint());
        let (open, decided) = (NodeId::mint(), NodeId::mint());
        let (existing_edge, held_assertion) = (EdgeId::mint(), AssertionId::mint());
        let (retained_evidence, proposer, reviewer) =
            (EvidenceId::mint(), AgentId::mint(), AgentId::mint());

        let mut decision_type = NodeType::new(decision, "Decision");
        let mut required_title = PropertyDefinition::new(title, "title", ValueType::String);
        required_title.required = true;
        decision_type.properties.insert(title, required_title);
        let mut many_tags = PropertyDefinition::new(tags, "tags", ValueType::String);
        many_tags.cardinality = Cardinality::Many;
        decision_type.properties.insert(tags, many_tags);
        decision_type.properties.insert(
            supersedes,
            PropertyDefinition::new(
                supersedes,
                "supersedes",
                ValueType::NodeRef {
                    allowed_types: [decision].into_iter().collect(),
                },
            ),
        );
        decision_type.lifecycle = Some(Lifecycle {
            initial: "open".to_owned(),
            states: ["open", "decided", "moot"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            transitions: [Transition::new("open", "decided")].into_iter().collect(),
        });
        let mut decide = OperationDefinition::new("decide");
        decide.transition = Some(Transition::new("open", "decided"));
        decision_type.operations.insert("decide".to_owned(), decide);

        let mut depends_on_type = EdgeType::new(depends_on, "depends_on");
        depends_on_type.source_types = [decision].into_iter().collect();
        depends_on_type.target_types = [decision].into_iter().collect();
        depends_on_type.cardinality = Cardinality::One;

        let ontology = Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(schema, Timestamp::EPOCH),
            node_types: vec![decision_type],
            edge_types: vec![depends_on_type],
        })
        .expect("the fixture ontology coheres");

        let mut open_node = Node::new(open, root_id, decision, "Adopt the eventlog store");
        open_node.type_state = Some("open".to_owned());
        open_node.properties.insert(
            title,
            vec![CanonicalValue::String(
                "Adopt the eventlog store".to_owned(),
            )],
        );
        let mut decided_node = Node::new(decided, root_id, decision, "Hash canonical state only");
        decided_node.type_state = Some("decided".to_owned());
        decided_node.properties.insert(
            title,
            vec![CanonicalValue::String(
                "Hash canonical state only".to_owned(),
            )],
        );

        let edge = Edge::new(
            existing_edge,
            root_id,
            depends_on,
            CanonicalRef::new(open),
            CanonicalRef::new(decided),
        );

        let assertion = Assertion {
            id: held_assertion,
            root_id,
            subject: Subject::Node(CanonicalRef::new(open)),
            predicate: Predicate::Property(title),
            object: Object::Value(CanonicalValue::String(
                "Adopt the eventlog store".to_owned(),
            )),
            evidence: [CanonicalRef::new(retained_evidence)].into_iter().collect(),
            proposed_by: proposer,
            lifecycle: ekr_graph::AssertionLifecycle::Active,
            assessment: Assessment::Accepted {
                validators: [reviewer].into_iter().collect(),
            },
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        };

        let evidence = Evidence {
            id: retained_evidence,
            source: EvidenceSource::Document {
                document_id: "docs/roadmap.md".to_owned(),
                section: Some("Phase 1".to_owned()),
            },
            content_hash: ContentHash::of(&"the roadmap's phase 1".to_owned()),
            extracted_by: proposer,
            observed_at: Timestamp::EPOCH,
            confidence: Confidence::CERTAIN,
        };

        Self {
            graph: CanonicalGraph {
                root: GraphRoot {
                    id: root_id,
                    space: Space::Canonical,
                    schema_version_id: schema,
                    parent: None,
                    created_at: Timestamp::EPOCH,
                },
                revision: RevisionNumber::new(7),
                ontology,
                nodes: [(open, open_node), (decided, decided_node)]
                    .into_iter()
                    .collect(),
                edges: [(existing_edge, edge)].into_iter().collect(),
                assertions: [(held_assertion, assertion)].into_iter().collect(),
                evidence: [(retained_evidence, evidence)].into_iter().collect(),
            },
            root_id,
            decision,
            depends_on,
            title,
            tags,
            supersedes,
            open,
            decided,
            existing_edge,
            held_assertion,
            retained_evidence,
            proposer,
            reviewer,
        }
    }

    fn snapshot(&self) -> GraphSnapshot<'_> {
        GraphSnapshot::of(&self.graph)
    }

    /// A transaction by the fixture's proposer, carrying the operations given.
    ///
    /// Its evidence set is derived from those operations rather than fixed, because the set is a
    /// manifest the structural validator holds equal to what the transaction's assertions cite —
    /// a fixture that declared a constant set would be testing the manifest rule in every case
    /// instead of the one written for it.
    fn proposal(&self, operations: Vec<GraphOperation>) -> GraphTransaction {
        let evidence = operations
            .iter()
            .filter_map(|operation| match operation {
                GraphOperation::AddAssertion(assertion) => Some(assertion.evidence.iter().copied()),
                _ => None,
            })
            .flatten()
            .collect();
        GraphTransaction {
            id: TransactionId::mint(),
            proposer: self.proposer,
            operations,
            evidence,
            schema_version: None,
        }
    }

    /// A well-formed `Decision` draft: a new id, the required title, nothing else.
    fn draft(&self, id: NodeId, name: &str) -> NodeDraft {
        NodeDraft {
            id,
            root_id: self.root_id,
            type_id: self.decision,
            canonical_name: name.to_owned(),
            properties: [(self.title, vec![Value::String(name.to_owned())])]
                .into_iter()
                .collect(),
        }
    }

    /// An assertion that cites the retained evidence, so provenance has nothing to say about it.
    fn supported_assertion(&self, id: AssertionId) -> Assertion<Value> {
        Assertion {
            id,
            root_id: self.root_id,
            subject: Subject::Node(self.open),
            predicate: Predicate::Property(self.title),
            object: Object::Value(Value::String("Adopt the eventlog store".to_owned())),
            evidence: [self.retained_evidence].into_iter().collect(),
            proposed_by: self.proposer,
            lifecycle: ekr_graph::AssertionLifecycle::Active,
            assessment: Assessment::Proposed,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        }
    }

    /// The deterministic pipeline, run by an actor who is not the proposer.
    fn pipeline(&self) -> Pipeline {
        Pipeline::deterministic(self.reviewer)
    }
}

#[test]
fn assertion_subject_predicate_object_cross_product_obeys_declarations() {
    let mut world = World::new();
    let mut relation = world
        .graph
        .ontology
        .edge_type(world.depends_on)
        .unwrap()
        .clone();
    relation.properties = world
        .graph
        .ontology
        .node_type(world.decision)
        .unwrap()
        .properties
        .clone();
    world.graph.ontology = Ontology::load(OntologyDocument {
        version: world.graph.ontology.version().clone(),
        node_types: vec![world
            .graph
            .ontology
            .node_type(world.decision)
            .unwrap()
            .clone()],
        edge_types: vec![relation],
    })
    .unwrap();
    for (si, subject) in [
        Subject::Node(world.open),
        Subject::Edge(world.existing_edge),
        Subject::Type(world.decision),
    ]
    .into_iter()
    .enumerate()
    {
        for (pi, predicate) in [
            Predicate::Property(world.title),
            Predicate::Property(world.supersedes),
            Predicate::Relation(world.depends_on),
        ]
        .into_iter()
        .enumerate()
        {
            for (oi, object) in [
                Object::Value(Value::String("title".into())),
                Object::Node(world.decided),
                Object::Type(world.decision),
                Object::Value(Value::NodeRef(world.decided)),
                Object::Value(Value::Integer(7)),
            ]
            .into_iter()
            .enumerate()
            {
                let mut assertion = world.supported_assertion(AssertionId::mint());
                assertion.subject = subject;
                assertion.predicate = predicate;
                assertion.object = object;
                let result = world.pipeline().validate(
                    &world.snapshot(),
                    &world.proposal(vec![GraphOperation::AddAssertion(Box::new(assertion))]),
                );
                let valid = match pi {
                    0 => si < 2 && oi == 0,
                    1 => si < 2 && (oi == 1 || oi == 3),
                    _ => si == 0 && oi == 1,
                };
                assert_eq!(
                    result.is_ok(),
                    valid,
                    "subject {si}, predicate {pi}, object {oi}: {result:?}"
                );
                if let Err(issues) = result {
                    assert!(
                        issues
                            .iter()
                            .all(|issue| issue.validator == ValidatorName::Type),
                        "assertion semantics must be the type validator's refusal: {issues:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn cancellation_cannot_leave_a_new_assertion_referring_to_the_cancelled_edge() {
    let mut world = World::new();
    let mut relation = world
        .graph
        .ontology
        .edge_type(world.depends_on)
        .unwrap()
        .clone();
    relation.properties.insert(
        world.title,
        PropertyDefinition::new(world.title, "title", ValueType::String),
    );
    world.graph.ontology = Ontology::load(OntologyDocument {
        version: world.graph.ontology.version().clone(),
        node_types: vec![world
            .graph
            .ontology
            .node_type(world.decision)
            .unwrap()
            .clone()],
        edge_types: vec![relation],
    })
    .unwrap();
    let id = EdgeId::mint();
    let create = GraphOperation::CreateEdge(EdgeDraft {
        id,
        root_id: world.root_id,
        type_id: world.depends_on,
        source: world.decided,
        target: world.open,
        properties: BTreeMap::new(),
    });
    let delete = GraphOperation::DeleteEdge(id);
    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.subject = Subject::Edge(id);
    let add = GraphOperation::AddAssertion(Box::new(assertion));
    for operations in [
        vec![create.clone(), delete.clone()],
        vec![delete.clone(), create.clone()],
    ] {
        assert!(world
            .pipeline()
            .validate(&world.snapshot(), &world.proposal(operations))
            .is_ok());
    }
    for operations in [
        vec![create.clone(), delete.clone(), add.clone()],
        vec![add.clone(), delete.clone(), create.clone()],
        vec![delete, create, add],
    ] {
        let issues = world
            .pipeline()
            .validate(&world.snapshot(), &world.proposal(operations))
            .unwrap_err();
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].validator, ValidatorName::Reference);
        assert_eq!(issues[0].code, "unresolved-edge");
    }
}

#[test]
fn retracting_a_retained_edge_assertion_does_not_erase_its_reference() {
    let mut world = World::new();
    let delete = GraphOperation::DeleteEdge(world.existing_edge);
    assert!(world
        .pipeline()
        .validate(&world.snapshot(), &world.proposal(vec![delete.clone()]))
        .is_ok());
    world
        .graph
        .assertions
        .get_mut(&world.held_assertion)
        .unwrap()
        .subject = Subject::Edge(CanonicalRef::new(world.existing_edge));
    let retract = GraphOperation::RetractAssertion(ekr_kernel::Retraction {
        assertion: world.held_assertion,
        reason: ekr_graph::RetractionReason::new("fixture withdrawal"),
    });
    for operations in [vec![delete.clone(), retract.clone()], vec![retract, delete]] {
        let issues = world
            .pipeline()
            .validate(&world.snapshot(), &world.proposal(operations))
            .unwrap_err();
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].validator, ValidatorName::Reference);
        assert_eq!(issues[0].code, "unresolved-edge");
    }
}

#[test]
fn independent_property_and_lifecycle_writes_are_order_invariant() {
    let world = World::new();
    let update = GraphOperation::UpdateProperty(PropertyMutation {
        node: world.open,
        property: world.tags,
        values: vec![Value::String("changed".into())],
    });
    let invoke = GraphOperation::Invoke {
        node: world.open,
        operation: "decide".into(),
        arguments: BTreeMap::new(),
    };
    for operations in [
        vec![update.clone(), invoke.clone()],
        vec![invoke, update.clone()],
        vec![update.clone(), update],
    ] {
        let result = world
            .pipeline()
            .validate(&world.snapshot(), &world.proposal(operations));
        assert!(
            result.is_ok(),
            "independent or identical property writes: {result:?}"
        );
    }
    let node = NodeId::mint();
    let create = GraphOperation::CreateNode(world.draft(node, "New decision"));
    let invoke = GraphOperation::Invoke {
        node,
        operation: "decide".into(),
        arguments: BTreeMap::new(),
    };
    for operations in [vec![create.clone(), invoke.clone()], vec![invoke, create]] {
        let result = world
            .pipeline()
            .validate(&world.snapshot(), &world.proposal(operations));
        assert!(
            result.is_ok(),
            "invoke newly created node at initial state: {result:?}"
        );
    }
}

#[test]
fn inherited_opaque_constraints_are_refused_on_child_node_properties() {
    let mut world = World::new();
    let mut parent = world
        .graph
        .ontology
        .node_type(world.decision)
        .unwrap()
        .clone();
    parent
        .properties
        .get_mut(&world.title)
        .unwrap()
        .constraints
        .push("not-evaluated".into());
    let child_id = TypeId::mint();
    let mut child = NodeType::new(child_id, "SpecialDecision");
    child.parents.insert(world.decision);
    world.graph.ontology = Ontology::load(OntologyDocument {
        version: world.graph.ontology.version().clone(),
        node_types: vec![parent, child],
        edge_types: vec![world
            .graph
            .ontology
            .edge_type(world.depends_on)
            .unwrap()
            .clone()],
    })
    .unwrap();
    world.graph.nodes.get_mut(&world.open).unwrap().type_id = child_id;
    let operation = GraphOperation::UpdateProperty(PropertyMutation {
        node: world.open,
        property: world.title,
        values: vec![Value::String("changed".into())],
    });
    let issues = world
        .pipeline()
        .validate(&world.snapshot(), &world.proposal(vec![operation]))
        .unwrap_err();
    assert_eq!(issues.len(), 1, "{issues:?}");
    assert_eq!(issues[0].validator, ValidatorName::OntologyConstraint);
    assert_eq!(issues[0].code, "unsupported-constraint");
}
