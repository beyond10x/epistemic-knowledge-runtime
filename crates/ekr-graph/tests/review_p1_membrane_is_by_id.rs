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

/// The same crossing at run time: canonical state holding an edge into a transient root, and the
/// only answer the crate can give is `None`.
#[test]
fn canonical_state_holds_an_edge_into_a_transient_root_and_no_type_refuses_it() {
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

    // Canonical state, holding an edge whose target is that candidate.
    let canonical_root = root(Space::Canonical);
    let held = Node::new(NodeId::mint(), canonical_root.id, TypeId::mint(), "held");
    let into_the_forest = Edge::new(
        EdgeId::mint(),
        canonical_root.id,
        TypeId::mint(),
        held.id,
        candidate_id,
    );
    let canonical = CanonicalGraph {
        root: canonical_root,
        revision: RevisionNumber::SEED,
        ontology,
        nodes: [(held.id, held)].into_iter().collect(),
        edges: [(into_the_forest.id, into_the_forest)]
            .into_iter()
            .collect(),
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

    // The edge is in canonical state and its target is that candidate. The reference types were
    // never in the path — the edge carries a `NodeId` — so the only thing left to ask is whether
    // canonical state resolves what it holds.
    let target = canonical
        .edges
        .values()
        .next()
        .expect("the edge is held")
        .target;
    assert!(
        canonical
            .resolve(&CanonicalRef::<Node>::new(target))
            .is_some(),
        "AGENTS.md invariant 2: a Canonical → Transient reference is unrepresentable at the type \
         level; canonical state holds an edge whose target {target} is a candidate of a transient \
         root, built from public fields with no CanonicalRef in the path, and resolution answers \
         None rather than the crossing being a type error"
    );
}
