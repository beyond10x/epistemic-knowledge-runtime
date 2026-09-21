//! Adversary, wave p1-06 pass 1: the two marker mechanisms ADR 0008's acceptance rests on, read as
//! the specification their own doc comments say they are.
//!
//! ADR 0008 typed the five references the task's `## Scope` names. Both cases below are about the
//! machinery those five now depend on, not about the five.

use std::collections::BTreeMap;

use ekr_core::{GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TypeId};
use ekr_graph::{
    CanonicalGraph, CanonicalRef, Evidence, GraphRoot, Node, Object, Space, ValueSpace,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion, Value};

/// An ontology with no declarations — nothing here is typed, only referenced.
fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("a document with no declarations coheres")
}

/// `Subject` and `Object`'s new reference parameter defaults to the **canonical** reference for
/// every value, including the transient one the crate's own documentation names.
///
/// `crates/ekr-graph/src/assertion.rs:90` — a sentence this unit did not change, under a type it
/// did:
///
/// > a candidate claim in a transient root is `Object<ekr_ontology::Value>` and may say something
/// > approximate.
///
/// After ADR 0008 that spelling is `Object<Value, CanonicalRef<Node>>`: the type the crate tells a
/// reader to use for a claim in a transient root demands a *canonical* reference in its `Node` arm.
/// [`ValueSpace`] is the trait that binds the value to the reference it may carry, and the default
/// does not go through it — so the one place the two can disagree is the one place the defaults are
/// used. `Assertion<V>` is unaffected because it writes `Object<V, V::NodeRef>` explicitly; a
/// caller writing the documented spelling is not.
///
/// The fix is a default that goes through the trait, which is legal Rust:
/// `Object<V: ValueSpace = CanonicalValue, R = <V as ValueSpace>::NodeRef>`, and the same for
/// `Subject`.
#[test]
fn the_default_reference_of_a_transient_claim_is_the_transient_reference() {
    // Compared to each other rather than to a literal, so the case does not pin rustc's spelling.
    let documented = std::any::type_name::<Object<Value>>();
    let transient = std::any::type_name::<Object<Value, <Value as ValueSpace>::NodeRef>>();
    // `type_name` elides a parameter that is at its default, so this printing identically to
    // `documented` is what says the default is the canonical reference.
    let canonical = std::any::type_name::<Object<Value, CanonicalRef<Node>>>();
    assert_eq!(
        documented, transient,
        "crates/ekr-graph/src/assertion.rs:90 tells a reader that a candidate claim in a transient \
         root is `Object<ekr_ontology::Value>`. ADR 0008 gave Object a second parameter whose \
         default does not consult ValueSpace, so the documented spelling prints as `{documented}` \
         — identically to `Object<Value, CanonicalRef<Node>>`, which prints `{canonical}` — while \
         the transient instantiation is `{transient}`. The type the crate names for a claim in a \
         transient root carries a canonical reference"
    );
}

/// `CanonicalRef<T>`'s marker does not keep a reference to one kind of thing out of a slot that
/// wants another, which is the one job its own doc comment gives it.
///
/// `crates/ekr-graph/src/canonical.rs:68-71`:
///
/// > The `T` of [`CanonicalRef`] is a marker with no data, and its whole job is to keep a reference
/// > to one kind of thing out of a slot that wants another.
///
/// `CanonicalTarget` is implemented for seven types — `Node`, `Edge`, `Assertion`, `Evidence`,
/// `Observation`, `Support`, `GraphRoot` — and `CanonicalRef<T>` holds a `NodeId` for every one of
/// them. `CanonicalDependency::node` therefore answers a `NodeId` for a reference that says it
/// points at evidence, and `CanonicalGraph::resolve` looks it up in the **nodes** map. A reference
/// to evidence resolves to a node.
///
/// This wave is what makes it load-bearing: `CanonicalTarget` is now the bound on `Serialize`,
/// `Deserialize`, `Canonical`, `Ord` and `Hash` for the type canonical state is made of.
///
/// **Filed rather than fixed, and this case asserts what is true.** Closing it is a design change
/// to the marker — the id a reference holds has to be typed by `T`, which is a change to
/// `CanonicalTarget`, to `CanonicalDependency::node` and to `CanonicalGraph::resolve` together —
/// and it is pre-existing: nothing in the tree reaches it, because `Node` is the only one of the
/// seven with a caller. `task:canonical-reference-holds-a-node-id-for-every-target` carries it, and
/// `crates/ekr-graph/src/canonical.rs`'s `CanonicalTarget` names it where the markers are declared.
///
/// So this asserts the defect and **goes red the day that task closes**, which is when somebody
/// should read it again and turn it back into the assertion the adversary wrote. The construction
/// is the adversary's, unchanged.
#[test]
fn a_reference_to_evidence_resolves_to_a_node_because_the_marker_is_decoration() {
    let root = GraphRoot {
        id: GraphRootId::mint(),
        space: Space::Canonical,
        schema_version_id: SchemaVersionId::mint(),
        parent: None,
        created_at: Timestamp::EPOCH,
    };
    let held = Node::new(NodeId::mint(), root.id, TypeId::mint(), "held");
    let held_id = held.id;
    let canonical = CanonicalGraph {
        root,
        revision: RevisionNumber::SEED,
        ontology: ontology(),
        nodes: [(held_id, held)].into_iter().collect(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };

    // A reference that says it points at a piece of evidence. `Evidence` is a `CanonicalTarget`,
    // so this is a well-formed value and a `CanonicalDependency`.
    let to_evidence: CanonicalRef<Evidence> = CanonicalRef::new(held_id);

    assert_eq!(
        canonical.resolve(&to_evidence).map(|node| node.id),
        Some(held_id),
        "canonical.rs says the marker's whole job is to keep a reference to one kind of thing out \
         of a slot that wants another. CanonicalRef<T> holds a NodeId for all seven targets, so a \
         CanonicalRef<Evidence> resolves against the nodes map and answers the node {held_id}: the \
         marker is decoration on the one operation that reads it. \
         task:canonical-reference-holds-a-node-id-for-every-target is what closes this, and the \
         day it does this case goes red"
    );
}
