//! Adversarial cases against `story:transaction-and-validators`.
//!
//! Each case here drives the implementation against a sentence the unit itself wrote — the
//! reference validator's account of what a graph identity is, the provenance validator's account
//! of who refuses an evidence id nothing holds, and AGENTS.md invariants 4 and 5 — rather than
//! against a behaviour the suite already asserts.
//!
//! The fixture is the runtime's own vocabulary: a `Decision` node type and a `depends_on` edge
//! type between decisions. No person, real or invented, appears in it.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue, Confidence, Edge,
    Evidence, EvidenceSource, GraphRoot, GraphSnapshot, Node, Object, Predicate, Space, Subject,
    TemporalRange, TransactionTime,
};
use ekr_kernel::{EdgeDraft, GraphOperation, GraphTransaction, NodeDraft, Pipeline};
use ekr_ontology::{
    Cardinality, EdgeType, NodeType, Ontology, OntologyDocument, PropertyDefinition, SchemaVersion,
    Value, ValueType,
};

/// Canonical state at revision 7: two decisions, one `depends_on` edge between them, one accepted
/// assertion and the one piece of evidence it cites.
struct World {
    graph: CanonicalGraph,
    root_id: GraphRootId,
    decision: TypeId,
    depends_on: TypeId,
    title: PropertyId,
    open: NodeId,
    decided: NodeId,
    held_assertion: AssertionId,
    retained_evidence: EvidenceId,
    proposer: AgentId,
    reviewer: AgentId,
}

impl World {
    /// The world, and the identities the cases name.
    fn new() -> Self {
        let root_id = GraphRootId::mint();
        let schema = SchemaVersionId::mint();
        let (decision, depends_on) = (TypeId::mint(), TypeId::mint());
        let (title, tags) = (PropertyId::mint(), PropertyId::mint());
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
        open_node.properties.insert(
            title,
            vec![CanonicalValue::String(
                "Adopt the eventlog store".to_owned(),
            )],
        );
        let mut decided_node = Node::new(decided, root_id, decision, "Hash canonical state only");
        decided_node.properties.insert(
            title,
            vec![CanonicalValue::String(
                "Hash canonical state only".to_owned(),
            )],
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
                edges: [(
                    existing_edge,
                    Edge::new(
                        existing_edge,
                        root_id,
                        depends_on,
                        CanonicalRef::new(open),
                        CanonicalRef::new(decided),
                    ),
                )]
                .into_iter()
                .collect(),
                assertions: [(held_assertion, assertion)].into_iter().collect(),
                evidence: [(retained_evidence, evidence)].into_iter().collect(),
            },
            root_id,
            decision,
            depends_on,
            title,
            open,
            decided,
            held_assertion,
            retained_evidence,
            proposer,
            reviewer,
        }
    }

    fn snapshot(&self) -> GraphSnapshot<'_> {
        GraphSnapshot::of(&self.graph)
    }

    /// A proposal by the fixture's proposer, citing the evidence ids given.
    fn proposal(
        &self,
        operations: Vec<GraphOperation>,
        evidence: BTreeSet<EvidenceId>,
    ) -> GraphTransaction {
        GraphTransaction {
            id: TransactionId::mint(),
            proposer: self.proposer,
            operations,
            evidence,
            schema_version: None,
        }
    }

    /// The deterministic pipeline, run by an actor who is not the proposer.
    fn pipeline(&self) -> Pipeline {
        Pipeline::deterministic(self.reviewer)
    }

    /// An assertion about the open decision's title, citing whatever evidence it is given.
    fn assertion(
        &self,
        id: AssertionId,
        object: &str,
        evidence: BTreeSet<EvidenceId>,
    ) -> Assertion<Value> {
        Assertion {
            id,
            root_id: self.root_id,
            subject: Subject::Node(self.open),
            predicate: Predicate::Property(self.title),
            object: Object::Value(Value::String(object.to_owned())),
            evidence,
            proposed_by: self.proposer,
            lifecycle: ekr_graph::AssertionLifecycle::Active,
            assessment: Assessment::Proposed,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        }
    }

    /// A well-formed `Decision` draft under the graph root the snapshot is of.
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
}

/// An evidence id the proposer minted, named in the transaction, is not provenance.
///
/// `src/validate/provenance.rs:157` says of the existence of cited evidence: "That is reference
/// existence, and the reference validator refuses it; an assertion citing an evidence id nothing
/// holds is refused there, once." It is not: `Reference::of` puts every id in
/// `GraphTransaction::evidence` into the resolvable set, and a transaction carries evidence *ids*
/// and no `Evidence` — no `GraphOperation` creates one — so an id the proposer invented resolves
/// against nothing but the proposer's having written it down twice.
///
/// That is AGENTS.md invariant 4 defeated exactly where it is stated: "'An agent said so' is not
/// sufficient." The provenance validator is satisfied by a non-empty set and the reference
/// validator is satisfied by the transaction naming it, and between them an assertion with no
/// evidence at all reaches canonical state.
#[test]
fn an_assertion_whose_only_evidence_the_proposer_invented_is_refused() {
    let world = World::new();
    let invented = EvidenceId::mint();
    let proposal = world.proposal(
        vec![GraphOperation::AddAssertion(Box::new(world.assertion(
            AssertionId::mint(),
            "Adopt the eventlog store",
            [invented].into_iter().collect(),
        )))],
        [invented].into_iter().collect(),
    );

    let issues = world
        .pipeline()
        .validate(&world.snapshot(), &proposal)
        .err()
        .unwrap_or_else(|| {
            panic!(
                "an assertion whose only evidence is an id the proposer minted was validated; \
                 nothing in the graph or the transaction holds evidence {invented}"
            )
        });
    assert!(
        !issues.is_empty(),
        "the refusal names the evidence that does not exist: {issues:?}"
    );
}

/// A node created over an id canonical state already holds.
///
/// The structural validator refuses one identity created twice *inside* one transaction
/// (`src/validate/structural.rs:22`). Nothing asks the other half: a `CreateNode` naming a node
/// the snapshot already holds passes every one of the seven, so a proposal can replace a committed
/// node's type, name and properties through the create path rather than the update path — AGENTS.md
/// invariant 5, "Committed revisions are immutable", decided by whatever the store does with an id
/// it has already seen.
#[test]
fn a_create_node_over_an_id_canonical_state_already_holds_is_refused() {
    let world = World::new();
    let proposal = world.proposal(
        vec![GraphOperation::CreateNode(
            world.draft(world.open, "A different decision entirely"),
        )],
        BTreeSet::new(),
    );

    let issues = world
        .pipeline()
        .validate(&world.snapshot(), &proposal)
        .err()
        .unwrap_or_else(|| {
            panic!(
                "a transaction that creates node {} a second time was validated; the snapshot \
                 already holds it",
                world.open
            )
        });
    assert!(
        !issues.is_empty(),
        "the refusal names the identity that already exists: {issues:?}"
    );
}

/// An assertion added over the id of an assertion canonical state already holds.
///
/// The same hole as the node above, on the record design § 36 is most explicit about: an assertion
/// is retracted, never erased, and a later claim is a new revision. This proposal carries the id of
/// an `Accepted` assertion and a different object, and no validator has a question about it.
#[test]
fn an_add_assertion_over_an_id_canonical_state_already_holds_is_refused() {
    let world = World::new();
    let proposal = world.proposal(
        vec![GraphOperation::AddAssertion(Box::new(world.assertion(
            world.held_assertion,
            "Something the accepted assertion does not say",
            [world.retained_evidence].into_iter().collect(),
        )))],
        [world.retained_evidence].into_iter().collect(),
    );

    let issues = world
        .pipeline()
        .validate(&world.snapshot(), &proposal)
        .err()
        .unwrap_or_else(|| {
            panic!(
                "a transaction that adds assertion {} a second time, with a different object, was \
                 validated; the snapshot already holds it as Accepted",
                world.held_assertion
            )
        });
    assert!(
        !issues.is_empty(),
        "the refusal names the assertion that already exists: {issues:?}"
    );
}

/// A node created under a graph root that does not exist.
///
/// `src/validate/reference.rs:15` enumerates what the validator resolves — "Nodes, edges,
/// assertions and evidence are graph identities and are resolved here" — and excuses exactly one
/// identity, `TypeId`, with a reason. `GraphRootId` is in neither list, and the word `root` does
/// not occur anywhere under `src/validate/`. So `NodeDraft::root_id`, `EdgeDraft::root_id` and
/// `Assertion::root_id` are unchecked: this proposal puts a node into canonical state under a root
/// nothing holds, which is design § 6.2's dangling reference in the one field that says which graph
/// the node is even in.
#[test]
fn a_node_created_under_a_graph_root_that_does_not_exist_is_refused() {
    let world = World::new();
    let elsewhere = GraphRootId::mint();
    let mut draft = world.draft(NodeId::mint(), "Adopt trybuild for the membrane");
    draft.root_id = elsewhere;
    let proposal = world.proposal(vec![GraphOperation::CreateNode(draft)], BTreeSet::new());

    let issues = world
        .pipeline()
        .validate(&world.snapshot(), &proposal)
        .err()
        .unwrap_or_else(|| {
            panic!(
                "a node was validated into graph root {elsewhere}, which is not the root of the \
                 snapshot it was validated against and is held by nothing"
            )
        });
    assert!(
        !issues.is_empty(),
        "the refusal names the graph root that does not exist: {issues:?}"
    );
}

/// One transaction, two orderings of the same operations, two verdicts.
///
/// `src/validate/reference.rs:9` states the semantics the kernel claims: "The transaction is read
/// as a set, not as a sequence. A transaction commits or does not; there is no instant between its
/// operations at which canonical state is half-changed." `Known::of` keeps that promise — it
/// inserts every created id whatever position it is in. `validate::cardinality::outgoing` does not:
/// it replays the operations in `Vec` order, so a `DeleteEdge` listed before the `CreateEdge` of
/// the same edge removes nothing and the edge is counted as surviving.
///
/// The result is that the same set of operations is accepted in one order and refused with
/// `edge-cardinality` in the other, which makes two of the seven validators disagree about what a
/// transaction is.
#[test]
fn reordering_the_operations_of_one_transaction_does_not_change_the_verdict() {
    let world = World::new();
    let transient = EdgeId::mint();
    let create = GraphOperation::CreateEdge(EdgeDraft {
        id: transient,
        root_id: world.root_id,
        type_id: world.depends_on,
        source: world.open,
        target: world.decided,
        properties: BTreeMap::new(),
    });
    let delete = GraphOperation::DeleteEdge(transient);

    let created_then_deleted =
        world.proposal(vec![create.clone(), delete.clone()], BTreeSet::new());
    let deleted_then_created = world.proposal(vec![delete, create], BTreeSet::new());

    let first = world
        .pipeline()
        .validate(&world.snapshot(), &created_then_deleted);
    let second = world
        .pipeline()
        .validate(&world.snapshot(), &deleted_then_created);

    assert!(
        first.is_ok(),
        "creation/deletion cancellation must validate: {first:?}"
    );
    assert!(
        second.is_ok(),
        "permuting cancellation must validate: {second:?}"
    );

    assert_eq!(
        first.is_ok(),
        second.is_ok(),
        "one operation set, two orders, two verdicts: create-then-delete gave {:?} and \
         delete-then-create gave {:?}",
        first.as_ref().map(|_| "Ok").map_err(|issues| issues
            .iter()
            .map(|issue| issue.code.clone())
            .collect::<Vec<_>>()),
        second.as_ref().map(|_| "Ok").map_err(|issues| issues
            .iter()
            .map(|issue| issue.code.clone())
            .collect::<Vec<_>>()),
    );
}
