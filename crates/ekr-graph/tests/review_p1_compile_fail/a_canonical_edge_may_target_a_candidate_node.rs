//! AGENTS.md invariant 2, asserted as a build failure over the types canonical state is made of.
//!
//! > A `Canonical → Transient` reference is unrepresentable at the type level, not merely refused.
//!
//! The function below takes a `TransientGraph`, reads the id of one of its candidates, and builds
//! a `CanonicalGraph` holding an `Edge` whose `target` is that id. If the invariant holds of
//! canonical state, this is not a type and the file does not compile. `trybuild` expects exactly
//! that; the case is red because the file compiles.
//!
//! The four cases in `tests/compile_fail/` are not disputed: `CanonicalRef`, `CanonicalDependency`
//! and `CanonicalTarget` are sealed and sound. This file shows that none of them is a field of
//! `Node`, `Edge`, `Assertion` or `CanonicalGraph`, so the crossing is written with a bare `NodeId`.

use std::collections::BTreeMap;

use ekr_core::{EdgeId, GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TypeId};
use ekr_graph::{CanonicalGraph, Edge, GraphRoot, Node, Space, TransientGraph};
use ekr_ontology::Ontology;

fn a_canonical_edge_into_a_transient_root(
    transient: &TransientGraph,
    ontology: Ontology,
) -> CanonicalGraph {
    let candidate: NodeId = *transient
        .nodes
        .keys()
        .next()
        .expect("the transient root holds a candidate");

    let root = GraphRoot {
        id: GraphRootId::mint(),
        space: Space::Canonical,
        schema_version_id: SchemaVersionId::mint(),
        parent: None,
        created_at: Timestamp::EPOCH,
    };
    let held = Node::new(NodeId::mint(), root.id, TypeId::mint(), "held");
    let into_the_forest = Edge::new(EdgeId::mint(), root.id, TypeId::mint(), held.id, candidate);

    CanonicalGraph {
        root,
        revision: RevisionNumber::SEED,
        ontology,
        nodes: [(held.id, held)].into_iter().collect(),
        edges: [(into_the_forest.id, into_the_forest)]
            .into_iter()
            .collect(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    }
}

fn main() {}
