//! External review probes at 209dd5e. Fixture copied from kernel validation.rs.
//!
//! The story's acceptance is one sentence — *a transaction carrying exactly one of the five
//! defects is refused, and the issue names that defect* — and it is written that way because the
//! sentence it replaced could not fail. "`validate` never returns `Ok` when any operation carries
//! a reference that does not resolve" is satisfied completely by a `validate` that answers `Err`
//! to everything. Two halves therefore live here and neither is redundant:
//!
//! * [`a_valid_transaction_validates`] closes the degenerate refuser, and
//! * the five defect cases below each assert the issue set names **exactly one** validator — the
//!   one the defect belongs to — which closes the degenerate accepter *and* a pipeline that
//!   refuses for the wrong reason.
//!
//! The fixture is the runtime's own vocabulary: a `Decision` node type with the lifecycle design
//! amendment 87 gives it, and a `depends_on` edge type between decisions. No person, real or
//! invented, appears in it.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::canonical::Canonical;
use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, CanonicalGraph, CanonicalRef, CanonicalValue, Confidence, Edge, Evidence,
    EvidenceSource, GraphRoot, GraphSnapshot, Node, Object, Predicate, RetractionReason, Space,
    Subject, TemporalRange, TransactionTime, ValidationState,
};
use ekr_kernel::{
    Authorization, Cardinality as CardinalityValidator, EdgeDraft, EntityMerge, GraphOperation,
    GraphTransaction, NodeDraft, OntologyConstraint, Pipeline, PropertyMutation, Provenance,
    Reference, Structural, Types, ValidationIssue, Validator, ValidatorName,
};
use ekr_ontology::{
    Cardinality, EdgeType, Lifecycle, NodeType, Ontology, OntologyDocument, OperationDefinition,
    PropertyDefinition, SchemaVersion, Transition, Value, ValueType,
};

/// The world every case below proposes against: one ontology, two decisions, one edge, one piece
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

fn verdict(world: &World, label: &str, operations: Vec<GraphOperation>) {
    let proposal = world.proposal(operations);
    let result = world.pipeline().validate(&world.snapshot(), &proposal);
    println!("{label}: {}", if result.is_ok() { "ACCEPTED" } else { "REFUSED" });
    if let Err(issues) = result { println!("  {issues:?}"); }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    use ekr_kernel::Commit;
    use ekr_store::{GraphDocument, ObjectStore, RevisionLog, SqliteStore, StorageClass};
    let world = World::new();
    verdict(&world, "positive control: valid node creation", vec![GraphOperation::CreateNode(world.draft(NodeId::mint(), "valid"))]);
    verdict(&world, "negative control: String property with Integer value", vec![GraphOperation::UpdateProperty(PropertyMutation { node: world.open, property: world.title, values: vec![Value::Integer(42)] })]);
    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.subject = Subject::Edge(world.existing_edge);
    verdict(&world, "delete an edge while adding an assertion referring to that edge", vec![
        GraphOperation::DeleteEdge(world.existing_edge),
        GraphOperation::AddAssertion(Box::new(assertion)),
    ]);

    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.predicate = Predicate::Relation(world.depends_on);
    assertion.object = Object::Value(Value::String("not a permitted node endpoint".into()));
    verdict(&world, "relation requiring Decision endpoints with String object", vec![
        GraphOperation::AddAssertion(Box::new(assertion)),
    ]);

    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.object = Object::Type(world.decision);
    verdict(&world, "String property assertion with a Type object", vec![
        GraphOperation::AddAssertion(Box::new(assertion)),
    ]);

    verdict(&world, "two conflicting updates of one property in unordered transaction", vec![
        GraphOperation::UpdateProperty(PropertyMutation { node: world.open, property: world.title, values: vec![Value::String("first".into())] }),
        GraphOperation::UpdateProperty(PropertyMutation { node: world.open, property: world.title, values: vec![Value::String("second".into())] }),
    ]);

    let mut invalid_type = NodeType::new(TypeId::mint(), "Invalid");
    invalid_type.parents.insert(TypeId::mint());
    verdict(&world, "define a type with nonexistent parent", vec![GraphOperation::DefineNodeType(Box::new(invalid_type))]);

    let mut modified = PropertyDefinition::new(PropertyId::mint(), "missing", ValueType::String);
    modified.required = true;
    verdict(&world, "modify a property that no type declares", vec![GraphOperation::ModifyProperty(modified)]);

    let mut constrained = World::new();
    let mut node_type = constrained.graph.ontology.node_type(constrained.decision).unwrap().clone();
    node_type.operations.get_mut("decide").unwrap().preconditions.push("UNSUPPORTED_CONSTRAINT".into());
    constrained.graph.ontology = Ontology::load(OntologyDocument {
        version: constrained.graph.ontology.version().clone(),
        node_types: vec![node_type],
        edge_types: vec![constrained.graph.ontology.edge_type(constrained.depends_on).unwrap().clone()],
    })?;
    verdict(&constrained, "Invoke with a nonempty unsupported precondition", vec![GraphOperation::Invoke { node: constrained.open, operation: "decide".into(), arguments: BTreeMap::new() }]);

    let mut document = GraphDocument::of(&world.graph);
    document.edges.get_mut(&world.existing_edge).unwrap().target = NodeId::mint();
    println!("seed document with dangling edge: {}", if document.into_canonical(world.graph.ontology.clone()).is_ok() { "ACCEPTED" } else { "REFUSED" });

    let directory = tempfile::TempDir::new()?;
    let path = directory.path().join("revisions.db");
    let open = || Commit::over(|validations| SqliteStore::sqlite(&path, "ekr", world.graph.ontology.clone()).map(|store| store.under(validations)));
    let commit = open()?;
    let seed = commit.store().put(StorageClass::Canonical, &GraphDocument::of(&world.graph).to_bytes()?, Timestamp::EPOCH)?;
    let _ = commit.store().append(&ekr_graph::RevisionEvent::Seeded { revision_id: ekr_core::RevisionId::mint(), seed_hash: seed.content_hash })?;
    let state = commit.store().fold()?;
    let node = NodeId::mint();
    let proposal = world.proposal(vec![GraphOperation::CreateNode(world.draft(node, "new node"))]);
    let validated = world.pipeline().validate(&GraphSnapshot::of(&state), &proposal).unwrap();
    let head = commit.commit(validated)?;
    println!("commit CreateNode: revision={}, node_present={}", head.revision, commit.store().fold()?.nodes.contains_key(&node));
    drop(commit);
    let reopened = open()?;
    println!("reopen through real kernel authority: revision={}", reopened.store().head()?.unwrap().revision);
    Ok(())
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
            CanonicalValue::String("Adopt the eventlog store".to_owned()),
        );
        let mut decided_node = Node::new(decided, root_id, decision, "Hash canonical state only");
        decided_node.type_state = Some("decided".to_owned());
        decided_node.properties.insert(
            title,
            CanonicalValue::String("Hash canonical state only".to_owned()),
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
            evidence: [retained_evidence].into_iter().collect(),
            proposed_by: proposer,
            validation: ValidationState::Accepted {
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
            validation: ValidationState::Proposed,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        }
    }

    /// The deterministic pipeline, run by an actor who is not the proposer.
    fn pipeline(&self) -> Pipeline {
        Pipeline::deterministic(self.reviewer)
    }
}
