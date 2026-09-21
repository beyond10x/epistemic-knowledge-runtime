//! Design § 23: "No equivalent `CanonicalGraph -> TransientRef` should exist."
//!
//! `CanonicalGraph::resolve` takes a `CanonicalDependency`, and `TransientRef` is not one — not
//! because a branch refuses it at run time, but because the bound does not hold. The function
//! below needs no values at all: the membrane is a property of the types.

use ekr_graph::{CanonicalGraph, Node, TransientRef};

fn resolve_a_transient_reference(graph: &CanonicalGraph, reference: &TransientRef<Node>) {
    let _ = graph.resolve(reference);
}

fn main() {}
