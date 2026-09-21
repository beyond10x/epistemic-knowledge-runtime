//! Adversary pass 2 over `story:transaction-and-validators`.
//!
//! Two claims the unit makes about itself, driven against the code that makes them.
//!
//! 1. `src/validate/structural.rs` says the class it refuses is "an operation that brings an
//!    identity into existence", that "all three creation paths are covered", and — in
//!    `tests/validation.rs` — that "a rule enumerated by its instances is a rule with a next
//!    instance". `DefineNodeType` and `DefineEdgeType` each bring a `TypeId` into existence and
//!    are the next two instances.
//! 2. `ekr_graph::ValidationState::Accepted` is documented as "it crossed the integrity boundary",
//!    and design § 21 makes that crossing what the canonical core *is*. The kernel is that
//!    boundary, and nothing in it reads the field an `AddAssertion` arrives carrying.

use std::collections::BTreeMap;

use ekr_core::{
    AgentId, AssertionId, ContentHash, EvidenceId, GraphRootId, NodeId, PropertyId, RevisionNumber,
    SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, CanonicalGraph, CanonicalValue, Confidence, Evidence, EvidenceSource, GraphRoot,
    GraphSnapshot, Node, Object, Predicate, Space, Subject, TemporalRange, TransactionTime,
    ValidationState,
};
use ekr_kernel::{GraphOperation, GraphTransaction, Pipeline, ValidationIssue};
use ekr_ontology::{
    EdgeType, NodeType, Ontology, OntologyDocument, PropertyDefinition, SchemaVersion, Value,
    ValueType,
};

/// The same shape of world `tests/validation.rs` proposes against, built here because an
/// integration test is its own binary: one declared node type, one declared edge type, one node,
/// one retained piece of evidence, at revision 7.
struct World {
    graph: CanonicalGraph,
    root_id: GraphRootId,
    decision: TypeId,
    depends_on: TypeId,
    title: PropertyId,
    open: NodeId,
    retained_evidence: EvidenceId,
    proposer: AgentId,
    reviewer: AgentId,
}

impl World {
    fn new() -> Self {
        let root_id = GraphRootId::mint();
        let schema = SchemaVersionId::mint();
        let (decision, depends_on) = (TypeId::mint(), TypeId::mint());
        let title = PropertyId::mint();
        let open = NodeId::mint();
        let (retained_evidence, proposer, reviewer) =
            (EvidenceId::mint(), AgentId::mint(), AgentId::mint());

        let mut decision_type = NodeType::new(decision, "Decision");
        decision_type.properties.insert(
            title,
            PropertyDefinition::new(title, "title", ValueType::String),
        );

        let mut depends_on_type = EdgeType::new(depends_on, "depends_on");
        depends_on_type.source_types = [decision].into_iter().collect();
        depends_on_type.target_types = [decision].into_iter().collect();

        let ontology = Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(schema, Timestamp::EPOCH),
            node_types: vec![decision_type],
            edge_types: vec![depends_on_type],
        })
        .expect("the fixture ontology coheres");

        let mut open_node = Node::new(open, root_id, decision, "Adopt the eventlog store");
        open_node.properties.insert(
            title,
            CanonicalValue::String("Adopt the eventlog store".to_owned()),
        );

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
                nodes: [(open, open_node)].into_iter().collect(),
                edges: BTreeMap::new(),
                assertions: BTreeMap::new(),
                evidence: [(retained_evidence, evidence)].into_iter().collect(),
            },
            root_id,
            decision,
            depends_on,
            title,
            open,
            retained_evidence,
            proposer,
            reviewer,
        }
    }

    fn snapshot(&self) -> GraphSnapshot<'_> {
        GraphSnapshot::of(&self.graph)
    }

    /// A proposal whose declared evidence set is derived from its assertions, so the manifest rule
    /// added in correction round 1 has nothing to say about any case here.
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

    fn pipeline(&self) -> Pipeline {
        Pipeline::deterministic(self.reviewer)
    }

    /// Validates `operations` and returns the issues, or `None` when the membrane let it through.
    fn outcome(&self, operations: Vec<GraphOperation>) -> Option<Vec<ValidationIssue>> {
        self.pipeline()
            .validate(&self.snapshot(), &self.proposal(operations))
            .err()
    }
}

/// A `TypeId` the ontology already declares is not declared a second time.
///
/// `src/validate/structural.rs` widened its class from the two instances pass 1 named to three —
/// node, edge and assertion — on the ground that the class is "an operation that brings an
/// identity into existence" rather than a list. It is five. `DefineNodeType` and `DefineEdgeType`
/// each mint a `TypeId`, which AGENTS.md invariant 3 makes an identity ("every persistent object
/// carries a stable id"), and a declaration over an id the ontology already holds replaces a
/// committed declaration through the declare path — which AGENTS.md invariant 5 says is never how
/// a committed record changes.
///
/// The three schema operations reach no validator at all: `Structural` matches them into `_`,
/// `Reference` into an empty arm, `Types` and `Cardinality` into an empty arm,
/// `OntologyConstraint` reads only `Invoke`, `Provenance` only `AddAssertion`, and `Authorization`
/// reads no operation. So this is not one missing arm; it is the whole schema half of
/// `ekr.kernel.OperationKind` passing the membrane unlooked-at.
#[test]
fn a_type_the_ontology_already_declares_is_not_declared_again() {
    let world = World::new();

    let node_type = world.outcome(vec![GraphOperation::DefineNodeType(Box::new(
        NodeType::new(world.decision, "Something else entirely"),
    ))]);
    assert!(
        node_type.is_some(),
        "a DefineNodeType over the TypeId the ontology already declares for `Decision` passed \
         every validator"
    );

    let edge_type = world.outcome(vec![GraphOperation::DefineEdgeType(Box::new(
        EdgeType::new(world.depends_on, "something_else"),
    ))]);
    assert!(
        edge_type.is_some(),
        "a DefineEdgeType over the TypeId the ontology already declares for `depends_on` passed \
         every validator"
    );
}

/// A proposer cannot declare its own assertion to have crossed the integrity boundary.
///
/// `ekr_graph::ValidationState::Accepted` is documented as "it crossed the integrity boundary. The
/// agents whose validation it rests on are named", and `ValidationState::is_accepted` cites design
/// § 21: the canonical core is what "has crossed the system's highest integrity boundary". This
/// kernel *is* that boundary (AGENTS.md invariant 1), and nothing in it reads the `validation`
/// field an `AddAssertion` arrives carrying — `transaction.rs:610` copies it into the canonical
/// form unchanged.
///
/// So an agent proposes an assertion already marked `Accepted`, naming as its validators agents
/// that never saw it, and the pipeline seals it. That is AGENTS.md invariant 4 — "an agent said so
/// is not sufficient" — defeated the same way pass 1's blocker defeated it: by the agent writing
/// the answer down itself. `src/validate/provenance.rs:26` considered reading this field and
/// declined, but its reason covers only the opposite direction: a proposer labelling a claim
/// `Proposed` to opt *out* of § 6.5. Opting *in* is not addressed and is not refused.
#[test]
fn a_proposer_cannot_mark_its_own_assertion_accepted() {
    let world = World::new();
    let forged = Assertion {
        id: AssertionId::mint(),
        root_id: world.root_id,
        subject: Subject::Node(world.open),
        predicate: Predicate::Property(world.title),
        object: Object::Value(Value::String("Adopt the eventlog store".to_owned())),
        evidence: [world.retained_evidence].into_iter().collect(),
        proposed_by: world.proposer,
        // The agent names the validator that did not act, on a claim it is proposing now.
        validation: ValidationState::Accepted {
            validators: [world.reviewer].into_iter().collect(),
        },
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };

    let issues = world.outcome(vec![GraphOperation::AddAssertion(Box::new(forged))]);
    assert!(
        issues.is_some(),
        "an assertion proposed as already `Accepted`, by validators that never saw it, passed \
         every validator and was sealed into a ValidatedTransaction"
    );

    // And the same assertion arriving as `Proposed` is the ordinary shape, so the refusal above is
    // about the state it claims and not about the assertion.
    let honest = Assertion {
        id: AssertionId::mint(),
        root_id: world.root_id,
        subject: Subject::Node(world.open),
        predicate: Predicate::Property(world.title),
        object: Object::Value(Value::String("Adopt the eventlog store".to_owned())),
        evidence: [world.retained_evidence].into_iter().collect(),
        proposed_by: world.proposer,
        validation: ValidationState::Proposed,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    assert_eq!(
        world.outcome(vec![GraphOperation::AddAssertion(Box::new(honest))]),
        None,
        "a `Proposed` assertion citing retained evidence is the ordinary shape and validates"
    );
}

/// An assertion whose object is not a value is still type-checked.
///
/// `src/validate/reference.rs:15` enumerates what it resolves and excuses exactly one identity:
///
/// > A `TypeId` is not: the ontology is not the graph, and an operation naming a type the schema
/// > does not declare is the type validator's refusal, where the rest of that operation's typing
/// > is decided.
///
/// For an `AddAssertion` it is not. `src/validate/types.rs:204` opens that arm with
/// `let Object::Value(value) = &assertion.object else { continue; }`, so every assertion whose
/// object is a node or a type leaves the arm before anything is looked at, and the one that stays
/// then requires `(Subject::Node, Predicate::Property)` and `continue`s otherwise. Between the two
/// validators, an assertion may name a `TypeId` nothing declares as its subject, as its predicate
/// or as its object, and no validator asks.
///
/// The third shape below is what shows this is a hole rather than a deferral: one predicate naming
/// a property the subject's type does not declare is refused with `undeclared-property` when the
/// object is a value, and is not refused at all when the object is a node. One defect, two
/// verdicts, decided by a field that is not the defect.
#[test]
fn an_assertion_naming_a_type_the_ontology_does_not_declare_is_refused() {
    let world = World::new();
    let undeclared_type = TypeId::mint();
    let undeclared_property = PropertyId::mint();

    let about_a_type = Assertion {
        id: AssertionId::mint(),
        root_id: world.root_id,
        subject: Subject::Type(undeclared_type),
        predicate: Predicate::Property(world.title),
        object: Object::Value(Value::String("a schema-level claim".to_owned())),
        evidence: [world.retained_evidence].into_iter().collect(),
        proposed_by: world.proposer,
        validation: ValidationState::Proposed,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    assert!(
        world
            .outcome(vec![GraphOperation::AddAssertion(Box::new(about_a_type))])
            .is_some(),
        "an assertion whose subject is a TypeId the ontology does not declare passed every \
         validator"
    );

    let by_a_relation = Assertion {
        id: AssertionId::mint(),
        root_id: world.root_id,
        subject: Subject::Node(world.open),
        predicate: Predicate::Relation(undeclared_type),
        object: Object::Node(world.open),
        evidence: [world.retained_evidence].into_iter().collect(),
        proposed_by: world.proposer,
        validation: ValidationState::Proposed,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    assert!(
        world
            .outcome(vec![GraphOperation::AddAssertion(Box::new(by_a_relation))])
            .is_some(),
        "an assertion whose predicate is an edge type the ontology does not declare passed every \
         validator"
    );

    // One defect, two objects. The value form is refused with `undeclared-property`.
    let undeclared_against_a_value = Assertion {
        id: AssertionId::mint(),
        root_id: world.root_id,
        subject: Subject::Node(world.open),
        predicate: Predicate::Property(undeclared_property),
        object: Object::Value(Value::String("one".to_owned())),
        evidence: [world.retained_evidence].into_iter().collect(),
        proposed_by: world.proposer,
        validation: ValidationState::Proposed,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    let refused = world
        .outcome(vec![GraphOperation::AddAssertion(Box::new(
            undeclared_against_a_value,
        ))])
        .expect("the value form of this defect is refused");
    assert!(
        refused
            .iter()
            .any(|issue| issue.code == "undeclared-property"),
        "the value form is refused with undeclared-property: {refused:?}"
    );

    // The node form is the same defect and is refused by nobody.
    let undeclared_against_a_node = Assertion {
        id: AssertionId::mint(),
        root_id: world.root_id,
        subject: Subject::Node(world.open),
        predicate: Predicate::Property(undeclared_property),
        object: Object::Node(world.open),
        evidence: [world.retained_evidence].into_iter().collect(),
        proposed_by: world.proposer,
        validation: ValidationState::Proposed,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    assert!(
        world
            .outcome(vec![GraphOperation::AddAssertion(Box::new(
                undeclared_against_a_node
            ))])
            .is_some(),
        "the same undeclared property, with a node as the object instead of a value, passed every \
         validator"
    );
}

/// A node is not merged into itself.
///
/// `EntityMerge`'s doc makes the two ids asymmetric — "a merge names the id that remains and the
/// id that becomes an alias of it" — so `absorbed == into` names one id as both the record that
/// survives and the record that stops being its own entity. The reference validator resolves both
/// halves and is satisfied because they are the same existing node, and nothing else reads a
/// `MergeEntity` at all.
///
/// The generator in `tests/validate_properties.rs` draws `absorbed` and `into` from one pool of
/// three independently, so it produces this shape roughly one proposal in three and no property
/// says anything about it.
#[test]
fn a_node_is_not_merged_into_itself() {
    let world = World::new();
    assert!(
        world
            .outcome(vec![GraphOperation::MergeEntity(ekr_kernel::EntityMerge {
                absorbed: world.open,
                into: world.open,
            })])
            .is_some(),
        "a MergeEntity naming one node as both the absorbed record and the surviving one passed \
         every validator"
    );
}
