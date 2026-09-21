//! Where the type-level membrane stops, written down as cases rather than assumed.
//!
//! `task:the-membrane-stops-at-the-store-boundary`. `ekr_graph::CanonicalValue` serialises
//! *through* `ekr_ontology::Value`, so a canonical node and an incubation-forest candidate produce
//! byte-identical documents and nothing on a stored node records which space it came from. The
//! guarantee ADR 0005's amendment states — the address exists exactly where canonical state does —
//! holds inside Rust and stops exactly where this crate begins.
//!
//! The decision this crate took is the smallest of the task's three: **the store never
//! deserialises a document straight into a canonical type.** Every document goes through
//! [`ekr_store::GraphDocument`], which holds the *transient* instantiation —
//! `Node<Value>`, `Edge<Value>`, `Assertion<Value>` — and
//! [`GraphDocument::into_canonical`](ekr_store::GraphDocument::into_canonical) is the one named
//! place the crossing happens.
//!
//! That does not make a candidate's document distinguishable from a canonical one. Nothing can,
//! while the bytes are identical; the first case below pins exactly that, so the sentence is a
//! checked fact rather than an assumption a later reader has to rediscover. What the decision buys
//! is that there is **one** place to put a stronger check the day one is wanted — a space marker on
//! the document, or distinct serde representations — rather than however many deserialisation
//! sites the store grew in the meantime.

mod fixture;

use std::collections::BTreeMap;

use ekr_core::{EvidenceId, GraphRootId, NodeId, PropertyId, TypeId};
use ekr_graph::{CanonicalValue, Node};
use ekr_ontology::Value;
use ekr_store::{Entity, GraphDocument, MembraneError};

/// A candidate's node and a canonical node carrying the same value produce the same bytes.
///
/// This is the finding, as a case. It is not a defect being asserted into permanence: it is the
/// statement that the membrane is carried by the *type*, not by the document, and the day a
/// document carries it too is the day this case changes.
#[test]
fn a_candidate_node_and_a_canonical_node_write_the_same_document() {
    let (node_id, root_id, type_id, property) = (
        NodeId::mint(),
        GraphRootId::mint(),
        TypeId::mint(),
        PropertyId::mint(),
    );

    let mut candidate: Node<Value> = Node::new(node_id, root_id, type_id, "an-incubating-claim");
    candidate
        .properties
        .insert(property, Value::Decimal("1.0".to_owned()));

    let mut canonical: Node<CanonicalValue> =
        Node::new(node_id, root_id, type_id, "an-incubating-claim");
    canonical
        .properties
        .insert(property, CanonicalValue::Decimal("1.0".to_owned()));

    assert_eq!(
        serde_json::to_string(&candidate).expect("a candidate serialises"),
        serde_json::to_string(&canonical).expect("a canonical node serialises"),
        "the two spaces write byte-identical documents; nothing on a node records its space"
    );
}

/// The one named crossing refuses a document canonical state cannot hold, and names where.
#[test]
fn the_crossing_refuses_a_float_and_says_which_property_carried_it() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let mut document = GraphDocument::of(&graph);
    let node_id = *graph.nodes.keys().next().expect("the seed has a node");
    let property = *graph.nodes[&node_id]
        .properties
        .keys()
        .next()
        .expect("the seed's node has a property");
    document
        .nodes
        .get_mut(&node_id)
        .expect("the document carries that node")
        .properties
        .insert(property, Value::Float(0.1));

    let refused = document.into_canonical(ontology);
    assert!(
        matches!(
            refused,
            Err(MembraneError::Node { node_id: refused_node, property: refused_property, .. })
                if refused_node == node_id && refused_property == property
        ),
        "a float in a property is refused at the crossing, and the refusal names it: {refused:?}"
    );
}

/// A float in an assertion's object is refused at the same place, not a second one.
#[test]
fn the_crossing_refuses_a_float_in_an_assertions_object() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let mut document = GraphDocument::of(&graph);
    let assertion_id = *graph
        .assertions
        .keys()
        .next()
        .expect("the seed has an assertion");
    document
        .assertions
        .get_mut(&assertion_id)
        .expect("the document carries that assertion")
        .object = ekr_graph::Object::Value(Value::Float(f64::NAN));

    let refused = document.into_canonical(ontology);
    assert!(
        matches!(
            refused,
            Err(MembraneError::Assertion { assertion_id: refused_id, .. })
                if refused_id == assertion_id
        ),
        "an assertion's object crosses at the same place: {refused:?}"
    );
}

/// A float on an edge property is refused there too.
#[test]
fn the_crossing_refuses_a_float_on_an_edge() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let mut document = GraphDocument::of(&graph);
    let edge_id = *graph.edges.keys().next().expect("the seed has an edge");
    let property = *graph.edges[&edge_id]
        .properties
        .keys()
        .next()
        .expect("the seed's edge has a property");
    document
        .edges
        .get_mut(&edge_id)
        .expect("the document carries that edge")
        .properties
        .insert(property, Value::Float(-0.0));

    let refused = document.into_canonical(ontology);
    assert!(
        matches!(
            refused,
            Err(MembraneError::Edge { edge_id: refused_edge, property: refused_property, .. })
                if refused_edge == edge_id && refused_property == property
        ),
        "an edge property crosses at the same place: {refused:?}"
    );
}

/// The crossing is total in the other direction: every canonical graph makes a document, and every
/// document a canonical graph made comes back.
#[test]
fn a_canonical_graph_round_trips_through_its_document() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let bytes = GraphDocument::of(&graph)
        .to_bytes()
        .expect("a canonical graph serialises");
    let back = GraphDocument::from_bytes(&bytes)
        .expect("its document reads back")
        .into_canonical(ontology)
        .expect("and crosses, because it was canonical when it was written");

    assert_eq!(back.nodes, graph.nodes);
    assert_eq!(back.edges, graph.edges);
    assert_eq!(back.assertions, graph.assertions);
    assert_eq!(back.evidence, graph.evidence);
    assert_eq!(back.root, graph.root);
    assert_eq!(back.revision, graph.revision);
}

/// An empty document still crosses: the refusal is about what is in a document, not about size.
#[test]
fn an_empty_document_crosses() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let mut document = GraphDocument::of(&graph);
    document.nodes = BTreeMap::new();
    document.edges = BTreeMap::new();
    document.assertions = BTreeMap::new();
    document.evidence = BTreeMap::new();

    let crossed = document
        .into_canonical(ontology)
        .expect("a document with nothing inadmissible in it crosses");
    assert!(crossed.nodes.is_empty());
}

// The rest of the class the adversary's two findings belong to.
//
// Findings 1 and 2 of correction round 1 were both "a field the crossing copies through without
// reading". Fixing those two and stopping would leave the class open, so the crossing now reads
// every field of a document that can be read against another field of the same document or against
// what the caller supplied, and the cases below are the members no finding named. The fields that
// *cannot* be read against anything are enumerated in the last case, so the bound is written down
// rather than implied by the absence of a case.

/// A record filed under a key that is not its own id is refused.
///
/// `CanonicalGraph::resolve` answers from the map, so a node filed under another node's id makes a
/// lookup return a record that is not the one asked for — silently, and with no float and no wrong
/// space to notice.
#[test]
fn the_crossing_refuses_a_record_filed_under_another_records_id() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let mut document = GraphDocument::of(&graph);
    let node_id = *graph.nodes.keys().next().expect("the seed has a node");
    let node = document.nodes.remove(&node_id).expect("that node");
    let wrong_key = NodeId::mint();
    document.nodes.insert(wrong_key, node);

    let refused = document.into_canonical(ontology);
    assert!(
        matches!(
            refused,
            Err(MembraneError::Misfiled { key, found })
                if key == Entity::Node(wrong_key) && found == Entity::Node(node_id)
        ),
        "a record filed under the wrong key is refused, and both ids are named: {refused:?}"
    );
}

/// A record belonging to another graph root is refused.
///
/// `root_id` is on every node, edge and assertion, and the document's own root carries the id they
/// should all name. A record carrying a different one is another root's state travelling inside
/// this root's document — which is the same crossing defect as a transient root, one level down.
#[test]
fn the_crossing_refuses_a_record_that_belongs_to_another_graph_root() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let mut document = GraphDocument::of(&graph);
    let edge_id = *graph.edges.keys().next().expect("the seed has an edge");
    let elsewhere = GraphRootId::mint();
    document.edges.get_mut(&edge_id).expect("that edge").root_id = elsewhere;

    let refused = document.into_canonical(ontology);
    assert!(
        matches!(
            refused,
            Err(MembraneError::Misrooted { entity, root_id, document_root })
                if entity == Entity::Edge(edge_id)
                    && root_id == elsewhere
                    && document_root == graph.root.id
        ),
        "a record naming another root is refused, and both roots are named: {refused:?}"
    );
}

/// The same check reaches assertions and evidence, not only the map the first case used.
///
/// A rule applied to one of four maps is a rule with three holes in it, and the three would each
/// have to be found separately.
#[test]
fn every_map_of_a_document_is_checked_and_not_only_the_first() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let assertion_id = *graph
        .assertions
        .keys()
        .next()
        .expect("the seed has an assertion");
    let mut misrooted = GraphDocument::of(&graph);
    misrooted
        .assertions
        .get_mut(&assertion_id)
        .expect("that assertion")
        .root_id = GraphRootId::mint();
    assert!(
        matches!(
            misrooted.into_canonical(ontology.clone()),
            Err(MembraneError::Misrooted { entity, .. }) if entity == Entity::Assertion(assertion_id)
        ),
        "assertions carry a root_id and it is read"
    );

    let evidence_id = *graph.evidence.keys().next().expect("the seed has evidence");
    let mut misfiled = GraphDocument::of(&graph);
    let evidence = misfiled
        .evidence
        .remove(&evidence_id)
        .expect("that evidence");
    let wrong_key = EvidenceId::mint();
    misfiled.evidence.insert(wrong_key, evidence);
    assert!(
        matches!(
            misfiled.into_canonical(ontology),
            Err(MembraneError::Misfiled { key, found })
                if key == Entity::Evidence(wrong_key) && found == Entity::Evidence(evidence_id)
        ),
        "evidence has no root_id to check, and its filing is still read"
    );
}

/// What the crossing cannot check, written down as a case rather than left to the absence of one.
///
/// `root.created_at` and `revision` have nothing in the document to disagree with, so a document
/// carrying any value in them crosses.
///
/// `root.parent` is the **narrowed** entry, and it was wrong here for one round: a parent cannot be
/// *resolved* — nothing in a document says which other roots exist — but it can be read against
/// `root.id`, and `adversary2_membrane_bounds.rs` holds that a root which is its own parent is
/// refused. The parent used below is freshly minted and therefore not this root, which is the case
/// that stays true: an unresolvable parent crosses, a self-referential one does not.
///
/// An assertion citing evidence the document does not carry also crosses, and that one is not a
/// bound but a **placement**: design § 20's reference validator is `ekr-kernel`'s, above this crate,
/// along with the type and ontology-constraint validators the module documentation lists.
#[test]
fn the_crossing_does_not_check_what_it_has_nothing_to_check_against() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let mut document = GraphDocument::of(&graph);
    document.root.parent = Some(GraphRootId::mint());
    document.root.created_at = ekr_core::Timestamp::from_millis(1_773_273_600_000);
    document.revision = ekr_core::RevisionNumber::new(999);
    let assertion_id = *graph
        .assertions
        .keys()
        .next()
        .expect("the seed has an assertion");
    document
        .assertions
        .get_mut(&assertion_id)
        .expect("that assertion")
        .evidence = [ekr_core::EvidenceId::mint()].into_iter().collect();

    let crossed = document
        .into_canonical(ontology)
        .expect("none of those four is readable against anything this crossing holds");
    assert_eq!(
        crossed.revision,
        ekr_core::RevisionNumber::new(999),
        "the revision crosses unread; the fold is what sets it"
    );
    assert!(
        crossed
            .root
            .parent
            .is_some_and(|parent| parent != crossed.root.id),
        "a parent root nothing can resolve crosses, because it is not this root"
    );
}
