//! What `architecture-decision-record:0005-float-is-not-canonical` decided, held against what
//! `task:canonical-value-in-the-graph` built.
//!
//! Two claims of that record are checked here, and neither is checked anywhere else in the suite:
//!
//! * **`Float` stays legal in the transient graph.** The ADR's decision says so in as many words:
//!   "`Float` stays legal in `ekr_ontology::Value` and in the transient graph. Nothing there is
//!   content-addressed, and an approximate measurement is a perfectly good thing to hold before it
//!   becomes canonical." [`TransientGraph`] is composed of [`Node`], [`Edge`] and [`Assertion`] —
//!   the *same* three types canonical state holds — so a constraint put on those types for the
//!   sake of content addressing landed on the transient side too, which is what this file's first
//!   two cases measured and what the ADR's amendment of 2026-09-21 answers: the three are generic
//!   over the value they carry, canonical state holds the admissible default and a transient root
//!   holds `ekr_ontology::Value`. The cases now hold the amended claim — a candidate carries a
//!   float — rather than the refusal they found.
//! * **`knowledge_root` becomes computable.** The same decision says the newtype landing in the
//!   two property maps and in `Object::Value` is what makes it so. `Root.knowledge_root` is the
//!   content address of *graph* state, and graph state is a root's nodes, its edges and its
//!   assertions (`systems/ekr/domains/graph.yaml`, `ekr.graph.GraphRoot` owns all three).
//!
//! These are the document's claims, not this file's opinion. Nothing else in the suite compares
//! the record against the code: the implementation's own cases assert the behaviour it built.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ekr_core::{
    AgentId, AssertionId, GraphRootId, NodeId, PropertyId, SchemaVersionId, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, CanonicalValue, GraphRoot, Node, Object, Predicate, Space, Subject, TemporalRange,
    TransactionTime, TransientGraph, ValidationState,
};
use ekr_ontology::Value;

/// A transient root: the space the ADR says an approximate measurement may sit in.
fn transient_root() -> GraphRoot {
    GraphRoot {
        id: GraphRootId::mint(),
        space: Space::Transient,
        schema_version_id: SchemaVersionId::mint(),
        parent: None,
        created_at: Timestamp::EPOCH,
    }
}

/// A candidate node of a transient root may carry a float-valued property.
///
/// `TransientGraph.nodes` is a `BTreeMap<NodeId, Node<Value>>`, so the value goes in as itself:
/// there is no conversion to pass and nothing to refuse it. The same property in a canonical node
/// is a [`CanonicalValue`] and a float cannot reach it — which is the asymmetry the amended ADR
/// asks for, and the reason the second half of this case checks that the admissible side still
/// refuses.
#[test]
fn a_transient_candidate_node_may_hold_an_approximate_measurement() {
    let root = transient_root();
    let candidate = NodeId::mint();
    let property = PropertyId::mint();
    let mut node: Node<Value> = Node::new(candidate, root.id, TypeId::mint(), "A. Smith?");
    node.properties.insert(property, Value::Float(36.6));

    let transient = TransientGraph {
        root,
        nodes: [(candidate, node)].into_iter().collect(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
    };
    assert_eq!(transient.nodes.len(), 1, "the candidate is in the root");
    assert_eq!(
        transient.nodes[&candidate].properties[&property],
        Value::Float(36.6),
        "the measurement is held as it arrived, not rounded into admissibility"
    );

    // And the other side of the membrane still refuses it, which is what the decision decided.
    assert!(
        CanonicalValue::try_from(Value::Float(36.6)).is_err(),
        "a float is still inadmissible in canonical state"
    );
}

/// A candidate assertion of a transient root may say something approximate.
///
/// The same reasoning one level over: `TransientGraph.assertions` holds `Assertion<Value>`, whose
/// `Object::Value` arm carries the value itself. A claim that has not crossed the integrity
/// boundary, and may never, is not refused for a reason that is only about addresses.
#[test]
fn a_transient_candidate_assertion_may_carry_an_approximate_measurement() {
    let root = transient_root();
    let held = Value::Float(0.82);

    let id = AssertionId::mint();
    let candidate: Assertion<Value> = Assertion {
        id,
        root_id: root.id,
        subject: Subject::Node(NodeId::mint()),
        predicate: Predicate::Property(PropertyId::mint()),
        object: Object::Value(held),
        evidence: BTreeSet::new(),
        proposed_by: AgentId::mint(),
        validation: ValidationState::Proposed,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };

    let transient = TransientGraph {
        root,
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: [(id, candidate)].into_iter().collect(),
    };
    assert_eq!(
        transient.assertions.len(),
        1,
        "the candidate claim is in the root"
    );
    assert_eq!(
        transient.assertions[&id].object,
        Object::Value(Value::Float(0.82)),
        "the claim says what it arrived saying"
    );
    assert!(
        CanonicalValue::try_from(Value::Float(0.82)).is_err(),
        "and canonical state would still refuse it"
    );
}

/// Every `.rs` file of this crate's `src/`, read.
fn crate_sources() -> Vec<String> {
    let directory = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
    let mut found: Vec<String> = std::fs::read_dir(directory)
        .expect("the crate has a src/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.extension().is_some_and(|e| e == "rs"))
        .map(|path| std::fs::read_to_string(&path).expect("a source file"))
        .collect();
    found.sort();
    assert!(
        found.len() >= 10,
        "the module scan is broken, not the crate"
    );
    found
}

/// Graph state is content-addressable, which is what `Root.knowledge_root` is the address of.
///
/// Read from the source rather than from a trait bound, because a missing implementation is a
/// compile error at the use site and a compile error is not a case anybody can run. The three
/// types are the ones `ekr.graph.GraphRoot` owns: nodes, edges and assertions. An assertion alone
/// is not graph state, so an address computed over assertions alone is not the graph's.
///
/// The head is matched by its shape rather than by a list of spellings — see
/// [`canonical_impl_head`]. The claim is unchanged — each type has an implementation — and
/// `tests/compile_fail/transient_state_has_no_content_address.rs` is what holds the bound itself,
/// which a text scan cannot.
#[test]
fn every_part_of_graph_state_has_a_canonical_encoding() {
    let sources = crate_sources();
    let missing: Vec<&str> = ["Node", "Edge", "Assertion"]
        .into_iter()
        .filter(|type_name| {
            !sources
                .iter()
                .any(|text| canonical_impl_head(text, type_name).is_some())
        })
        .collect();

    assert!(
        missing.is_empty(),
        "ADR 0005 decides that the newtype landing in Object::Value, Node.properties and \
         Edge.properties is what makes Root.knowledge_root computable, and {missing:?} still has \
         no Canonical implementation, so the address of graph state cannot be computed from graph \
         state"
    );
}

/// The `{` that opens `impl … Canonical for <type_name> …`, whatever bounds the implementation
/// carries.
///
/// **Structural, and deliberately not a list of spellings.** The list was two —
/// `impl Canonical for Root {` and `impl<V: Canonical> Canonical for Node<V> {` — and
/// `architecture-decision-record:0008-canonical-state-references-are-typed` added a third,
/// `impl<V: ValueSpace + Canonical> Canonical for Edge<V> {`, at which point a scan enumerating
/// spellings reported the type as having *no implementation at all*. A rule enumerated by its
/// instances has a next instance; this one reads the shape — a line beginning `impl`, naming
/// `Canonical for` the type at an identifier boundary, and opening a block.
fn canonical_impl_head(source: &str, type_name: &str) -> Option<usize> {
    let needle = format!(" Canonical for {type_name}");
    source.match_indices(&needle).find_map(|(at, _)| {
        let line_start = source[..at].rfind('\n').map_or(0, |n| n + 1);
        if !source[line_start..at].trim_start().starts_with("impl") {
            return None;
        }
        // The next character after the name is what keeps `Node` from matching `NodeDraft`.
        if !source[at + needle.len()..].starts_with(['<', ' ', '{']) {
            return None;
        }
        let line_end = source[at..]
            .find('\n')
            .map_or(source.len(), |offset| at + offset);
        source[at..line_end].rfind('{').map(|offset| at + offset)
    })
}
