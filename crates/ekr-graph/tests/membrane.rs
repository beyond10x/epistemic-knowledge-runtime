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
//!
//! A fourth build failure in the same directory is about the other direction the membrane runs in,
//! and arrived with `architecture-decision-record:0005-float-is-not-canonical` as amended: the
//! three types both spaces hold are generic over their value, `Canonical` is implemented only
//! where that value is, and so transient state has no content address —
//! `compile_fail/transient_state_has_no_content_address.rs`. It is run by the same case below,
//! which globs the directory.

use std::collections::BTreeMap;

use ekr_core::{GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TypeId};
use ekr_graph::{
    CanonicalDependency, CanonicalGraph, CanonicalRef, GraphRoot, LocalRef, Node, Resolved, Space,
    TransientGraph, TransientRef,
};
use ekr_ontology::Value;
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

    match transient.resolve(&local, &canonical) {
        Some(Resolved::Local(node)) => assert_eq!(node.id, candidate),
        other => panic!("a transient root resolves its own nodes, as candidates: {other:?}"),
    }
    match transient.resolve(&borrowed, &canonical) {
        Some(Resolved::Canonical(node)) => assert_eq!(node.id, alice),
        other => panic!(
            "the permitted direction of the membrane: transient may depend on canonical, and what \
             comes back from that direction is a canonical node: {other:?}"
        ),
    }
    assert_eq!(transient.root.space, Space::Transient);
}

/// Which side a reference resolved to is part of the answer, and an id does not carry it.
///
/// `TransientRef`'s `PartialEq` compares by what is pointed at **and** which side it is on,
/// "because a candidate that happens to carry the same id as a canonical node is not that node".
/// This is that sentence over the resolver: one id, held by a candidate in a transient root and by
/// a node in canonical state, resolves to two different variants over two different types — and the
/// two records are not the same record.
///
/// The case exists because `Resolved` briefly had an `id()` that answered from both arms, and both
/// callers in this file used it, so nothing here asserted which side came back. The public-surface
/// guard could not notice: it counts any `.id` in the suite as a use of the accessor.
#[test]
fn a_candidate_and_a_canonical_node_that_share_an_id_do_not_resolve_alike() {
    let canonical_root = root(Space::Canonical);
    let transient_root = root(Space::Transient);
    let shared = NodeId::mint();

    let canonical = CanonicalGraph {
        root: canonical_root,
        revision: RevisionNumber::SEED,
        ontology: empty_ontology(),
        nodes: [(
            shared,
            Node::new(shared, canonical_root.id, TypeId::mint(), "Alice"),
        )]
        .into_iter()
        .collect(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };
    let transient = TransientGraph {
        root: transient_root,
        nodes: [(
            shared,
            Node::<Value>::new(shared, transient_root.id, TypeId::mint(), "A. Smith?"),
        )]
        .into_iter()
        .collect(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
    };

    let local: TransientRef<Node> = TransientRef::Local(LocalRef::new(shared));
    let borrowed: TransientRef<Node> = TransientRef::Canonical(CanonicalRef::new(shared));
    assert_ne!(
        local, borrowed,
        "the two references are not equal, which is what this case is about"
    );
    assert_eq!(local.node(), borrowed.node(), "and they carry one id");

    match (
        transient.resolve(&local, &canonical),
        transient.resolve(&borrowed, &canonical),
    ) {
        (Some(Resolved::Local(candidate)), Some(Resolved::Canonical(node))) => {
            assert_eq!(candidate.id, node.id, "one id");
            assert_ne!(
                candidate.canonical_name, node.canonical_name,
                "and not one node"
            );
        }
        other => panic!("one id resolved to two sides, and did not come back as two: {other:?}"),
    }
}

/// The trait is a statement about which references canonical state may hold, so it is asked
/// directly rather than only through the graph.
#[test]
fn a_canonical_reference_is_the_only_canonical_dependency() {
    let node = NodeId::mint();
    let reference: CanonicalRef<Node> = CanonicalRef::new(node);

    fn only_canonical<R: CanonicalDependency<Target = Node>>(reference: &R) -> NodeId {
        reference.id()
    }

    assert_eq!(only_canonical(&reference), node);
}

/// Every guarantee in `tests/compile_fail/`, as a build failure rather than a review comment.
///
/// The directory is globbed, so a case added there is run without this file changing. Nine today:
/// eight about which references canonical state may hold — five of them added by wave p1-14: a
/// reference holds its own kind's id, a claim's node references, a subject's edge arm and an
/// assertion's evidence are canonical references, and the kinds canonical state keeps no map of
/// are not targets — and one about which state has a content address. `crates/ekr-graph/src/lib.rs`
/// maps every reference canonical state holds to its case. Each names in
/// its own doc comment what it is about; this case asserts only that each fails to compile with the
/// message recorded beside it.
#[test]
fn the_membrane_is_a_set_of_build_failures() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/compile_fail/*.rs");
}
