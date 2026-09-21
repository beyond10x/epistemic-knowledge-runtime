//! The canonical/transient membrane: design § 23, and AGENTS.md invariant 2.
//!
//! > No equivalent `CanonicalGraph -> TransientRef` should exist. Invalid dependency states should
//! > be difficult or impossible to represent.
//!
//! AGENTS.md states the same thing as a claim that can be checked: "A `Canonical → Transient`
//! reference is unrepresentable at the type level, **not merely refused**." A runtime refusal is a
//! branch someone can forget to take and a reviewer has to find; the cases below put the
//! guarantee where a build fails on it.
//!
//! Two halves, and the second is the one that keeps the first true:
//!
//! * a `TransientRef` cannot be handed to canonical state — `compile_fail/canonical_graph_rejects_a_transient_ref.rs`;
//! * and the trait that says so is **sealed**, so a crate above this one cannot answer the first
//!   case by implementing its way past it — `compile_fail/canonical_dependency_is_sealed.rs`.

use std::collections::BTreeMap;

use ekr_core::{GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TypeId};
use ekr_graph::{
    CanonicalDependency, CanonicalGraph, CanonicalRef, GraphRoot, LocalRef, Node, Space,
    TransientGraph, TransientRef,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};

fn root(space: Space) -> GraphRoot {
    GraphRoot {
        id: GraphRootId::mint(),
        space,
        schema_version_id: SchemaVersionId::mint(),
        parent: None,
        created_at: Timestamp::EPOCH,
    }
}

fn empty_ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("an empty ontology coheres")
}

#[test]
fn canonical_state_resolves_a_canonical_reference() {
    let canonical_root = root(Space::Canonical);
    let alice = NodeId::mint();
    let graph = CanonicalGraph {
        root: canonical_root,
        revision: RevisionNumber::SEED,
        ontology: empty_ontology(),
        nodes: [(
            alice,
            Node::new(alice, canonical_root.id, TypeId::mint(), "Alice"),
        )]
        .into_iter()
        .collect(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };

    let reference: CanonicalRef<Node> = CanonicalRef::new(alice);
    assert_eq!(reference.node(), alice);
    assert_eq!(
        graph.resolve(&reference).map(|node| node.id),
        Some(alice),
        "canonical state may depend on canonical state"
    );

    // A reference to something the graph does not hold is `None`, not a panic and not a stub.
    let dangling: CanonicalRef<Node> = CanonicalRef::new(NodeId::mint());
    assert!(graph.resolve(&dangling).is_none());
}

#[test]
fn transient_state_may_depend_on_canonical_state_and_on_its_own() {
    let canonical_root = root(Space::Canonical);
    let alice = NodeId::mint();
    let canonical = CanonicalGraph {
        root: canonical_root,
        revision: RevisionNumber::SEED,
        ontology: empty_ontology(),
        nodes: [(
            alice,
            Node::new(alice, canonical_root.id, TypeId::mint(), "Alice"),
        )]
        .into_iter()
        .collect(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };

    let transient_root = root(Space::Transient);
    let candidate = NodeId::mint();
    let transient = TransientGraph {
        root: transient_root,
        nodes: [(
            candidate,
            Node::new(candidate, transient_root.id, TypeId::mint(), "A. Smith?"),
        )]
        .into_iter()
        .collect(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
    };

    let local: TransientRef<Node> = TransientRef::Local(LocalRef::new(candidate));
    let borrowed: TransientRef<Node> = TransientRef::Canonical(CanonicalRef::new(alice));
    assert_eq!(local.node(), candidate);
    assert_eq!(borrowed.node(), alice);

    assert_eq!(
        transient.resolve(&local, &canonical).map(|node| node.id),
        Some(candidate),
        "a transient root resolves its own nodes"
    );
    assert_eq!(
        transient.resolve(&borrowed, &canonical).map(|node| node.id),
        Some(alice),
        "the permitted direction of the membrane: transient may depend on canonical"
    );
    assert_eq!(transient.root.space, Space::Transient);
}

/// The trait is a statement about which references canonical state may hold, so it is asked
/// directly rather than only through the graph.
#[test]
fn a_canonical_reference_is_the_only_canonical_dependency() {
    let node = NodeId::mint();
    let reference: CanonicalRef<Node> = CanonicalRef::new(node);

    fn only_canonical<R: CanonicalDependency>(reference: &R) -> NodeId {
        reference.node()
    }

    assert_eq!(only_canonical(&reference), node);
}

/// The guarantee, as a build failure rather than a review comment.
#[test]
fn canonical_state_cannot_hold_a_transient_reference() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/compile_fail/*.rs");
}
