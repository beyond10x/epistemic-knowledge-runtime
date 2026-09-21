//! A name is a property; an id is not — design § 6.4 and § 10, against the real `Node`.
//!
//! `story:kernel-identity-and-hashing` shipped this invariant against a fixture struct, because
//! `Node` did not exist in `ekr-core` and could not: AGENTS.md invariant 8 keeps domain concepts
//! out of the crates below `ekr-graph`. A case over a stand-in cannot fail for the reason it
//! exists — it tests the stand-in. `Node` exists now, so the case is asked of it.

use ekr_core::{GraphRootId, NodeId, TypeId};
use ekr_graph::Node;

#[test]
fn a_node_renamed_a_thousand_times_keeps_its_id() {
    let id = NodeId::mint();
    let root = GraphRootId::mint();
    let mut node: Node = Node::new(id, root, TypeId::mint(), "Acme");

    for generation in 0..1000 {
        let previous = std::mem::replace(&mut node.canonical_name, format!("Acme {generation}"));
        node.aliases.push(previous);
        assert_eq!(
            node.id, id,
            "a rename moved the identity at generation {generation}"
        );
    }

    assert_eq!(node.canonical_name, "Acme 999");
    assert_eq!(node.aliases.len(), 1000);
    assert_eq!(node.aliases[0], "Acme");
    assert_eq!(node.id, id);
    assert_eq!(node.root_id, root);
}

#[test]
fn two_nodes_that_share_a_name_do_not_share_an_id() {
    let root = GraphRootId::mint();
    let type_id = TypeId::mint();
    let one: Node = Node::new(NodeId::mint(), root, type_id, "Acme");
    let other: Node = Node::new(NodeId::mint(), root, type_id, "Acme");

    assert_eq!(one.canonical_name, other.canonical_name);
    assert_ne!(one.id, other.id, "a name became an identity");
}

/// Amendment 87: a node's lifecycle state is the world's state — whether a decision is decided —
/// and it is a property like any other, so it moves without the id moving.
#[test]
fn the_lifecycle_state_of_a_node_is_a_property_and_not_its_identity() {
    let id = NodeId::mint();
    let mut node: Node = Node::new(id, GraphRootId::mint(), TypeId::mint(), "Adopt eventlog");

    assert_eq!(
        node.type_state, None,
        "a node starts with no declared state"
    );
    node.type_state = Some("open".to_owned());
    node.type_state = Some("decided".to_owned());

    assert_eq!(node.type_state.as_deref(), Some("decided"));
    assert_eq!(node.id, id);
    assert!(node.properties.is_empty());
}
