//! AGENTS.md invariant 2, for the node references a canonical claim holds.
//!
//! `Subject::Node`, `Object::Node` and `CanonicalValue::NodeRef` are canonical references to nodes
//! in canonical state (`architecture-decision-record:0008-canonical-state-references-are-typed`).
//! No case held them until wave p1-14: an edge's two ends had one
//! (`tests/review_p1_compile_fail/a_canonical_edge_may_target_a_candidate_node.rs`), the three
//! places a claim names a node did not. Neither a bare id nor a transient reference inhabits any of
//! them.

use ekr_core::NodeId;
use ekr_graph::{CanonicalValue, Node, Object, Subject, TransientRef};

fn a_canonical_subject_from_a_bare_node_id(node: NodeId) -> Subject {
    Subject::Node(node)
}

fn a_canonical_object_from_a_transient_node(node: TransientRef<Node>) -> Object {
    Object::Node(node)
}

fn a_canonical_value_from_a_bare_node_id(node: NodeId) -> CanonicalValue {
    CanonicalValue::NodeRef(node)
}

fn main() {}
