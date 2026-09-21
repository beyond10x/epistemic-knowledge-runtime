//! Node types, edge types and property definitions: design § 11.1, § 11.2 and § 12, plus the
//! lifecycle and operations amendment 87 adds to a node type.
//!
//! These are data. Nothing here decides whether a value satisfies a definition — that is
//! [`crate::check`] — and nothing here decides whether a definition is coherent, which is
//! [`crate::schema::Ontology::load`]. A `NodeType` on its own is a record someone wrote down.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{PropertyId, TypeId};
use serde::{Deserialize, Serialize};

use crate::lifecycle::{Lifecycle, OperationDefinition};
use crate::value::{Cardinality, ValueType};

/// A property of a node type or an edge type: design § 11.2, and
/// `ekr.ontology.PropertyDefinition`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyDefinition {
    /// The property's stable id. A name is a property of a property, not its identity.
    pub id: PropertyId,
    /// What a reader calls it.
    pub name: String,
    /// The type every value of it must have.
    pub value_type: ValueType,
    /// How many values of it a node may carry.
    #[serde(default)]
    pub cardinality: Cardinality,
    /// Whether a node must carry at least one.
    #[serde(default)]
    pub required: bool,
    /// Design § 11.2 `constraints: Vec<Constraint>`. The constraint language is not defined by the
    /// design, and `systems/ekr/domains/ontology.yaml` marks it `UNMAPPED`, so these are carried
    /// as opaque text and refuse nothing. Inventing a language here would put a guess in a schema.
    #[serde(default)]
    pub constraints: Vec<String>,
}

impl PropertyDefinition {
    /// A single, optional, unconstrained property of this type.
    #[must_use]
    pub fn new(id: PropertyId, name: impl Into<String>, value_type: ValueType) -> Self {
        Self {
            id,
            name: name.into(),
            value_type,
            cardinality: Cardinality::One,
            required: false,
            constraints: Vec::new(),
        }
    }
}

/// A node type: design § 11.1, extended by amendment 87, and `ekr.ontology.NodeType`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeType {
    /// The type's stable id.
    pub id: TypeId,
    /// What a reader calls it.
    pub name: String,
    /// The types this one specialises. A node of this type carries their properties too.
    #[serde(default)]
    pub parents: BTreeSet<TypeId>,
    /// The properties this type declares itself, by id.
    #[serde(default)]
    pub properties: BTreeMap<PropertyId, PropertyDefinition>,
    /// Whether the type is only ever specialised. An abstract type has no nodes.
    #[serde(default)]
    pub abstract_type: bool,
    /// The lifecycle of a node of this type, if it has one (amendment 87).
    #[serde(default)]
    pub lifecycle: Option<Lifecycle>,
    /// The named operations of this type, by name (amendment 87).
    #[serde(default)]
    pub operations: BTreeMap<String, OperationDefinition>,
}

impl NodeType {
    /// A concrete type with no parents, no properties, no lifecycle and no operations.
    #[must_use]
    pub fn new(id: TypeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            parents: BTreeSet::new(),
            properties: BTreeMap::new(),
            abstract_type: false,
            lifecycle: None,
            operations: BTreeMap::new(),
        }
    }
}

/// An edge type: design § 12, and `ekr.ontology.EdgeType`. Relations have schemas just like nodes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeType {
    /// The type's stable id.
    pub id: TypeId,
    /// What a reader calls it.
    pub name: String,
    /// The node types an edge of this type may start at. Empty is refused at load.
    #[serde(default)]
    pub source_types: BTreeSet<TypeId>,
    /// The node types an edge of this type may end at. Empty is refused at load.
    #[serde(default)]
    pub target_types: BTreeSet<TypeId>,
    /// How many edges of this type one source may have.
    #[serde(default)]
    pub cardinality: Cardinality,
    /// The properties an edge of this type declares.
    #[serde(default)]
    pub properties: BTreeMap<PropertyId, PropertyDefinition>,
    /// The edge type that is this one read backwards, if there is one.
    #[serde(default)]
    pub inverse: Option<TypeId>,
    /// Whether the relation holds in both directions.
    #[serde(default)]
    pub symmetric: bool,
    /// Whether the relation composes with itself.
    #[serde(default)]
    pub transitive: bool,
}

impl EdgeType {
    /// An edge type with no endpoints declared yet — which is not loadable until they are.
    #[must_use]
    pub fn new(id: TypeId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            source_types: BTreeSet::new(),
            target_types: BTreeSet::new(),
            cardinality: Cardinality::One,
            properties: BTreeMap::new(),
            inverse: None,
            symmetric: false,
            transitive: false,
        }
    }
}
