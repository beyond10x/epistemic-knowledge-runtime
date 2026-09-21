//! The membrane holds only if the trait that states it cannot be joined from outside.
//!
//! Without the seal, a crate above this one answers
//! `canonical_graph_rejects_a_transient_ref.rs` by implementing `CanonicalDependency` for its own
//! reference type — and AGENTS.md invariant 2 becomes a convention again.

use ekr_core::NodeId;
use ekr_graph::CanonicalDependency;

struct Smuggled(NodeId);

impl CanonicalDependency for Smuggled {
    fn node(&self) -> NodeId {
        self.0
    }
}

fn main() {}
