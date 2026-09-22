//! Nodes: the things the graph holds, and the identity that survives what they are called.
//!
//! Design § 10 and amendment 87, projected from `ekr.graph.Node` of
//! `systems/ekr/domains/graph.yaml`.

use std::collections::BTreeMap;

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{GraphRootId, NodeId, PropertyId, TypeId};
use serde::{Deserialize, Serialize};

use crate::value::CanonicalValue;

/// One node of a graph root: `ekr.graph.Node` of `systems/ekr/domains/graph.yaml`.
///
/// # A name is not an identity
///
/// Design § 6.4 and § 10, and AGENTS.md invariant 3: `canonical_name` and `aliases` are
/// properties, and [`id`](Node::id) is minted once. A rename writes a new `canonical_name`; it
/// does not produce a new node, and nothing downstream has to notice.
///
/// # Two field shapes that are not the domain's, and why
///
/// * `properties` is keyed by [`PropertyId`], where `graph.yaml` declares
///   `Map<String, ekr.graph.TypedValue>`. A map keyed by the property's *name* makes the name an
///   identity, which AGENTS.md invariant 3 says it is not — a property renamed in a schema
///   transaction would silently orphan every value stored under the old key.
/// * a property's value is a `V`, where the domain carries a `TypedValue { kind, canonical }`.
///   That is the same flattening `ekr.ontology.PropertyDefinition` applies to `ValueType`, and for
///   the same stated reason: `ess/1` types are not recursive, so a `List` or a `Record` value
///   cannot be expressed there. The crate holds the recursive value the type checker needs.
///
/// Both are reported rather than reconciled; `systems/` is not this crate's to edit.
///
/// # Why the value is a parameter
///
/// `architecture-decision-record:0005-float-is-not-canonical`, as amended after adversary pass 1.
/// Canonical state holds [`CanonicalValue`], which has no float at any depth, and that is the
/// default — `Node` on its own is `Node<CanonicalValue>` and every canonical use reads unchanged.
/// [`TransientGraph`](crate::TransientGraph) holds `Node<ekr_ontology::Value>`, because a candidate
/// that has not crossed the integrity boundary may carry an approximate measurement and refusing
/// one at the incubation boundary would invert what that boundary is for.
///
/// **[`Canonical`] is implemented only where `V` is**, which is the part the parameter buys:
/// `Node<CanonicalValue>` has a content address and `Node<Value>` does not, so *only canonical
/// state can be content-addressed* is a property of the type system rather than a convention.
/// `tests/compile_fail/transient_state_has_no_content_address.rs` is that as a build failure.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node<V = CanonicalValue> {
    /// The node's stable id, minted once and never derived from anything a person can edit.
    pub id: NodeId,
    /// The graph root that owns it.
    pub root_id: GraphRootId,
    /// The node type it is an instance of, declared by the ontology.
    pub type_id: TypeId,
    /// The name a person reads. A property, not an identity.
    pub canonical_name: String,
    /// Names it has also been known by — a rename keeps the old one here rather than losing it.
    pub aliases: Vec<String>,
    /// The state of this node in its type's lifecycle (amendment 87): whether a decision is
    /// `open` or `decided`, whether a person has departed. `None` when the type declares no
    /// lifecycle.
    ///
    /// The world's lifecycle, not the runtime's knowledge of it — design § 29's `KnowledgeState`
    /// is a different ladder and arrives in P3.
    pub type_state: Option<String>,
    /// Its property values, by the property's id.
    #[serde(
        deserialize_with = "crate::node::property_values",
        bound(deserialize = "V: Deserialize<'de>")
    )]
    pub properties: BTreeMap<PropertyId, Vec<V>>,
}

pub(crate) fn property_values<'de, D, V>(
    decoder: D,
) -> Result<BTreeMap<PropertyId, Vec<V>>, D::Error>
where
    D: serde::Deserializer<'de>,
    V: Deserialize<'de>,
{
    let values: BTreeMap<PropertyId, Vec<V>> = ekr_core::decode::unique_map(decoder)?;
    if values.values().any(Vec::is_empty) {
        return Err(serde::de::Error::custom(
            "empty outer property values must be absent",
        ));
    }
    Ok(values)
}

impl<V> Node<V> {
    /// A node with a name and nothing else: no aliases, no lifecycle state, no properties.
    ///
    /// The fields are public, so a caller that needs more sets it. There is no writer here and no
    /// `rename`: this crate is the data model, and every change to canonical state arrives as a
    /// transaction the kernel commits.
    #[must_use]
    pub fn new(
        id: NodeId,
        root_id: GraphRootId,
        type_id: TypeId,
        canonical_name: impl Into<String>,
    ) -> Self {
        Self {
            id,
            root_id,
            type_id,
            canonical_name: canonical_name.into(),
            aliases: Vec::new(),
            type_state: None,
            properties: BTreeMap::new(),
        }
    }
}

impl<V: Canonical> Canonical for Node<V> {
    /// The seven fields in declaration order.
    ///
    /// Structural, with no discriminant: a node is not a sum type, and rule 5 of
    /// `ekr_core::canonical` says a composite value's own field structure is what distinguishes
    /// it. The order is the contract — moving a field moves every address that contains this node,
    /// and `Root.knowledge_root` is an address over graph state, which is a root's nodes, its
    /// edges and its assertions.
    ///
    /// Bounded on `V`, so this exists for `Node<CanonicalValue>` and not for `Node<Value>`: the
    /// address exists exactly where canonical state does.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.root_id.encode(out);
        self.type_id.encode(out);
        self.canonical_name.encode(out);
        self.aliases.encode(out);
        out.option(self.type_state.as_ref());
        self.properties.encode(out);
    }
}
