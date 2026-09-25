//! Typed edges: design § 12, and `ekr.graph.Edge` of `systems/ekr/domains/graph.yaml`.

use std::collections::BTreeMap;

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{EdgeId, GraphRootId, PropertyId, TypeId};
use serde::{Deserialize, Serialize};

use crate::canonical::ValueSpace;
use crate::value::CanonicalValue;

/// One directed, typed relation between two nodes.
///
/// Design § 12 also gives an edge `provenance` and a `validation` state. Neither is a field here:
/// both are properties of the *claim* that the relation holds, and a claim is an
/// [`Assertion`](crate::Assertion) — which carries its evidence, its validation state and its two
/// time dimensions, and can be retracted without the edge ceasing to have existed.
/// `graph.yaml`'s `ekr.graph.Edge` declares neither either.
///
/// Generic over the value its properties carry, defaulting to [`CanonicalValue`], for the reason
/// and with the consequence [`Node`](crate::Node) is: canonical state holds `Edge<CanonicalValue>`
/// and has an address for it, a transient root holds `Edge<ekr_ontology::Value>` and has none.
///
/// # Its two ends are references, not ids
///
/// `architecture-decision-record:0008-canonical-state-references-are-typed`. [`source`](Edge::source)
/// and [`target`](Edge::target) are `V::NodeRef` — [`CanonicalRef<Node>`](crate::CanonicalRef) where
/// the value is canonical, a bare [`NodeId`](ekr_core::NodeId) where it is a candidate's. A
/// canonical edge into a transient root was a well-formed value of this type until then, which is
/// the word AGENTS.md invariant 2 excludes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(
    feature = "schema",
    schemars(bound = "V: schemars::JsonSchema, V::NodeRef: schemars::JsonSchema")
)]
#[serde(deny_unknown_fields)]
pub struct Edge<V: ValueSpace = CanonicalValue> {
    /// The edge's stable id.
    pub id: EdgeId,
    /// The graph root that owns it.
    pub root_id: GraphRootId,
    /// The edge type it is an instance of.
    pub type_id: TypeId,
    /// The node the relation runs from.
    pub source: V::NodeRef,
    /// The node it runs to.
    pub target: V::NodeRef,
    /// Its property values, by the property's id — keyed as [`Node::properties`](crate::Node) is,
    /// and for the same reason.
    #[cfg_attr(
        feature = "schema",
        schemars(with = "BTreeMap<PropertyId, crate::schema::NonEmptyValues<V>>")
    )]
    #[serde(
        deserialize_with = "crate::node::property_values",
        bound(deserialize = "V: Deserialize<'de>")
    )]
    pub properties: BTreeMap<PropertyId, Vec<V>>,
}

impl<V: ValueSpace> Edge<V> {
    /// An edge with no properties.
    #[must_use]
    pub fn new(
        id: EdgeId,
        root_id: GraphRootId,
        type_id: TypeId,
        source: V::NodeRef,
        target: V::NodeRef,
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

impl<V: ValueSpace + Canonical> Canonical for Edge<V> {
    /// The six fields in declaration order, structural and bounded on `V` exactly as
    /// [`Node`](crate::Node)'s is.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.root_id.encode(out);
        self.type_id.encode(out);
        self.source.encode(out);
        self.target.encode(out);
        self.properties.encode(out);
    }
}
