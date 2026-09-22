//! The integrity membrane: design § 19–20, and `story:transaction-and-validators`.
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

/// The validators that raised the issues, deduplicated and sorted by the order the pipeline runs
/// them — what "the issue names that defect" is asserted against.
fn refusing_validators(issues: &[ValidationIssue]) -> Vec<ValidatorName> {
    let mut named: Vec<ValidatorName> = issues.iter().map(|issue| issue.validator).collect();
    named.dedup();
    named
}

/// The codes the issues carry, in the order they were raised.
fn codes(issues: &[ValidationIssue]) -> Vec<&str> {
    issues.iter().map(|issue| issue.code.as_str()).collect()
}

/// Validates `operations` as a proposal of `world` and expects a refusal.
fn refuse(world: &World, operations: Vec<GraphOperation>) -> Vec<ValidationIssue> {
    let proposal = world.proposal(operations);
    match world.pipeline().validate(&world.snapshot(), &proposal) {
        Ok(validated) => panic!(
            "expected a refusal; the pipeline validated against revision {} with hash {}",
            validated.validated_against(),
            validated.validation_hash()
        ),
        Err(issues) => {
            assert!(
                issues
                    .iter()
                    .all(|issue| issue.transaction_id == proposal.id),
                "every issue names the transaction it was raised against: {issues:?}"
            );
            issues
        }
    }
}

/// The half of the acceptance that closes a `validate` answering `Err` to everything.
///
/// It carries one of each operation shape that touches graph state, so that a validator refusing
/// a well-formed operation of any of those shapes turns this red.
#[test]
fn a_valid_transaction_validates() {
    let world = World::new();
    let fresh = NodeId::mint();
    let proposal = world.proposal(vec![
        GraphOperation::CreateNode(world.draft(fresh, "Adopt trybuild for the membrane")),
        GraphOperation::UpdateProperty(PropertyMutation {
            node: world.open,
            property: world.tags,
            values: vec![
                Value::String("phase-1".to_owned()),
                Value::String("kernel".to_owned()),
            ],
        }),
        GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id: world.root_id,
            type_id: world.depends_on,
            source: fresh,
            target: world.decided,
            properties: BTreeMap::new(),
        }),
        GraphOperation::DeleteEdge(world.existing_edge),
        GraphOperation::AddAssertion(Box::new(world.supported_assertion(AssertionId::mint()))),
        GraphOperation::RetractAssertion(world.held_assertion),
        GraphOperation::Invoke {
            node: world.open,
            operation: "decide".to_owned(),
            arguments: BTreeMap::new(),
        },
    ]);

    let validated = world
        .pipeline()
        .validate(&world.snapshot(), &proposal)
        .unwrap_or_else(|issues| panic!("a valid transaction was refused: {issues:?}"));

    assert_eq!(validated.validated_against(), RevisionNumber::new(7));
    assert_eq!(validated.transaction().id, proposal.id);
    assert_eq!(validated.transaction().operations.len(), 7);
}

/// Defect 1 of the acceptance: a reference that does not resolve.
#[test]
fn an_unresolvable_reference_is_refused_by_the_reference_validator() {
    let world = World::new();
    let issues = refuse(
        &world,
        vec![GraphOperation::UpdateProperty(PropertyMutation {
            node: NodeId::mint(),
            property: world.tags,
            values: vec![Value::String("phase-1".to_owned())],
        })],
    );

    assert_eq!(refusing_validators(&issues), vec![ValidatorName::Reference]);
    assert_eq!(codes(&issues), vec!["unresolved-node"]);
}

/// Defect 2 of the acceptance: a value that fails its declared type.
#[test]
fn a_value_that_fails_its_type_is_refused_by_the_type_validator() {
    let world = World::new();
    let issues = refuse(
        &world,
        vec![GraphOperation::UpdateProperty(PropertyMutation {
            node: world.open,
            property: world.title,
            values: vec![Value::Integer(7)],
        })],
    );

    assert_eq!(refusing_validators(&issues), vec![ValidatorName::Type]);
    assert_eq!(codes(&issues), vec!["wrong-type"]);
    assert!(
        issues[0].message.contains("String") && issues[0].message.contains("Integer"),
        "the issue names the type that was declared and the one that arrived: {}",
        issues[0].message
    );
}

/// Defect 3 of the acceptance: a cardinality the edge type forbids.
///
/// `depends_on` is `Cardinality::One`, the snapshot already holds one such edge out of the open
/// decision, and this proposes a second. The existing edge is read from canonical state, so a
/// validator that only looked at the transaction cannot pass this.
#[test]
fn an_edge_cardinality_the_type_forbids_is_refused_by_the_cardinality_validator() {
    let world = World::new();
    let issues = refuse(
        &world,
        vec![GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id: world.root_id,
            type_id: world.depends_on,
            source: world.open,
            target: world.decided,
            properties: BTreeMap::new(),
        })],
    );

    assert_eq!(
        refusing_validators(&issues),
        vec![ValidatorName::Cardinality]
    );
    assert_eq!(codes(&issues), vec!["edge-cardinality"]);
}

/// Defect 4 of the acceptance: an `Invoke` the lifecycle does not declare.
///
/// The decided decision is in `decided`; `decide` moves `open -> decided`, and the lifecycle
/// declares no move out of `decided` at all.
#[test]
fn an_invoke_the_lifecycle_does_not_declare_is_refused_by_the_ontology_validator() {
    let world = World::new();
    let issues = refuse(
        &world,
        vec![GraphOperation::Invoke {
            node: world.decided,
            operation: "decide".to_owned(),
            arguments: BTreeMap::new(),
        }],
    );

    assert_eq!(
        refusing_validators(&issues),
        vec![ValidatorName::OntologyConstraint]
    );
    assert_eq!(codes(&issues), vec!["transition-refused"]);
}

/// Defect 5 of the acceptance: a canonical assertion with no evidence.
///
/// Design § 6.5, and the P1 policy the story fixes: at least one `Evidence`, and `proposed_by`
/// alone is not evidence — so an assertion naming its proposer and nothing else is refused.
#[test]
fn a_canonical_assertion_without_evidence_is_refused_by_the_provenance_validator() {
    let world = World::new();
    let mut unsupported = world.supported_assertion(AssertionId::mint());
    unsupported.evidence = BTreeSet::new();

    let issues = refuse(
        &world,
        vec![GraphOperation::AddAssertion(Box::new(unsupported))],
    );

    assert_eq!(
        refusing_validators(&issues),
        vec![ValidatorName::Provenance]
    );
    assert_eq!(codes(&issues), vec!["assertion-without-evidence"]);
}

/// `architecture-decision-record:0005-float-is-not-canonical`, the kernel's half.
///
/// The ontology declares `tags` a `String`, so a float fails its declared type as well; the
/// property this case is about is the other one — the issue that names the value canonical state
/// does not admit, and where inside the value it sits.
#[test]
fn a_value_canonical_state_does_not_admit_is_refused_with_its_path() {
    let world = World::new();
    let issues = refuse(
        &world,
        vec![GraphOperation::UpdateProperty(PropertyMutation {
            node: world.open,
            property: world.tags,
            values: vec![Value::List(vec![
                Value::String("phase-1".to_owned()),
                Value::Record(
                    [("weight".to_owned(), Value::Float(0.5))]
                        .into_iter()
                        .collect(),
                ),
            ])],
        })],
    );

    assert_eq!(refusing_validators(&issues), vec![ValidatorName::Type]);
    assert!(
        codes(&issues).contains(&"inadmissible-value"),
        "the refusal names the defect: {issues:?}"
    );
    let named = issues
        .iter()
        .find(|issue| issue.code == "inadmissible-value")
        .expect("the inadmissible value is named");
    assert!(
        named.message.contains("value[1].weight"),
        "the issue names where inside the value it sits: {}",
        named.message
    );
    // And which operation it came from, which is what makes this a measurement of the *validator*
    // rather than of the conversion the pipeline does afterwards: both refuse a float and both
    // name the path, and only the validator knows the property it was carried under.
    assert!(
        named.message.contains(&world.tags.to_string()),
        "the issue names the property that carried it: {}",
        named.message
    );
}

/// Design § 6.10: the actor that proposes cannot be the sole basis of its own validation.
#[test]
fn a_transaction_whose_validator_is_its_proposer_is_refused() {
    let world = World::new();
    let proposal = world.proposal(vec![GraphOperation::CreateNode(
        world.draft(NodeId::mint(), "Adopt trybuild for the membrane"),
    )]);

    let issues = Pipeline::deterministic(world.proposer)
        .validate(&world.snapshot(), &proposal)
        .expect_err("a transaction cannot validate itself");

    assert_eq!(
        refusing_validators(&issues),
        vec![ValidatorName::Authorization]
    );
    assert_eq!(codes(&issues), vec!["proposer-is-validator"]);

    assert_eq!(
        issues[0],
        ValidationIssue {
            transaction_id: proposal.id,
            validator: ValidatorName::Authorization,
            code: "proposer-is-validator".to_owned(),
            message: issues[0].message.clone(),
        },
        "an issue is transaction, validator, code and message, and nothing else"
    );

    // The same proposal, validated by anyone else, passes: the refusal is about the actor.
    assert!(world
        .pipeline()
        .validate(&world.snapshot(), &proposal)
        .is_ok());
}

/// The story's third shipped case: one transaction, one snapshot, two runs, one hash.
#[test]
fn a_valid_transaction_validates_to_the_same_hash_twice() {
    let world = World::new();
    let proposal = world.proposal(vec![GraphOperation::CreateNode(
        world.draft(NodeId::mint(), "Adopt trybuild for the membrane"),
    )]);

    let first = world
        .pipeline()
        .validate(&world.snapshot(), &proposal)
        .expect("valid");
    let second = world
        .pipeline()
        .validate(&world.snapshot(), &proposal)
        .expect("valid");

    assert_eq!(first.validation_hash(), second.validation_hash());
}

/// And the other direction: the revision validated against is part of what was validated.
///
/// Two snapshots of the same state at two revisions. A hash that did not cover the revision would
/// let a transaction validated against revision 7 present itself as validated against 8, which is
/// the stale-commit check of design § 72 defeated at its own input.
#[test]
fn the_validation_hash_covers_the_revision_it_was_validated_against() {
    let world = World::new();
    let proposal = world.proposal(vec![GraphOperation::CreateNode(
        world.draft(NodeId::mint(), "Adopt trybuild for the membrane"),
    )]);
    let at_seven = world
        .pipeline()
        .validate(&world.snapshot(), &proposal)
        .expect("valid");

    // The same state, one revision on: a clone rather than a second `World`, so that the graph
    // root and every identity the proposal names are the ones this snapshot holds.
    let mut moved = world.graph.clone();
    moved.revision = RevisionNumber::new(8);
    let later = GraphSnapshot::of(&moved);
    let at_eight = world
        .pipeline()
        .validate(&later, &proposal)
        .expect("valid against the later revision too");

    assert_eq!(at_eight.validated_against(), RevisionNumber::new(8));
    assert_ne!(at_seven.validation_hash(), at_eight.validation_hash());
}

/// Two transactions that differ anywhere validate to different hashes.
///
/// One field of one operation, changed: a `validation_hash` that did not cover the operations
/// would collide here, and a transaction could be swapped for another after validation and before
/// commit. `every_field_of_every_encoded_type_reaches_its_encoding` is the general statement; this
/// is one instance of it, measured through the pipeline rather than through the encoding.
#[test]
fn two_transactions_that_differ_validate_to_different_hashes() {
    let world = World::new();
    let fresh = NodeId::mint();
    let one = world.proposal(vec![GraphOperation::CreateNode(
        world.draft(fresh, "Adopt trybuild for the membrane"),
    )]);
    let mut other = GraphTransaction {
        id: one.id,
        proposer: one.proposer,
        operations: one.operations.clone(),
        evidence: one.evidence.clone(),
    };
    let GraphOperation::CreateNode(draft) = &mut other.operations[0] else {
        panic!("the operation this case varies is the one it built");
    };
    draft.canonical_name = "Adopt trybuild for the membrane, twice".to_owned();

    let first = world
        .pipeline()
        .validate(&world.snapshot(), &one)
        .expect("valid");
    let second = world
        .pipeline()
        .validate(&world.snapshot(), &other)
        .expect("valid");

    assert_ne!(first.validation_hash(), second.validation_hash());
}

/// A proposal with no operations is refused before anything is looked up.
#[test]
fn a_transaction_with_no_operations_is_refused_by_the_structural_validator() {
    let world = World::new();
    let issues = refuse(&world, Vec::new());

    assert_eq!(
        refusing_validators(&issues),
        vec![ValidatorName::Structural]
    );
    assert_eq!(codes(&issues), vec!["empty-transaction"]);
}

/// One identity, created twice in one transaction, is not a well-formed transaction.
#[test]
fn an_identity_created_twice_in_one_transaction_is_refused() {
    let world = World::new();
    let twice = NodeId::mint();
    let issues = refuse(
        &world,
        vec![
            GraphOperation::CreateNode(world.draft(twice, "Adopt trybuild for the membrane")),
            GraphOperation::CreateNode(world.draft(twice, "Adopt trybuild for the membrane")),
        ],
    );

    assert_eq!(
        refusing_validators(&issues),
        vec![ValidatorName::Structural]
    );
    assert_eq!(codes(&issues), vec!["duplicate-identity"]);
}

/// An identity canonical state already holds is not created a second time.
///
/// The structural validator's other half. All three creation paths are here rather than the two an
/// adversary named, because the class is "an operation that brings an identity into existence" —
/// node, edge and assertion — and a rule enumerated by its instances is a rule with a next
/// instance.
#[test]
fn an_identity_canonical_state_already_holds_is_not_created_again() {
    let world = World::new();

    let node = refuse(
        &world,
        vec![GraphOperation::CreateNode(
            world.draft(world.open, "A different decision entirely"),
        )],
    );
    assert_eq!(refusing_validators(&node), vec![ValidatorName::Structural]);
    assert_eq!(codes(&node), vec!["identity-already-exists"]);

    let edge = refuse(
        &world,
        vec![GraphOperation::CreateEdge(EdgeDraft {
            id: world.existing_edge,
            root_id: world.root_id,
            type_id: world.depends_on,
            source: world.decided,
            target: world.open,
            properties: BTreeMap::new(),
        })],
    );
    assert_eq!(codes(&edge), vec!["identity-already-exists"]);

    let assertion = refuse(
        &world,
        vec![GraphOperation::AddAssertion(Box::new(
            world.supported_assertion(world.held_assertion),
        ))],
    );
    assert_eq!(codes(&assertion), vec!["identity-already-exists"]);

    // And the rule is about canonical state, not about the ids being unusual: the same three
    // operations over fresh ids are accepted.
    let fresh = NodeId::mint();
    assert!(world
        .pipeline()
        .validate(
            &world.snapshot(),
            &world.proposal(vec![
                GraphOperation::CreateNode(world.draft(fresh, "Adopt trybuild for the membrane")),
                GraphOperation::CreateEdge(EdgeDraft {
                    id: EdgeId::mint(),
                    root_id: world.root_id,
                    type_id: world.depends_on,
                    source: fresh,
                    target: world.open,
                    properties: BTreeMap::new(),
                }),
                GraphOperation::AddAssertion(Box::new(
                    world.supported_assertion(AssertionId::mint())
                )),
            ])
        )
        .is_ok());
}

/// A type the ontology already declares is not declared a second time.
///
/// The fourth and fifth members of the identity class, and the last two: `ekr.kernel.OperationKind`
/// has eleven operations, five of which bring an identifier into existence, and the match in
/// `Structural` is exhaustive so a twelfth cannot join them silently.
///
/// `TypeId` is one id space across both indexes, which is why a node type over an *edge* type's id
/// is refused too.
#[test]
fn a_type_the_ontology_already_declares_is_not_declared_again() {
    let world = World::new();

    let node_type = refuse(
        &world,
        vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            world.decision,
            "Something else entirely",
        )))],
    );
    assert_eq!(
        refusing_validators(&node_type),
        vec![ValidatorName::Structural]
    );
    assert_eq!(codes(&node_type), vec!["identity-already-exists"]);

    let edge_type = refuse(
        &world,
        vec![GraphOperation::DefineEdgeType(Box::new(EdgeType::new(
            world.depends_on,
            "something_else",
        )))],
    );
    assert_eq!(codes(&edge_type), vec!["identity-already-exists"]);

    // One id space: a node type cannot take an id the ontology holds as an edge type.
    let across = refuse(
        &world,
        vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            world.depends_on,
            "Decision",
        )))],
    );
    assert_eq!(codes(&across), vec!["identity-already-exists"]);

    // Twice in one transaction is the other half of the same rule.
    let fresh = TypeId::mint();
    let twice = refuse(
        &world,
        vec![
            GraphOperation::DefineNodeType(Box::new(NodeType::new(fresh, "Observation"))),
            GraphOperation::DefineEdgeType(Box::new(EdgeType::new(fresh, "observes"))),
        ],
    );
    assert_eq!(codes(&twice), vec!["duplicate-identity"]);

    // Schema evolution has no application semantics in P1, including fresh declarations.
    assert_eq!(
        codes(&refuse(
            &world,
            vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
                TypeId::mint(),
                "Observation"
            )))]
        )),
        vec!["unsupported-operation"]
    );
}

/// A proposal states a claim; it does not state the verdict on the claim.
///
/// Every state but `Proposed` is refused, not only the `Accepted` an adversary demonstrated: each
/// of the six is mintable by the same proposer in the same field, so a rule written against one
/// would be a rule with five ways round it.
#[test]
fn an_assertion_cannot_arrive_carrying_its_own_verdict() {
    let world = World::new();
    let states = [
        ValidationState::Validating {
            completed: 7,
            required: 7,
        },
        ValidationState::Accepted {
            validators: [world.reviewer].into_iter().collect(),
        },
        ValidationState::Rejected { issues: Vec::new() },
        ValidationState::Disputed {
            competing_assertions: Vec::new(),
        },
        ValidationState::Superseded {
            by: world.held_assertion,
        },
        ValidationState::Retracted {
            at_revision: RevisionNumber::new(7),
            reason: RetractionReason::new("the world moved on"),
        },
    ];

    for state in states {
        let mut forged = world.supported_assertion(AssertionId::mint());
        let named = state.name();
        forged.validation = state;
        let issues = refuse(&world, vec![GraphOperation::AddAssertion(Box::new(forged))]);
        assert_eq!(
            refusing_validators(&issues),
            vec![ValidatorName::Provenance],
            "{named} is refused by the provenance validator"
        );
        assert_eq!(codes(&issues), vec!["assertion-states-its-own-verdict"]);
        assert!(
            issues[0].message.contains(named),
            "the issue names the state that was claimed: {}",
            issues[0].message
        );
    }

    // `Proposed` is the one a proposal carries, and it validates.
    assert!(world
        .pipeline()
        .validate(
            &world.snapshot(),
            &world.proposal(vec![GraphOperation::AddAssertion(Box::new(
                world.supported_assertion(AssertionId::mint())
            ))])
        )
        .is_ok());
}

/// A node is not merged into itself.
#[test]
fn a_merge_names_two_nodes() {
    let world = World::new();
    let issues = refuse(
        &world,
        vec![GraphOperation::MergeEntity(EntityMerge {
            absorbed: world.open,
            into: world.open,
        })],
    );
    assert_eq!(
        refusing_validators(&issues),
        vec![ValidatorName::Structural]
    );
    assert_eq!(codes(&issues), vec!["merge-into-itself"]);

    // Even a well-formed merge must await the integration phase's application semantics.
    assert_eq!(
        codes(&refuse(
            &world,
            vec![GraphOperation::MergeEntity(EntityMerge {
                absorbed: world.open,
                into: world.decided,
            })]
        )),
        vec!["unsupported-operation"]
    );
}

/// An assertion's `TypeId`s are resolved, and its predicate is checked whatever its object is.
///
/// The second half is the one that matters: the object's shape used to decide whether the
/// predicate was looked at, so one undeclared property was refused against a value and accepted
/// against a node. One defect, two verdicts, decided by a field that was not the defect.
#[test]
fn an_assertions_types_are_resolved_whatever_shape_its_object_has() {
    let world = World::new();
    let undeclared_type = TypeId::mint();
    let undeclared_property = PropertyId::mint();

    let mut about_a_type = world.supported_assertion(AssertionId::mint());
    about_a_type.subject = Subject::Type(undeclared_type);
    let by_subject = refuse(
        &world,
        vec![GraphOperation::AddAssertion(Box::new(about_a_type))],
    );
    assert_eq!(codes(&by_subject), vec!["unknown-type"]);

    let mut to_a_type = world.supported_assertion(AssertionId::mint());
    to_a_type.object = Object::Type(undeclared_type);
    let by_object = refuse(
        &world,
        vec![GraphOperation::AddAssertion(Box::new(to_a_type))],
    );
    assert_eq!(codes(&by_object), vec!["unknown-type"]);

    let mut by_relation = world.supported_assertion(AssertionId::mint());
    by_relation.predicate = Predicate::Relation(undeclared_type);
    by_relation.object = Object::Node(world.decided);
    let by_predicate = refuse(
        &world,
        vec![GraphOperation::AddAssertion(Box::new(by_relation))],
    );
    assert_eq!(codes(&by_predicate), vec!["unknown-type"]);

    // One defect, two objects, one verdict — which is the point of this case.
    for object in [
        Object::Value(Value::String("one".to_owned())),
        Object::Node(world.decided),
        Object::Type(world.decision),
    ] {
        let mut undeclared = world.supported_assertion(AssertionId::mint());
        undeclared.predicate = Predicate::Property(undeclared_property);
        undeclared.object = object.clone();
        let issues = refuse(
            &world,
            vec![GraphOperation::AddAssertion(Box::new(undeclared))],
        );
        assert_eq!(
            codes(&issues),
            vec!["undeclared-property"],
            "the same undeclared property, with {object:?} as the object"
        );
    }

    // And a declared property against a node object is checked as the reference it is: `title` is
    // a String, so a node is the wrong kind for it.
    let mut wrong_kind = world.supported_assertion(AssertionId::mint());
    wrong_kind.object = Object::Node(world.decided);
    let issues = refuse(
        &world,
        vec![GraphOperation::AddAssertion(Box::new(wrong_kind))],
    );
    assert_eq!(codes(&issues), vec!["wrong-type"]);

    // A `supersedes` claim naming a Decision is what that property declares, and it validates.
    let mut supersedes = world.supported_assertion(AssertionId::mint());
    supersedes.predicate = Predicate::Property(world.supersedes);
    supersedes.object = Object::Node(world.decided);
    assert!(world
        .pipeline()
        .validate(
            &world.snapshot(),
            &world.proposal(vec![GraphOperation::AddAssertion(Box::new(supersedes))])
        )
        .is_ok());
}

/// A record names the graph root the snapshot reads, or it is refused.
///
/// `root_id` is the field that says which graph a record is even in, and all three of the records
/// an operation can carry have one.
#[test]
fn a_record_under_another_graph_root_is_refused() {
    let world = World::new();
    let elsewhere = GraphRootId::mint();

    let mut node = world.draft(NodeId::mint(), "Adopt trybuild for the membrane");
    node.root_id = elsewhere;
    let by_node = refuse(&world, vec![GraphOperation::CreateNode(node)]);
    assert_eq!(
        refusing_validators(&by_node),
        vec![ValidatorName::Reference]
    );
    assert_eq!(codes(&by_node), vec!["unresolved-graph-root"]);

    let by_edge = refuse(
        &world,
        vec![GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id: elsewhere,
            type_id: world.depends_on,
            source: world.decided,
            target: world.open,
            properties: BTreeMap::new(),
        })],
    );
    assert_eq!(codes(&by_edge), vec!["unresolved-graph-root"]);

    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.root_id = elsewhere;
    let by_assertion = refuse(
        &world,
        vec![GraphOperation::AddAssertion(Box::new(assertion))],
    );
    assert_eq!(codes(&by_assertion), vec!["unresolved-graph-root"]);
}

/// The declared evidence set is the evidence the transaction's assertions cite, and nothing else.
///
/// Both directions: an id declared and cited by nothing, and an id cited and not declared. The
/// second is the one that matters — `evidence_hash` addresses the declared set, so a transaction
/// whose assertions rest on evidence the declaration omits is stored claiming to rest on less
/// than it does.
#[test]
fn the_declared_evidence_set_is_the_evidence_the_assertions_cite() {
    let world = World::new();
    let supported =
        GraphOperation::AddAssertion(Box::new(world.supported_assertion(AssertionId::mint())));

    let undeclared = GraphTransaction {
        id: TransactionId::mint(),
        proposer: world.proposer,
        operations: vec![supported.clone()],
        evidence: BTreeSet::new(),
    };
    let issues = world
        .pipeline()
        .validate(&world.snapshot(), &undeclared)
        .expect_err("the declaration omits what the assertion rests on");
    assert_eq!(
        refusing_validators(&issues),
        vec![ValidatorName::Structural]
    );
    assert_eq!(codes(&issues), vec!["evidence-set-mismatch"]);

    let overdeclared = GraphTransaction {
        id: TransactionId::mint(),
        proposer: world.proposer,
        operations: vec![supported],
        evidence: [world.retained_evidence, EvidenceId::mint()]
            .into_iter()
            .collect(),
    };
    assert_eq!(
        codes(
            &world
                .pipeline()
                .validate(&world.snapshot(), &overdeclared)
                .expect_err("the declaration names evidence nothing rests on")
        ),
        vec!["evidence-set-mismatch"]
    );
}

/// An evidence id resolves against retained evidence, and the transaction's own list is not that.
///
/// The kernel-side sibling of `tests/adversary_membrane.rs`: there the case is that the proposal is
/// refused at all, here it is which validator refuses it and with what code. Provenance counts and
/// Reference resolves; an id the proposer minted is refused by Reference exactly once, however
/// many times the proposer wrote it down.
#[test]
fn an_evidence_id_the_transaction_declares_does_not_make_it_exist() {
    let world = World::new();
    let invented = EvidenceId::mint();
    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.evidence = [invented].into_iter().collect();

    let issues = refuse(
        &world,
        vec![GraphOperation::AddAssertion(Box::new(assertion))],
    );
    assert_eq!(refusing_validators(&issues), vec![ValidatorName::Reference]);
    assert_eq!(codes(&issues), vec!["unresolved-evidence"]);
    assert!(
        issues[0].message.contains(&invented.to_string()),
        "the refusal names the evidence that does not exist: {}",
        issues[0].message
    );
}

/// The rest of the reference validator's class, each member shown refused.
///
/// The acceptance names one unresolvable reference; a reference validator that resolved nodes and
/// nothing else would satisfy it. Every identity an operation can name is here.
#[test]
fn every_kind_of_dangling_reference_is_refused() {
    let world = World::new();
    let absent_node = NodeId::mint();

    let unresolved_edge = refuse(&world, vec![GraphOperation::DeleteEdge(EdgeId::mint())]);
    assert_eq!(codes(&unresolved_edge), vec!["unresolved-edge"]);

    let unresolved_assertion = refuse(
        &world,
        vec![GraphOperation::RetractAssertion(AssertionId::mint())],
    );
    assert_eq!(codes(&unresolved_assertion), vec!["unresolved-assertion"]);

    let mut uncited = world.supported_assertion(AssertionId::mint());
    uncited.evidence = [EvidenceId::mint()].into_iter().collect();
    let unresolved_evidence = refuse(
        &world,
        vec![GraphOperation::AddAssertion(Box::new(uncited))],
    );
    assert_eq!(codes(&unresolved_evidence), vec!["unresolved-evidence"]);

    let unresolved_edge_endpoint = refuse(
        &world,
        vec![GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id: world.root_id,
            type_id: world.depends_on,
            source: absent_node,
            target: world.decided,
            properties: BTreeMap::new(),
        })],
    );
    assert_eq!(codes(&unresolved_edge_endpoint), vec!["unresolved-node"]);

    let unresolved_merge = refuse(
        &world,
        vec![GraphOperation::MergeEntity(EntityMerge {
            absorbed: absent_node,
            into: world.open,
        })],
    );
    assert_eq!(codes(&unresolved_merge), vec!["unresolved-node"]);

    let unresolved_invoke = refuse(
        &world,
        vec![GraphOperation::Invoke {
            node: absent_node,
            operation: "decide".to_owned(),
            arguments: BTreeMap::new(),
        }],
    );
    assert_eq!(codes(&unresolved_invoke), vec!["unresolved-node"]);

    // A reference *inside* a value is a reference: a `NodeRef` to a node that does not exist.
    let unresolved_value = refuse(
        &world,
        vec![GraphOperation::UpdateProperty(PropertyMutation {
            node: world.open,
            property: world.supersedes,
            values: vec![Value::NodeRef(absent_node)],
        })],
    );
    assert_eq!(codes(&unresolved_value), vec!["unresolved-node"]);
}

/// A reference created earlier in the same transaction resolves.
///
/// Without this, every multi-operation proposal would be refused, and the reference validator
/// would be a validator nothing can pass.
#[test]
fn a_reference_to_something_the_same_transaction_creates_resolves() {
    let world = World::new();
    let fresh = NodeId::mint();
    let fresh_edge = EdgeId::mint();
    let proposal = world.proposal(vec![
        GraphOperation::CreateNode(world.draft(fresh, "Adopt trybuild for the membrane")),
        GraphOperation::CreateEdge(EdgeDraft {
            id: fresh_edge,
            root_id: world.root_id,
            type_id: world.depends_on,
            source: fresh,
            target: world.decided,
            properties: BTreeMap::new(),
        }),
        GraphOperation::DeleteEdge(fresh_edge),
    ]);

    assert!(world
        .pipeline()
        .validate(&world.snapshot(), &proposal)
        .is_ok());
}

/// The rest of the type validator's class.
#[test]
fn every_type_refusal_the_validator_can_make_is_reachable() {
    let world = World::new();

    let undeclared = refuse(
        &world,
        vec![GraphOperation::UpdateProperty(PropertyMutation {
            node: world.open,
            property: PropertyId::mint(),
            values: vec![Value::String("phase-1".to_owned())],
        })],
    );
    assert_eq!(codes(&undeclared), vec!["undeclared-property"]);

    let unknown_type = refuse(
        &world,
        vec![GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id: world.root_id,
            type_id: TypeId::mint(),
            canonical_name: "A decision of no declared type".to_owned(),
            properties: BTreeMap::new(),
        })],
    );
    assert_eq!(codes(&unknown_type), vec!["unknown-type"]);

    let unknown_edge_type = refuse(
        &world,
        vec![GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id: world.root_id,
            type_id: TypeId::mint(),
            source: world.open,
            target: world.decided,
            properties: BTreeMap::new(),
        })],
    );
    assert_eq!(codes(&unknown_edge_type), vec!["unknown-type"]);
}

/// A type declared abstract has no instances, and a draft claiming one is refused.
#[test]
fn a_node_of_an_abstract_type_is_refused() {
    let mut world = World::new();
    let record = TypeId::mint();
    let mut abstract_type = NodeType::new(record, "Record");
    abstract_type.abstract_type = true;
    let mut concrete = NodeType::new(world.decision, "Decision");
    concrete.parents = [record].into_iter().collect();
    world.graph.ontology = Ontology::load(OntologyDocument {
        version: world.graph.ontology.version().clone(),
        node_types: vec![abstract_type, concrete],
        edge_types: Vec::new(),
    })
    .expect("an abstract parent and a concrete child cohere");

    let issues = refuse(
        &world,
        vec![GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id: world.root_id,
            type_id: record,
            canonical_name: "A record of nothing in particular".to_owned(),
            properties: BTreeMap::new(),
        })],
    );
    assert_eq!(refusing_validators(&issues), vec![ValidatorName::Type]);
    assert_eq!(codes(&issues), vec!["abstract-type"]);
}

/// An edge type declares which types it may run between, and an endpoint outside them is refused.
#[test]
fn an_edge_endpoint_of_the_wrong_type_is_refused() {
    let mut world = World::new();
    let observation = TypeId::mint();
    let mut depends_on = EdgeType::new(world.depends_on, "depends_on");
    depends_on.source_types = [observation].into_iter().collect();
    depends_on.target_types = [world.decision].into_iter().collect();
    depends_on.cardinality = Cardinality::Many;
    world.graph.ontology = Ontology::load(OntologyDocument {
        version: world.graph.ontology.version().clone(),
        node_types: vec![
            NodeType::new(world.decision, "Decision"),
            NodeType::new(observation, "Observation"),
        ],
        edge_types: vec![depends_on],
    })
    .expect("two node types and an edge between them cohere");

    let issues = refuse(
        &world,
        vec![GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id: world.root_id,
            type_id: world.depends_on,
            source: world.open,
            target: world.decided,
            properties: BTreeMap::new(),
        })],
    );
    assert_eq!(refusing_validators(&issues), vec![ValidatorName::Type]);
    assert_eq!(codes(&issues), vec!["edge-endpoint-type"]);
}

/// An operation's arguments are typed, and both directions of that are refused: an argument the
/// operation does not declare, and one it declares that the invocation does not carry.
#[test]
fn an_invocation_carries_exactly_the_arguments_its_operation_declares() {
    let mut world = World::new();
    let mut decision_type = NodeType::new(world.decision, "Decision");
    let mut required_title = PropertyDefinition::new(world.title, "title", ValueType::String);
    required_title.required = true;
    decision_type.properties.insert(world.title, required_title);
    decision_type.lifecycle = Some(Lifecycle {
        initial: "open".to_owned(),
        states: ["open", "decided"].into_iter().map(str::to_owned).collect(),
        transitions: [Transition::new("open", "decided")].into_iter().collect(),
    });
    let mut decide = OperationDefinition::new("decide");
    decide.transition = Some(Transition::new("open", "decided"));
    decide
        .arguments
        .insert("rationale".to_owned(), ValueType::String);
    decision_type.operations.insert("decide".to_owned(), decide);
    world.graph.ontology = Ontology::load(OntologyDocument {
        version: world.graph.ontology.version().clone(),
        node_types: vec![decision_type],
        edge_types: Vec::new(),
    })
    .expect("an operation with a typed argument coheres");

    let undeclared = refuse(
        &world,
        vec![GraphOperation::Invoke {
            node: world.open,
            operation: "decide".to_owned(),
            arguments: [
                ("rationale".to_owned(), Value::String("measured".to_owned())),
                ("haste".to_owned(), Value::Boolean(true)),
            ]
            .into_iter()
            .collect(),
        }],
    );
    assert_eq!(codes(&undeclared), vec!["undeclared-argument"]);

    let missing = refuse(
        &world,
        vec![GraphOperation::Invoke {
            node: world.open,
            operation: "decide".to_owned(),
            arguments: BTreeMap::new(),
        }],
    );
    assert_eq!(codes(&missing), vec!["missing-argument"]);

    let wrong = refuse(
        &world,
        vec![GraphOperation::Invoke {
            node: world.open,
            operation: "decide".to_owned(),
            arguments: [("rationale".to_owned(), Value::Integer(7))]
                .into_iter()
                .collect(),
        }],
    );
    assert_eq!(codes(&wrong), vec!["wrong-type"]);
}

/// The seven validators of design § 20, in the order the design lists them.
///
/// The pipeline's order is data, and which validator refused is what every case above asserts
/// against, so the list is pinned rather than read back from the thing under test.
///
/// **Which half this carries:** the transcription, and only that. It holds
/// `Pipeline::deterministic` to a list written here, so a validator inserted, dropped or moved
/// turns it red. It does *not* read `ekr.kernel.ValidatorName` out of the domain, so the domain
/// and this list could drift apart together — the same gap
/// [`the_eleven_operation_numbers_are_the_domains_and_the_declarations`] closes for
/// `OperationKind` by reading the document. The difference is that a validator's position is not
/// in any content address, so drift here costs a wrong order and not a moved hash.
#[test]
fn the_pipeline_runs_the_seven_deterministic_validators_in_order() {
    let world = World::new();
    assert_eq!(
        world.pipeline().validators(),
        vec![
            ValidatorName::Structural,
            ValidatorName::Reference,
            ValidatorName::Type,
            ValidatorName::Cardinality,
            ValidatorName::OntologyConstraint,
            ValidatorName::Provenance,
            ValidatorName::Authorization,
        ]
    );

    // Each names itself, so an issue's `validator` field cannot drift from the module that raised
    // it without this going red.
    assert_eq!(Structural.name(), ValidatorName::Structural);
    assert_eq!(Reference.name(), ValidatorName::Reference);
    assert_eq!(Types.name(), ValidatorName::Type);
    assert_eq!(CardinalityValidator.name(), ValidatorName::Cardinality);
    assert_eq!(OntologyConstraint.name(), ValidatorName::OntologyConstraint);
    assert_eq!(Provenance.name(), ValidatorName::Provenance);
    assert_eq!(
        Authorization {
            actor: world.reviewer
        }
        .name(),
        ValidatorName::Authorization
    );
}

/// The rest of the cardinality validator's class: a property is a count too.
#[test]
fn property_cardinality_and_required_presence_are_refused() {
    let world = World::new();

    let too_many = refuse(
        &world,
        vec![GraphOperation::UpdateProperty(PropertyMutation {
            node: world.open,
            property: world.title,
            values: vec![
                Value::String("one title".to_owned()),
                Value::String("another title".to_owned()),
            ],
        })],
    );
    assert_eq!(codes(&too_many), vec!["property-cardinality"]);

    let missing = refuse(
        &world,
        vec![GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id: world.root_id,
            type_id: world.decision,
            canonical_name: "A decision with no title".to_owned(),
            properties: BTreeMap::new(),
        })],
    );
    assert_eq!(codes(&missing), vec!["missing-required-property"]);
}

/// The rest of the ontology-constraint validator's class: an operation the type does not declare.
#[test]
fn an_operation_the_type_does_not_declare_is_refused() {
    let world = World::new();
    let issues = refuse(
        &world,
        vec![GraphOperation::Invoke {
            node: world.open,
            operation: "reconsider".to_owned(),
            arguments: BTreeMap::new(),
        }],
    );

    assert_eq!(
        refusing_validators(&issues),
        vec![ValidatorName::OntologyConstraint]
    );
    assert_eq!(codes(&issues), vec!["operation-not-declared"]);
}

/// Every code the kernel can raise is raised by a case in this suite.
///
/// **What it covers and nothing wider:** the `const` code literals declared in
/// `src/validate/*.rs`, each of which must appear as a quoted string in this crate's `tests/`
/// directory. It does not measure that the case which names a code asserts anything useful about
/// it, and it says nothing about codes assembled at run time — there are none, and a code that
/// stopped being a `const` literal would leave this guard silently.
///
/// It exists because a hand-kept list of codes is the defect and a missing entry is only its
/// symptom: a validator that grows an eighth refusal nobody wrote a case for turns this red
/// without anyone extending anything.
#[test]
fn every_issue_code_the_kernel_can_raise_is_raised_by_a_case() {
    let sources = kernel_sources("src/validate");
    let suite = kernel_sources("tests");

    let mut declared: Vec<String> = sources
        .iter()
        .flat_map(|text| text.lines())
        .filter_map(code_literal)
        .collect();
    declared.sort();
    declared.dedup();
    assert!(
        declared.len() >= 7,
        "the code scan is broken, not the validators: {declared:?}"
    );

    let cases: String = suite.concat();
    let unexercised: Vec<&String> = declared
        .iter()
        .filter(|code| !cases.contains(&format!("\"{code}\"")))
        .collect();
    assert!(
        unexercised.is_empty(),
        "issue codes no case names: {unexercised:?}"
    );
}

/// The eleven `GraphOperation` numbers are the domain's list, the declaration order and the
/// encoding's, all three.
///
/// `Encoder::variant`'s own doc puts the obligation here and names the precedent:
///
/// > `index` identifies the variant, and it is part of the contract: **changing a variant's number
/// > moves every content address that contains it.** […] a type whose numbering and whose variant
/// > list disagree is a type with a silent defect rather than a compile error. The obligation is on
/// > the implementor to pin its own mapping; `crates/ekr-graph/tests/revision_events.rs` does that
/// > for `RevisionEvent`, both by transcribing the six numbers and by reading its source for the
/// > declaration order.
///
/// Nothing pinned the eleven, and no other mechanism can: the mechanism table above has a row for
/// a renumbering with four silent cells. A renumbering is invisible to a hash-separation property
/// (renumbering is injective), to a field-order probe (the marker moves with the variant), and to
/// both declaration scans (the numbers are literals in a `match`).
///
/// Three lists, held equal, the way the precedent holds three:
///
/// 1. `ekr.kernel.OperationKind` of `systems/ekr/domains/kernel.yaml`, read at run time;
/// 2. the declaration order of `GraphOperation` in `src/transaction.rs`, read as text;
/// 3. the `out.variant(N)` sequence in that file's `encode`, read as text — the half that can move
///    independently of the declaration, which is exactly the silent defect the encoder warns of.
///
/// And then the numbers themselves, transcribed here and checked against the bytes, so that a
/// renumbering which moved all three lists together still turns this red.
#[test]
fn the_eleven_operation_numbers_are_the_domains_and_the_declarations() {
    /// The eleven names in the order their numbers count in, transcribed from
    /// `ekr.kernel.OperationKind`.
    const NAMES: [&str; 11] = [
        "CreateNode",
        "UpdateProperty",
        "CreateEdge",
        "DeleteEdge",
        "AddAssertion",
        "RetractAssertion",
        "DefineNodeType",
        "DefineEdgeType",
        "ModifyProperty",
        "MergeEntity",
        "Invoke",
    ];

    let domain = read_workspace_file("systems/ekr/domains/kernel.yaml");
    let declared_in_domain: Vec<String> = domain
        .split_once("name: ekr.kernel.OperationKind")
        .expect("the domain declares OperationKind")
        .1
        .split_once("variants:")
        .expect("the enumeration has variants")
        .1
        .lines()
        .skip(1)
        .take_while(|line| line.trim_start().starts_with("- "))
        .map(|line| line.trim().trim_start_matches("- ").to_owned())
        .collect();
    assert_eq!(
        declared_in_domain,
        NAMES.to_vec(),
        "the domain's OperationKind is not what this case transcribes"
    );

    let source = read_workspace_file("crates/ekr-kernel/src/transaction.rs");
    // The declaration is found by its shape and not by its exact generics: it was written
    // `pub enum GraphOperation<V = Value> {` and ADR 0008 made it
    // `pub enum GraphOperation<V: ValueSpace = Value> {`, at which point a scan spelling the
    // parameter out reported the enum as undeclared rather than as having changed.
    let declaration = source
        .lines()
        .find(|line| line.starts_with("pub enum GraphOperation") && line.trim_end().ends_with('{'))
        .expect("the crate declares GraphOperation");
    let body = source
        .split_once(declaration)
        .expect("the declaration is in the source it came from")
        .1;
    // A variant head is a line at one level of indentation inside the enum, in any of the three
    // forms the enum uses: `Name(`, `Name {` and a bare `Name,`. `rustfmt` runs in the gate, so
    // the one-head-per-line shape is not an assumption about anybody's style.
    let declared_in_source: Vec<String> = body
        .lines()
        .take_while(|line| *line != "}")
        .filter(|line| line.starts_with("    ") && !line.starts_with("     "))
        .filter_map(|line| {
            let head = line.trim();
            let name: String = head.chars().take_while(|c| c.is_alphanumeric()).collect();
            // What follows the identifier is what makes it a variant head rather than a statement:
            // `(` for a tuple variant, ` {` for a struct one, `,` for a unit one.
            let rest = &head[name.len()..];
            (!name.is_empty()
                && name.starts_with(char::is_uppercase)
                && (rest.starts_with('(') || rest.starts_with(" {") || rest == ","))
                .then_some(name)
        })
        .collect();
    assert_eq!(
        declared_in_source, declared_in_domain,
        "the enum's declaration order is not the domain's order"
    );

    // The other half: the arms of `GraphOperation`'s own `encode`, each `Self::Name` followed by
    // its `out.variant(N)`. Anchored on the impl block and closed at the end of it, so that the
    // other five `Canonical` implementations in this file — one of which writes a variant marker
    // of its own — are outside the scan.
    // Anchored by the shape of the head rather than by its exact bounds, for the reason the
    // declaration above is: the bounds moved once already.
    let head = source
        .lines()
        .find(|line| {
            line.starts_with("impl") && line.contains(" Canonical for GraphOperation<V> {")
        })
        .expect("GraphOperation implements Canonical");
    let encode = source
        .split_once(head)
        .expect("the head is in the source it came from")
        .1
        .split_once("\n}")
        .expect("the impl block closes")
        .0;
    let mut numbered: Vec<(u32, String)> = Vec::new();
    let mut pending: Option<String> = None;
    for line in encode.lines() {
        let line = line.trim();
        if let Some(arm) = line.strip_prefix("Self::") {
            pending = Some(arm.chars().take_while(|c| c.is_alphanumeric()).collect());
        } else if let Some(number) = line.strip_prefix("out.variant(") {
            let index: u32 = number
                .trim_end_matches(");")
                .parse()
                .expect("a variant index is a number");
            if let Some(name) = pending.take() {
                numbered.push((index, name));
            }
        }
    }
    assert_eq!(
        numbered.len(),
        NAMES.len(),
        "the encode scan found {numbered:?}, which is not eleven arms"
    );
    for (position, (index, name)) in numbered.iter().enumerate() {
        let expected = u32::try_from(position).expect("eleven variants fit in a u32");
        assert_eq!(
            (*index, name.as_str()),
            (expected, NAMES[position]),
            "the encoding numbers {name} as {index}; renumbering moves every validation_hash that \
             contains one"
        );
    }

    // And the bytes, so that all three lists moving together is still caught. The marker is the
    // tag byte and the index, big-endian, and it opens the encoding.
    let world = World::new();
    for (position, operation) in one_of_each_operation(&world).iter().enumerate() {
        let index = u32::try_from(position).expect("eleven variants fit in a u32");
        let mut expected = vec![0x0f_u8];
        expected.extend_from_slice(&index.to_be_bytes());
        assert_eq!(
            &operation.canonical_bytes()[..5],
            &expected[..],
            "{} does not open with variant marker {index}",
            NAMES[position]
        );
    }
}

/// One operation of each variant, in the order the eleven numbers count in.
fn one_of_each_operation(world: &World) -> Vec<GraphOperation<CanonicalValue>> {
    let node = NodeId::mint();
    vec![
        GraphOperation::CreateNode(NodeDraft {
            id: node,
            root_id: world.root_id,
            type_id: world.decision,
            canonical_name: "one".to_owned(),
            properties: BTreeMap::new(),
        }),
        GraphOperation::UpdateProperty(PropertyMutation {
            node,
            property: world.title,
            values: Vec::new(),
        }),
        GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id: world.root_id,
            type_id: world.depends_on,
            source: node,
            target: node,
            properties: BTreeMap::new(),
        }),
        GraphOperation::DeleteEdge(EdgeId::mint()),
        GraphOperation::AddAssertion(Box::new(Assertion {
            id: AssertionId::mint(),
            root_id: world.root_id,
            subject: Subject::Node(CanonicalRef::new(node)),
            predicate: Predicate::Property(world.title),
            object: Object::Value(CanonicalValue::String("one".to_owned())),
            evidence: BTreeSet::new(),
            proposed_by: world.proposer,
            validation: ValidationState::Proposed,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        })),
        GraphOperation::RetractAssertion(AssertionId::mint()),
        GraphOperation::DefineNodeType(Box::new(NodeType::new(TypeId::mint(), "Decision"))),
        GraphOperation::DefineEdgeType(Box::new(EdgeType::new(TypeId::mint(), "depends_on"))),
        GraphOperation::ModifyProperty(PropertyDefinition::new(
            world.title,
            "title",
            ValueType::String,
        )),
        GraphOperation::MergeEntity(EntityMerge {
            absorbed: node,
            into: world.open,
        }),
        GraphOperation::Invoke {
            node,
            operation: "decide".to_owned(),
            arguments: BTreeMap::new(),
        },
    ]
}

/// The encoding writes a struct's id-bearing fields in declaration order.
///
/// **The third mechanism, and it exists because the other two cannot see this.** A field swapped
/// with another in an `encode` body is still *named* there, so
/// `every_field_of_every_encoded_type_reaches_its_encoding` passes; and swapping two writes
/// permutes the encoding, which leaves it injective, so
/// `the_encoding_separates_transactions_that_differ` passes too. Measured, not assumed: with
/// `source_types` and `target_types` swapped in `DeclaredEdgeType::encode`, all 40 cases of this
/// crate were green.
///
/// What a swap does move is every address the type has ever reached, which is why the field order
/// is written into each `encode`'s doc as the contract. This reads the bytes back: an id is
/// sixteen big-endian bytes of UUID and appears nowhere else in the encoding, so the position of
/// each id in the output is observable, and the positions must ascend in declaration order.
///
/// **What it covers and nothing wider:** the structs listed below, and within each only the fields
/// that carry a distinct id. A `bool`, a `String` or a count is not located this way.
///
/// # The bound this used to state was false
///
/// It said: "pinning the whole layout needs a stored byte vector over fixed ids, and an id is only
/// constructible here by minting […] which the story forbids adding". Adversary pass 2 measured
/// that and it is wrong — hold every other field constant, vary one, and the offset of the first
/// differing byte is where that field is written; no fixed id and no stored vector are needed. It
/// wrote the counter-example over seven types and thirty-four fields, including every field this
/// guard's bound named as unreachable, and it is adopted at `tests/encoding_field_order.rs`.
///
/// This one stays as the narrower half of the pair, and the difference is what it reads: this
/// locates a field in **one** encoding, where the probe compares **two** and so depends on the two
/// values it varies between encoding differently. Neither is the other's duplicate, and this one's
/// message names the offsets directly.
#[test]
fn the_encoding_writes_id_bearing_fields_in_declaration_order() {
    let root_id = GraphRootId::mint();
    let (type_id, source_type, target_type, inverse) = (
        TypeId::mint(),
        TypeId::mint(),
        TypeId::mint(),
        TypeId::mint(),
    );
    let (node, edge, source, target) = (
        NodeId::mint(),
        EdgeId::mint(),
        NodeId::mint(),
        NodeId::mint(),
    );

    let mut declared = EdgeType::new(type_id, "depends_on");
    declared.source_types = [source_type].into_iter().collect();
    declared.target_types = [target_type].into_iter().collect();
    declared.inverse = Some(inverse);
    let edge_type = GraphOperation::<CanonicalValue>::DefineEdgeType(Box::new(declared));

    let node_draft = GraphOperation::CreateNode(NodeDraft::<CanonicalValue> {
        id: node,
        root_id,
        type_id,
        canonical_name: "Adopt trybuild for the membrane".to_owned(),
        properties: BTreeMap::new(),
    });

    let edge_draft = GraphOperation::CreateEdge(EdgeDraft::<CanonicalValue> {
        id: edge,
        root_id,
        type_id,
        source,
        target,
        properties: BTreeMap::new(),
    });

    // Each row is one encoding and the ids its fields carry, in the order the `encode` body
    // documents. `EdgeType` is first because it is the one that was written in the wrong order
    // while every other case in the crate stayed green.
    for (what, operation, expected) in [
        (
            "EdgeType: id, source_types, target_types, inverse",
            &edge_type,
            vec![
                type_id.as_u128(),
                source_type.as_u128(),
                target_type.as_u128(),
                inverse.as_u128(),
            ],
        ),
        (
            "NodeDraft: id, root_id, type_id",
            &node_draft,
            vec![node.as_u128(), root_id.as_u128(), type_id.as_u128()],
        ),
        (
            "EdgeDraft: id, root_id, type_id, source, target",
            &edge_draft,
            vec![
                edge.as_u128(),
                root_id.as_u128(),
                type_id.as_u128(),
                source.as_u128(),
                target.as_u128(),
            ],
        ),
    ] {
        let bytes = operation.canonical_bytes();
        let found: Vec<usize> = expected
            .iter()
            .map(|id| {
                let needle = id.to_be_bytes();
                bytes
                    .windows(needle.len())
                    .position(|window| window == needle)
                    .unwrap_or_else(|| panic!("{what}: an id the encoding must carry is absent"))
            })
            .collect();
        let mut ascending = found.clone();
        ascending.sort_unstable();
        assert_eq!(
            found, ascending,
            "{what}: the ids appear at {found:?}, which is not declaration order"
        );
    }
}

/// Every field of every type the kernel encodes reaches that type's encoding.
///
/// **What it covers and nothing wider:** the `pub` fields declared by the structs listed in
/// [`ENCODED`], each of which must be named in the same file's encoding text. A struct absent from
/// that list is not checked, a field that is named but written in the wrong order is not caught,
/// and an enum's variants are not read at all.
///
/// It is here because the alternative is a mutation table — a row per field, varying one and
/// asserting the hash moved — and a table has to be extended by hand when a field arrives. This
/// does not: a field added to `NodeDraft`, or to `ekr_ontology::NodeType` upstream, and left out
/// of the kernel's encoding turns this red in the change that added it. A field that never reaches
/// the encoding is a field two different transactions can disagree about under one
/// `validation_hash`.
///
/// # Which half this carries, and which half the property carries
///
/// Two mechanisms cover the encoding and each is blind where the other sees:
///
/// **Four** mechanisms cover the encoding and each is blind where the others see. The columns are
/// this guard; `the_encoding_separates_transactions_that_differ` in `tests/validate_properties.rs`;
/// [`the_encoding_writes_id_bearing_fields_in_declaration_order`]; and
/// `every_field_of_the_kernels_encodings_is_written_in_declaration_order` in
/// `tests/encoding_field_order.rs`, which adversary pass 2 wrote and which covers the most.
///
/// | | this guard | hash separation | id offsets | the field-order probe |
/// |---|---|---|---|---|
/// | a field left out of an `encode` | **red**, from the declaration | red only if two generated values differ in exactly that field | silent unless the field carries an id | **red**, for all 34 |
/// | two fields written in the wrong order | silent — both are still *named* | **silent** | red if both carry ids | **red**, whatever their type |
/// | a variant tag reused between two variants | silent | **red** | silent | silent |
/// | a variant renumbered | silent | silent | silent | silent — [`the_eleven_operation_numbers_are_the_domains_and_the_declarations`] carries it |
/// | a type nobody generates | **red** if a field is missing | silent — which is what hid `DefineEdgeType` until its strategy arrived | red if it is in the list | **red** if it is in the list |
///
/// Two cells were measured rather than assumed, and both refuted a stated bound. The hash-property
/// cell in row two: swapping `source_types` and `target_types` in `DeclaredEdgeType::encode` left
/// all 40 cases of this crate green, because a permutation of an injective encoding is still
/// injective. And the whole of the last column, against this file's claim that it could not be
/// built without a forbidden dependency.
///
/// So: this one holds the encoding to the *declarations*, hash separation holds it to
/// *injectivity*, and the two order mechanisms hold it to its *layout*. The property is only as
/// wide as its generator, which is why that generator carries all eleven operation variants.
#[test]
fn every_field_of_every_encoded_type_reaches_its_encoding() {
    let transaction = read_workspace_file("crates/ekr-kernel/src/transaction.rs");
    let ontology_types = read_workspace_file("crates/ekr-ontology/src/types.rs");
    let ontology_lifecycle = read_workspace_file("crates/ekr-ontology/src/lifecycle.rs");
    let encoding = encoding_bodies(&transaction);
    assert!(
        encoding.len() > 500,
        "the encoding scan found {} bytes of `fn encode`, which is not an encoding",
        encoding.len()
    );

    let mut missing: Vec<String> = Vec::new();
    let mut checked = 0usize;
    for (struct_name, declared_in) in ENCODED {
        let source = match declared_in {
            "transaction" => &transaction,
            "ontology-types" => &ontology_types,
            "ontology-lifecycle" => &ontology_lifecycle,
            other => panic!("no such source: {other}"),
        };
        let fields = public_fields(source, struct_name);
        assert!(
            !fields.is_empty(),
            "{struct_name} was not found where it is declared, or has no fields"
        );
        for field in fields {
            checked += 1;
            if !encoding.contains(&field) {
                missing.push(format!("{struct_name}.{field}"));
            }
        }
    }
    assert!(checked > 30, "the field scan is broken: {checked} fields");
    assert!(
        missing.is_empty(),
        "fields the kernel's encoding never names: {missing:?}"
    );
}

/// Every struct whose fields the kernel's `validation_hash` is computed over, and the file each is
/// declared in. The upstream ones are here because the kernel encodes them by hand: they are
/// `ekr_ontology`'s, and a foreign trait cannot be implemented for a foreign type.
const ENCODED: [(&str, &str); 11] = [
    ("GraphTransaction", "transaction"),
    ("NodeDraft", "transaction"),
    ("EdgeDraft", "transaction"),
    ("PropertyMutation", "transaction"),
    ("EntityMerge", "transaction"),
    ("PropertyDefinition", "ontology-types"),
    ("NodeType", "ontology-types"),
    ("EdgeType", "ontology-types"),
    ("Lifecycle", "ontology-lifecycle"),
    ("Transition", "ontology-lifecycle"),
    ("OperationDefinition", "ontology-lifecycle"),
];

/// The body of every `fn encode…` in `source`, concatenated — the text a field must be named in.
///
/// Delimited by counting braces from the one that opens the function, so a declaration elsewhere
/// in the file is not mistaken for a use in the encoding.
fn encoding_bodies(source: &str) -> String {
    let mut bodies = String::new();
    let mut from = 0;
    while let Some(offset) = source[from..].find("fn encode") {
        let at = from + offset;
        from = at + 1;
        let Some(open) = source[at..].find('{') else {
            continue;
        };
        let start = at + open;
        let mut depth = 0usize;
        for (index, character) in source[start..].char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        bodies.push_str(&source[start..=start + index]);
                        from = start + index;
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    bodies
}

/// The `pub` field names a struct declares, read from its source text.
fn public_fields(source: &str, struct_name: &str) -> Vec<String> {
    let opening = format!("pub struct {struct_name}");
    let Some(at) = source.find(&opening) else {
        return Vec::new();
    };
    let body = &source[at..];
    let Some(end) = body.find("\n}") else {
        return Vec::new();
    };
    body[..end]
        .lines()
        .filter_map(|line| line.trim().strip_prefix("pub "))
        .filter_map(|rest| rest.split(':').next())
        .filter(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
        .map(str::to_owned)
        .collect()
}

/// The `const … : &str = "code";` literal a line declares, if it declares one — at any
/// visibility, so that making a code `pub(super)` does not take it out of the scan.
fn code_literal(line: &str) -> Option<String> {
    let line = line.trim();
    let declaration = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub(super) "))
        .or_else(|| line.strip_prefix("pub "))
        .unwrap_or(line);
    if !declaration.starts_with("const ") || !declaration.contains("&str") {
        return None;
    }
    let (_, rest) = declaration.split_once('"')?;
    let (code, _) = rest.split_once('"')?;
    Some(code.to_owned())
}

/// Every `.rs` file at or below `crates/ekr-kernel/<relative>`, read.
fn kernel_sources(relative: &str) -> Vec<String> {
    fn walk(directory: &std::path::Path, into: &mut Vec<String>) {
        let entries = std::fs::read_dir(directory)
            .unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()));
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|e| e == "rs") {
                into.push(
                    std::fs::read_to_string(&path)
                        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display())),
                );
            }
        }
    }
    let mut found = Vec::new();
    walk(
        &std::path::PathBuf::from(
            std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets the manifest directory"),
        )
        .join(relative),
        &mut found,
    );
    assert!(!found.is_empty(), "no sources under {relative}");
    found
}

/// A file read relative to the workspace root.
fn read_workspace_file(relative: &str) -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets the manifest directory");
    let root = std::path::Path::new(&manifest)
        .ancestors()
        .nth(2)
        .expect("crates/ekr-kernel sits two levels below the workspace root")
        .join(relative);
    std::fs::read_to_string(&root).unwrap_or_else(|e| panic!("reading {}: {e}", root.display()))
}

/// AGENTS.md invariant 1, as a build failure rather than a comment.
///
/// The directory is globbed, so a case added there runs without this file changing.
#[test]
fn only_the_kernel_constructs_a_validated_transaction() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/compile_fail/*.rs");
}

/// Review regressions: references resolve against the resulting graph, not a union of ids.
#[test]
fn deleting_an_edge_referenced_by_a_new_or_retained_assertion_is_refused() {
    let mut world = World::new();
    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.subject = Subject::Edge(world.existing_edge);
    for operations in [
        vec![
            GraphOperation::DeleteEdge(world.existing_edge),
            GraphOperation::AddAssertion(Box::new(assertion.clone())),
        ],
        vec![
            GraphOperation::AddAssertion(Box::new(assertion)),
            GraphOperation::DeleteEdge(world.existing_edge),
        ],
    ] {
        assert!(codes(&refuse(&world, operations)).contains(&"unresolved-edge"));
    }
    world
        .graph
        .assertions
        .get_mut(&world.held_assertion)
        .unwrap()
        .subject = Subject::Edge(world.existing_edge);
    assert!(codes(&refuse(
        &world,
        vec![GraphOperation::DeleteEdge(world.existing_edge)]
    ))
    .contains(&"unresolved-edge"));
}

#[test]
fn relation_assertions_require_compatible_node_endpoints() {
    let world = World::new();
    for (subject, object) in [
        (
            Subject::Node(world.open),
            Object::Value(Value::String("scalar".into())),
        ),
        (Subject::Node(world.open), Object::Type(world.decision)),
        (
            Subject::Edge(world.existing_edge),
            Object::Node(world.decided),
        ),
        (Subject::Type(world.decision), Object::Node(world.decided)),
    ] {
        let mut assertion = world.supported_assertion(AssertionId::mint());
        assertion.subject = subject;
        assertion.predicate = Predicate::Relation(world.depends_on);
        assertion.object = object;
        assert!(codes(&refuse(
            &world,
            vec![GraphOperation::AddAssertion(Box::new(assertion))]
        ))
        .contains(&"edge-endpoint-type"));
    }
    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.predicate = Predicate::Relation(world.depends_on);
    assertion.object = Object::Node(world.decided);
    assert!(world
        .pipeline()
        .validate(
            &world.snapshot(),
            &world.proposal(vec![GraphOperation::AddAssertion(Box::new(assertion))])
        )
        .is_ok());
}

#[test]
fn relation_assertions_check_both_endpoint_types_and_allow_inherited_types() {
    let mut world = World::new();
    let other = TypeId::mint();
    let child = TypeId::mint();
    let mut subtype = NodeType::new(child, "SpecialDecision");
    subtype.parents.insert(world.decision);
    let decision = world
        .graph
        .ontology
        .node_type(world.decision)
        .unwrap()
        .clone();
    let relation = world
        .graph
        .ontology
        .edge_type(world.depends_on)
        .unwrap()
        .clone();
    world.graph.ontology = Ontology::load(OntologyDocument {
        version: world.graph.ontology.version().clone(),
        node_types: vec![decision, NodeType::new(other, "Observation"), subtype],
        edge_types: vec![relation],
    })
    .unwrap();
    for node in [world.open, world.decided] {
        world.graph.nodes.get_mut(&node).unwrap().type_id = other;
        let mut assertion = world.supported_assertion(AssertionId::mint());
        assertion.predicate = Predicate::Relation(world.depends_on);
        assertion.object = Object::Node(world.decided);
        assert!(codes(&refuse(
            &world,
            vec![GraphOperation::AddAssertion(Box::new(assertion))]
        ))
        .contains(&"edge-endpoint-type"));
        world.graph.nodes.get_mut(&node).unwrap().type_id = child;
    }
    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.predicate = Predicate::Relation(world.depends_on);
    assertion.object = Object::Node(world.decided);
    assert!(world
        .pipeline()
        .validate(
            &world.snapshot(),
            &world.proposal(vec![GraphOperation::AddAssertion(Box::new(assertion))])
        )
        .is_ok());
}

#[test]
fn property_assertions_check_type_objects_edge_properties_and_type_subjects() {
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
    for subject in [
        Subject::Node(world.open),
        Subject::Edge(world.existing_edge),
    ] {
        let mut assertion = world.supported_assertion(AssertionId::mint());
        assertion.subject = subject;
        assertion.object = Object::Type(world.decision);
        assert_eq!(
            codes(&refuse(
                &world,
                vec![GraphOperation::AddAssertion(Box::new(assertion))]
            )),
            vec!["wrong-type"]
        );
    }
    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.subject = Subject::Type(world.decision);
    assert_eq!(
        codes(&refuse(
            &world,
            vec![GraphOperation::AddAssertion(Box::new(assertion))]
        )),
        vec!["undeclared-property"]
    );
    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.subject = Subject::Edge(world.existing_edge);
    assert!(world
        .pipeline()
        .validate(
            &world.snapshot(),
            &world.proposal(vec![GraphOperation::AddAssertion(Box::new(assertion))])
        )
        .is_ok());
}

#[test]
fn competing_property_writes_are_refused_in_every_order() {
    let world = World::new();
    let first = GraphOperation::UpdateProperty(PropertyMutation {
        node: world.open,
        property: world.title,
        values: vec![Value::String("first".into())],
    });
    let second = GraphOperation::UpdateProperty(PropertyMutation {
        node: world.open,
        property: world.title,
        values: vec![Value::String("second".into())],
    });
    for operations in [vec![first.clone(), second.clone()], vec![second, first]] {
        assert_eq!(
            codes(&refuse(&world, operations)),
            vec!["conflicting-write"]
        );
    }
    let fresh = NodeId::mint();
    let create = GraphOperation::CreateNode(world.draft(fresh, "first"));
    let update = GraphOperation::UpdateProperty(PropertyMutation {
        node: fresh,
        property: world.title,
        values: vec![Value::String("second".into())],
    });
    for operations in [vec![create.clone(), update.clone()], vec![update, create]] {
        assert_eq!(
            codes(&refuse(&world, operations)),
            vec!["conflicting-write"]
        );
    }
}

#[test]
fn competing_lifecycle_writes_are_refused() {
    let world = World::new();
    let invoke = GraphOperation::Invoke {
        node: world.open,
        operation: "decide".into(),
        arguments: BTreeMap::new(),
    };
    assert_eq!(
        codes(&refuse(&world, vec![invoke.clone(), invoke])),
        vec!["conflicting-write"]
    );
}

#[test]
fn unsupported_schema_changes_and_merges_refuse_explicitly() {
    let world = World::new();
    let mut invalid_type = NodeType::new(TypeId::mint(), "Invalid");
    invalid_type.parents.insert(TypeId::mint());
    for operation in [
        GraphOperation::DefineNodeType(Box::new(invalid_type)),
        GraphOperation::DefineEdgeType(Box::new(EdgeType::new(TypeId::mint(), "invalid"))),
        GraphOperation::ModifyProperty(PropertyDefinition::new(
            PropertyId::mint(),
            "missing",
            ValueType::String,
        )),
        GraphOperation::MergeEntity(EntityMerge {
            absorbed: world.open,
            into: world.decided,
        }),
    ] {
        assert_eq!(
            codes(&refuse(&world, vec![operation])),
            vec!["unsupported-operation"]
        );
    }
}

#[test]
fn opaque_preconditions_and_emissions_are_not_silently_accepted() {
    for emission in [false, true] {
        let mut world = World::new();
        let mut decision = world
            .graph
            .ontology
            .node_type(world.decision)
            .unwrap()
            .clone();
        let operation = decision.operations.get_mut("decide").unwrap();
        if emission {
            operation.emits.push("unspecified".into());
        } else {
            operation
                .preconditions
                .push("UNSUPPORTED_CONSTRAINT".into());
        }
        world.graph.ontology = Ontology::load(OntologyDocument {
            version: world.graph.ontology.version().clone(),
            node_types: vec![decision],
            edge_types: vec![world
                .graph
                .ontology
                .edge_type(world.depends_on)
                .unwrap()
                .clone()],
        })
        .unwrap();
        assert_eq!(
            codes(&refuse(
                &world,
                vec![GraphOperation::Invoke {
                    node: world.open,
                    operation: "decide".into(),
                    arguments: BTreeMap::new()
                }]
            )),
            vec!["unsupported-constraint"]
        );
    }
}

#[test]
fn applicable_opaque_property_constraints_refuse_all_node_write_paths() {
    let mut world = World::new();
    let mut decision = world
        .graph
        .ontology
        .node_type(world.decision)
        .unwrap()
        .clone();
    decision
        .properties
        .get_mut(&world.title)
        .unwrap()
        .constraints
        .push("UNSUPPORTED_CONSTRAINT".into());
    world.graph.ontology = Ontology::load(OntologyDocument {
        version: world.graph.ontology.version().clone(),
        node_types: vec![decision],
        edge_types: vec![world
            .graph
            .ontology
            .edge_type(world.depends_on)
            .unwrap()
            .clone()],
    })
    .unwrap();
    for operation in [
        GraphOperation::CreateNode(world.draft(NodeId::mint(), "valid")),
        GraphOperation::UpdateProperty(PropertyMutation {
            node: world.open,
            property: world.tags,
            values: vec![Value::String("tag".into())],
        }),
        GraphOperation::Invoke {
            node: world.open,
            operation: "decide".into(),
            arguments: BTreeMap::new(),
        },
        GraphOperation::AddAssertion(Box::new(world.supported_assertion(AssertionId::mint()))),
    ] {
        assert_eq!(
            codes(&refuse(&world, vec![operation])),
            vec!["unsupported-constraint"]
        );
    }
}

#[test]
fn an_empty_property_assignment_still_requires_a_declared_property() {
    let world = World::new();
    assert_eq!(
        codes(&refuse(
            &world,
            vec![GraphOperation::UpdateProperty(PropertyMutation {
                node: world.open,
                property: PropertyId::mint(),
                values: vec![]
            })]
        )),
        vec!["undeclared-property"]
    );
}

#[test]
fn property_cardinality_uses_the_candidate_node() {
    let world = World::new();
    let fresh = NodeId::mint();
    let mut draft = world.draft(fresh, "valid");
    draft.properties.remove(&world.title);
    let create = GraphOperation::CreateNode(draft);
    let update = GraphOperation::UpdateProperty(PropertyMutation {
        node: fresh,
        property: world.title,
        values: vec![Value::String("valid".into())],
    });
    for operations in [vec![create.clone(), update.clone()], vec![update, create]] {
        assert!(world
            .pipeline()
            .validate(&world.snapshot(), &world.proposal(operations))
            .is_ok());
    }
}

#[test]
fn an_unresolved_assertion_endpoint_does_not_acquire_an_unrelated_type_refusal() {
    let world = World::new();
    for (subject, object, predicate) in [
        (
            Subject::Node(NodeId::mint()),
            Object::Node(world.decided),
            Predicate::Relation(world.depends_on),
        ),
        (
            Subject::Node(world.open),
            Object::Node(NodeId::mint()),
            Predicate::Relation(world.depends_on),
        ),
        (
            Subject::Edge(EdgeId::mint()),
            Object::Value(Value::String("valid".into())),
            Predicate::Property(world.title),
        ),
    ] {
        let mut assertion = world.supported_assertion(AssertionId::mint());
        assertion.subject = subject;
        assertion.object = object;
        assertion.predicate = predicate;
        assert_eq!(
            refusing_validators(&refuse(
                &world,
                vec![GraphOperation::AddAssertion(Box::new(assertion))]
            )),
            vec![ValidatorName::Reference]
        );
    }
}

#[test]
fn opaque_edge_property_constraints_refuse_creation_and_assertions() {
    let mut world = World::new();
    let mut relation = world
        .graph
        .ontology
        .edge_type(world.depends_on)
        .unwrap()
        .clone();
    let mut property = PropertyDefinition::new(world.title, "title", ValueType::String);
    property.constraints.push("UNSUPPORTED_CONSTRAINT".into());
    relation.properties.insert(world.title, property);
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
    let mut assertion = world.supported_assertion(AssertionId::mint());
    assertion.subject = Subject::Edge(world.existing_edge);
    for operation in [
        GraphOperation::CreateEdge(EdgeDraft {
            id: EdgeId::mint(),
            root_id: world.root_id,
            type_id: world.depends_on,
            source: world.decided,
            target: world.open,
            properties: [(world.title, vec![Value::String("valid".into())])]
                .into_iter()
                .collect(),
        }),
        GraphOperation::AddAssertion(Box::new(assertion)),
    ] {
        assert_eq!(
            codes(&refuse(&world, vec![operation])),
            vec!["unsupported-constraint"]
        );
    }
}
