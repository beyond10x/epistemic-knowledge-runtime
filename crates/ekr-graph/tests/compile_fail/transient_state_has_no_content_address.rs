//! The address exists exactly where canonical state does.
//!
//! `architecture-decision-record:0005-float-is-not-canonical`, as amended after adversary pass 1:
//! `Node`, `Edge` and `Assertion` are generic over the value they carry, and `Canonical` is
//! implemented only where that parameter is itself `Canonical`. `ekr_ontology::Value` is not —
//! rule 4 of `ekr_core::canonical` admits no float and `Value` has a `Float` variant — so the
//! three types a transient root holds have no content address.
//!
//! All three, and not only the one the amendment's example names: an address over graph state is
//! an address over nodes, edges and assertions, so a hole in any of them is a hole in it.

use ekr_core::ContentHash;
use ekr_graph::{Assertion, Edge, Node};
use ekr_ontology::Value;

fn address_of_a_candidate_node(node: &Node<Value>) -> ContentHash {
    ContentHash::of(node)
}

fn address_of_a_candidate_edge(edge: &Edge<Value>) -> ContentHash {
    ContentHash::of(edge)
}

fn address_of_a_candidate_assertion(assertion: &Assertion<Value>) -> ContentHash {
    ContentHash::of(assertion)
}

fn main() {}
