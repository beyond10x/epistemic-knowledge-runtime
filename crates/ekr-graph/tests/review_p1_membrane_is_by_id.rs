//! Independent review of the P1 core: AGENTS.md invariant 2, read against what canonical state is
//! made of rather than against the reference types beside it.
//!
//! > A `Canonical → Transient` reference is unrepresentable at the type level, not merely refused.
//!
//! Two findings that must not be confused, and the case names keep them apart.
//!
//! **The sealed machinery is sound.** `CanonicalDependency` and `CanonicalTarget` are sealed,
//! `CanonicalRef<T>` is bounded, and the four cases in `tests/compile_fail/` each fail to compile
//! for the reason their `.stderr` pins. This review could not fault any of that.
//!
//! **And canonical state holds none of it.** `CanonicalGraph` is a `GraphRoot` and four maps, and
//! every reference inside those maps is a bare id: `Edge::source`, `Edge::target`,
//! `Subject::Node`, `Object::Node`, `CanonicalValue::NodeRef`, `Assertion::evidence`. No field of
//! `Node`, `Edge`, `Assertion` or `CanonicalGraph` has the type `CanonicalRef<_>`. So the crossing
//! the invariant says is unrepresentable is written with a `NodeId`, compiles, constructs, and is
//! *refused* — by the kernel's reference validator, on the transaction path only. The trybuild cases
//! prove a property of types canonical state does not use.

use std::collections::BTreeMap;

use ekr_core::{EdgeId, GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TypeId};
use ekr_graph::{
    CanonicalGraph, CanonicalRef, Edge, GraphRoot, LocalRef, Node, Resolved, Space, TransientGraph,
    TransientRef,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion, Value};

/// The invariant, asserted literally as a build failure, in a directory of its own so that the
/// four existing cases keep proving what they prove.
///
/// The case in `tests/review_p1_compile_fail/` builds a `CanonicalGraph` whose edge targets a node
/// held only by a `TransientGraph`. The invariant says that is not a type. It is one, so the file
/// compiles, and `trybuild` reports the case red for that reason.
#[test]
fn the_sealed_reference_types_are_sound_and_canonical_state_holds_none_of_them() {
    let cases = trybuild::TestCases::new();
    cases.compile_fail("tests/review_p1_compile_fail/*.rs");
}

/// The same crossing at run time — **and after ADR 0008 there is no such value to build.**
///
/// This case asserted the defect: it built a `CanonicalGraph` holding an `Edge` whose `target` was
/// a candidate's id, and asserted that resolving it answered something, so that the only way to
/// green was for the construction to stop being a type.
/// `architecture-decision-record:0008-canonical-state-references-are-typed` did that — `Edge`'s two
/// ends are `CanonicalRef<Node>` where the value is canonical — and the case's own three lines stop
/// compiling with it: `Edge::new(…, candidate_id)` is a type error, and so is
/// `CanonicalRef::<Node>::new(edge.target)`, because `target` is already one.
///
/// A case staged through the path the fix closes cannot survive the fix, and the trybuild case
/// above is the same assertion in the form that can: it *is* the build failure. What is left to
/// read at run time is the other half of the sentence — what canonical state does hold, and what it
/// answers for a reference it does not.
#[test]
fn canonical_state_resolves_the_references_it_holds_and_answers_none_for_one_it_does_not() {
    let schema = SchemaVersionId::mint();
    let ontology = Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(schema, Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("an empty document coheres");
    let root = |space| GraphRoot {
        id: GraphRootId::mint(),
        space,
        schema_version_id: schema,
        parent: None,
        created_at: Timestamp::EPOCH,
    };

    // A candidate, held by a transient root and nowhere else.
    let transient_root = root(Space::Transient);
    let candidate: Node<Value> = Node::new(
        NodeId::mint(),
        transient_root.id,
        TypeId::mint(),
        "a-candidate",
    );
    let candidate_id = candidate.id;
    let transient = TransientGraph {
        root: transient_root,
        nodes: [(candidate_id, candidate)].into_iter().collect(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
    };

    // Canonical state, holding an edge between two nodes it holds. Its ends are references and not
    // ids, so the edge into the forest the trybuild case writes is not a value of this type.
    let canonical_root = root(Space::Canonical);
    let held = Node::new(NodeId::mint(), canonical_root.id, TypeId::mint(), "held");
    let also_held = Node::new(
        NodeId::mint(),
        canonical_root.id,
        TypeId::mint(),
        "also-held",
    );
    let between = Edge::new(
        EdgeId::mint(),
        canonical_root.id,
        TypeId::mint(),
        CanonicalRef::new(held.id),
        CanonicalRef::new(also_held.id),
    );
    let canonical = CanonicalGraph {
        root: canonical_root,
        revision: RevisionNumber::SEED,
        ontology,
        nodes: [(held.id, held), (also_held.id, also_held)]
            .into_iter()
            .collect(),
        edges: [(between.id, between)].into_iter().collect(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };

    // The candidate is a candidate: the permitted direction resolves it on the transient side.
    assert!(
        matches!(
            transient.resolve(
                &TransientRef::<Node>::Local(LocalRef::new(candidate_id)),
                &canonical
            ),
            Some(Resolved::Local(_))
        ),
        "the target is a candidate of the transient root"
    );

    // What canonical state holds, it resolves.
    let target = canonical
        .edges
        .values()
        .next()
        .expect("the edge is held")
        .target;
    assert!(
        canonical.resolve(&target).is_some(),
        "canonical state resolves a reference it holds"
    );

    // And a canonical reference to an id canonical state does not hold is answered `None` — which
    // is a *dangling* reference and a different thing from the forbidden one. The type refuses the
    // crossing; the kernel's reference validator refuses the dangle. Minting this one is the only
    // way to write it at all, and it is still not the crossing: nothing here says the id belongs to
    // a transient root, and no type could, because an id does not carry which side it came from.
    assert!(
        canonical
            .resolve(&CanonicalRef::<Node>::new(candidate_id))
            .is_none(),
        "a reference canonical state does not hold resolves to None rather than to something"
    );
}
