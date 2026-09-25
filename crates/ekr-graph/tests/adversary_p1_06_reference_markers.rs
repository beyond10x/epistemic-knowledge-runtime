//! Adversary, wave p1-06 pass 1: the two marker mechanisms ADR 0008's acceptance rests on, read as
//! the specification their own doc comments say they are.
//!
//! ADR 0008 typed the five references the task's `## Scope` names. Both cases below are about the
//! machinery those five now depend on, not about the five.

use std::collections::BTreeMap;

use ekr_core::{
    AgentId, ContentHash, EvidenceId, GraphRootId, NodeId, RevisionNumber, SchemaVersionId,
    Timestamp, TypeId,
};
use ekr_graph::{
    CanonicalGraph, CanonicalRef, Confidence, Evidence, EvidenceSource, GraphRoot, Node, Object,
    Space, ValueSpace,
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

/// `CanonicalRef<T>`'s marker keeps a reference to one kind of thing out of a slot that wants
/// another, which is the one job its own doc comment gives it.
///
/// What the adversary of wave p1-06 measured: `CanonicalTarget` was implemented for seven types —
/// `Node`, `Edge`, `Assertion`, `Evidence`, `Observation`, `Support`, `GraphRoot` — and
/// `CanonicalRef<T>` held a `NodeId` for every one of them, so `CanonicalGraph::resolve` looked a
/// reference to evidence up in the **nodes** map and answered a node. Until wave p1-14 this case
/// asserted that defect, so that it would go red the day the task closed.
///
/// # Inverted by wave p1-14
///
/// `task:canonical-reference-holds-a-node-id-for-every-target` closed it with an associated `Id`
/// on `CanonicalTarget`: `CanonicalRef<Evidence>` holds an `EvidenceId`, and `resolve` looks in
/// the map for its kind. The adversary's construction is kept — one graph, a node the reference
/// could be confused with — and sharpened: the node and the evidence carry the **same 128 bits**,
/// so the only thing that can tell them apart is the marker. The case now asserts the assertion
/// the adversary wrote: a reference to evidence answers the evidence, and a reference to a node
/// with those bits answers the node.
///
/// That `CanonicalRef::<Evidence>::new` no longer accepts a `NodeId` at all is a build failure,
/// not a case: `tests/compile_fail/a_canonical_reference_holds_the_id_of_its_kind.rs`.
#[test]
fn a_reference_to_evidence_resolves_to_evidence_and_never_to_a_node() {
    let root = GraphRoot {
        id: GraphRootId::mint(),
        space: Space::Canonical,
        schema_version_id: SchemaVersionId::mint(),
        parent: None,
        created_at: Timestamp::EPOCH,
    };
    let bits = NodeId::mint().to_uuid();
    let held = Node::new(NodeId::from_uuid(bits), root.id, TypeId::mint(), "held");
    let retained = Evidence {
        id: EvidenceId::from_uuid(bits),
        source: EvidenceSource::HumanStatement { identity: None },
        content_hash: ContentHash::of_bytes(b"retained"),
        extracted_by: AgentId::mint(),
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::CERTAIN,
    };
    let with_both = CanonicalGraph {
        root,
        revision: RevisionNumber::SEED,
        ontology: ontology(),
        nodes: [(held.id, held.clone())].into_iter().collect(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: [(retained.id, retained.clone())].into_iter().collect(),
    };
    let nodes_only = CanonicalGraph {
        evidence: BTreeMap::new(),
        ..with_both.clone()
    };

    let to_evidence: CanonicalRef<Evidence> = CanonicalRef::new(EvidenceId::from_uuid(bits));
    let to_node: CanonicalRef<Node> = CanonicalRef::new(NodeId::from_uuid(bits));

    let answered: Option<&Evidence> = with_both.resolve(&to_evidence);
    assert_eq!(
        answered,
        Some(&retained),
        "a CanonicalRef<Evidence> resolves against the evidence map and answers the evidence"
    );
    assert_eq!(
        with_both.resolve(&to_node),
        Some(&held),
        "a CanonicalRef<Node> with the same bits still answers the node"
    );
    assert_eq!(
        nodes_only.resolve(&to_evidence),
        None,
        "a graph holding a node with the reference's bits and no evidence does not resolve a \
         reference to evidence: the marker, and not the bits, decides where it looks"
    );
}
