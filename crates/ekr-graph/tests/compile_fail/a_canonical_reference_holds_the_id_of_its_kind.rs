//! A canonical reference holds the id of the kind it names, and no other.
//!
//! `task:canonical-reference-holds-a-node-id-for-every-target`: `CanonicalRef<T>` used to store a
//! `NodeId` whatever `T` was, so a reference that said it pointed at evidence was built from a node
//! id and resolved against the nodes map. `CanonicalTarget` now names each kind's id type, and a
//! reference to evidence is made from an `EvidenceId` or not at all.

use ekr_core::{EvidenceId, NodeId};
use ekr_graph::{CanonicalRef, Evidence, Node};

fn a_reference_to_evidence_from_a_node_id(node: NodeId) -> CanonicalRef<Evidence> {
    CanonicalRef::new(node)
}

fn a_reference_to_a_node_from_an_evidence_id(evidence: EvidenceId) -> CanonicalRef<Node> {
    CanonicalRef::new(evidence)
}

fn main() {}
