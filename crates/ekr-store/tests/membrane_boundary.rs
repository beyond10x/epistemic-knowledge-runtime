//! Serialization remains a DTO operation. Semantic crossings moved to kernel/tests/seed.rs.
use ekr_core::{GraphRootId, NodeId, PropertyId, TypeId};
use ekr_graph::{CanonicalValue, Node};
use ekr_ontology::Value;

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
