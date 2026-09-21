//! The way round the other two: a canonical reference *to a transient reference*.
//!
//! `CanonicalGraph::resolve` takes a `CanonicalDependency`, and `CanonicalRef<T>` is one for every
//! `T`. With `T` unbounded, `CanonicalRef<TransientRef<Node>>` was a well-formed type and canonical
//! state accepted it — the bound refused the outer type and said nothing about what it pointed at.
//!
//! AGENTS.md invariant 2 says the crossing is unrepresentable at the type level, so `T` is bounded
//! by the sealed `CanonicalTarget`, which the transient types do not implement and no crate above
//! this one can make them implement.

use ekr_core::NodeId;
use ekr_graph::{CanonicalRef, Node, TransientRef};

fn a_canonical_reference_to_a_transient_reference(
    node: NodeId,
) -> CanonicalRef<TransientRef<Node>> {
    CanonicalRef::new(node)
}

fn main() {}
