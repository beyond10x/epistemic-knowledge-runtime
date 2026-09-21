//! Nodes: the things the graph holds, and the identity that survives what they are called.
//!
//! Design § 10 and amendment 87, projected from `ekr.graph.Node` of
//! `systems/ekr/domains/graph.yaml`.

use std::collections::BTreeMap;

use ekr_core::{GraphRootId, NodeId, PropertyId, TypeId};
use ekr_ontology::Value;
use serde::{Deserialize, Serialize};

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
/// * a property's value is an [`ekr_ontology::Value`], where the domain carries a
///   `TypedValue { kind, canonical }`. That is the same flattening `ekr.ontology.PropertyDefinition`
///   applies to `ValueType`, and for the same stated reason: `ess/1` types are not recursive, so a
///   `List` or a `Record` value cannot be expressed there. The crate holds the recursive value the
///   type checker needs.
///
/// Both are reported rather than reconciled; `systems/` is not this crate's to edit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
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
    pub properties: BTreeMap<PropertyId, Value>,
}

impl Node {
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
