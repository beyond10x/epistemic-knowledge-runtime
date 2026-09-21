//! Typed edges: design § 12, and `ekr.graph.Edge` of `systems/ekr/domains/graph.yaml`.

use std::collections::BTreeMap;

use ekr_core::{EdgeId, GraphRootId, NodeId, PropertyId, TypeId};
use ekr_ontology::Value;
use serde::{Deserialize, Serialize};

/// One directed, typed relation between two nodes.
///
/// Design § 12 also gives an edge `provenance` and a `validation` state. Neither is a field here:
/// both are properties of the *claim* that the relation holds, and a claim is an
/// [`Assertion`](crate::Assertion) — which carries its evidence, its validation state and its two
/// time dimensions, and can be retracted without the edge ceasing to have existed.
/// `graph.yaml`'s `ekr.graph.Edge` declares neither either.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// The edge's stable id.
    pub id: EdgeId,
    /// The graph root that owns it.
    pub root_id: GraphRootId,
    /// The edge type it is an instance of.
    pub type_id: TypeId,
    /// The node the relation runs from.
    pub source: NodeId,
    /// The node it runs to.
    pub target: NodeId,
    /// Its property values, by the property's id — keyed as [`Node::properties`](crate::Node) is,
    /// and for the same reason.
    pub properties: BTreeMap<PropertyId, Value>,
}

impl Edge {
    /// An edge with no properties.
    #[must_use]
    pub fn new(
        id: EdgeId,
        root_id: GraphRootId,
        type_id: TypeId,
        source: NodeId,
        target: NodeId,
    ) -> Self {
        Self {
            id,
            root_id,
            type_id,
            source,
            target,
            properties: BTreeMap::new(),
        }
    }
}
