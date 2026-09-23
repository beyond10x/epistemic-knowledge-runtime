//! AGENTS.md invariant 2, for the edge arm of a canonical assertion's subject.
//!
//! `Subject::Edge` held a bare `EdgeId` beside `Subject::Node(CanonicalRef<Node>)`, so a canonical
//! claim about an edge was written with an id and the crossing was refused by the kernel at commit
//! time — which is the word the invariant excludes. The arm is a `CanonicalRef<Edge>` in canonical
//! state: neither a bare id nor a transient reference to an edge inhabits it.

use ekr_core::EdgeId;
use ekr_graph::{Edge, Subject, TransientRef};

fn a_canonical_subject_from_a_bare_edge_id(edge: EdgeId) -> Subject {
    Subject::Edge(edge)
}

fn a_canonical_subject_from_a_transient_edge(edge: TransientRef<Edge>) -> Subject {
    Subject::Edge(edge)
}

fn main() {}
