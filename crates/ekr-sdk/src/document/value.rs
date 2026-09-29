//! Schema declarations and typed values, as a seed's ontology section and the schema operations
//! write them (`docs/cli.md`, "Value types", "Node types", "Edge types").

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{NodeId, PropertyId, Timestamp, TypeId};
use serde::{Deserialize, Serialize};

/// How many values a property may carry, or how many edges of a type may leave one node.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Cardinality {
    /// At most one.
    #[default]
    One,
    /// Any number.
    Many,
}

/// A declared value type: `{value_kind, parameters}`, the eleven kinds of `docs/cli.md`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "value_kind", content = "parameters", deny_unknown_fields)]
pub enum ValueType {
    /// Text.
    String,
    /// A truth value.
    Boolean,
    /// A signed 64-bit whole number.
    Integer,
    /// An approximate number. It may be declared; no Float value is ever committed.
    Float,
    /// An exact number held as text.
    Decimal,
    /// Milliseconds since the Unix epoch, UTC.
    Timestamp,
    /// A signed whole number in a unit the schema states.
    Duration,
    /// A reference to a node of one of these types, or of a subtype of one.
    NodeRef {
        /// The node types a value may point at; at least one.
        allowed_types: BTreeSet<TypeId>,
    },
    /// One of these variants, case-sensitive.
    Enum {
        /// The variants; at least one.
        variants: BTreeSet<String>,
    },
    /// One value that is a sequence of elements of this type.
    List(Box<ValueType>),
    /// One value with exactly these fields.
    Record(BTreeMap<String, ValueType>),
}

/// A value: `{value_kind, value}`, of the kind its property declares.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "value_kind", content = "value", deny_unknown_fields)]
pub enum Value {
    /// Text.
    String(String),
    /// A truth value.
    Boolean(bool),
    /// A signed 64-bit whole number.
    Integer(i64),
    /// An approximate number. Proposable, and refused by validation as `inadmissible-value`.
    Float(f64),
    /// An exact number as text, compared as text: write one canonical form.
    Decimal(String),
    /// Milliseconds since the Unix epoch, UTC.
    Timestamp(Timestamp),
    /// A signed whole number.
    Duration(i64),
    /// A node.
    NodeRef(NodeId),
    /// One declared variant.
    Enum(String),
    /// A sequence of values.
    List(Vec<Value>),
    /// Named fields.
    Record(BTreeMap<String, Value>),
}

/// A property of a node type or an edge type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyDefinition {
    /// The property's id; the key it is filed under.
    pub id: PropertyId,
    /// The name a reader sees; not an identity.
    pub name: String,
    /// What every value must be.
    pub value_type: ValueType,
    /// How many values a node may carry.
    #[serde(default)]
    pub cardinality: Cardinality,
    /// Whether a node must carry at least one value.
    #[serde(default)]
    pub required: bool,
    /// Reserved: any write touching a type with a constraint is refused. Leave it empty.
    #[serde(default)]
    pub constraints: Vec<String>,
}

impl PropertyDefinition {
    /// A single-valued, optional property under a freshly minted id.
    #[must_use]
    pub fn new(name: impl Into<String>, value_type: ValueType) -> Self {
        Self {
            id: PropertyId::mint(),
            name: name.into(),
            value_type,
            cardinality: Cardinality::One,
            required: false,
            constraints: Vec::new(),
        }
    }

    /// This property with `cardinality`.
    #[must_use]
    pub fn with_cardinality(mut self, cardinality: Cardinality) -> Self {
        self.cardinality = cardinality;
        self
    }

    /// This property, required.
    #[must_use]
    pub fn as_required(mut self) -> Self {
        self.required = true;
        self
    }
}

/// One move of a lifecycle.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    /// The state a node must be in.
    pub from: String,
    /// The state it is in afterwards.
    pub to: String,
}

impl Transition {
    /// The move from `from` to `to`.
    #[must_use]
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

/// The states a node of a type moves through, and the moves.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Lifecycle {
    /// The state a new node is in; one of `states`.
    pub initial: String,
    /// Every state.
    pub states: BTreeSet<String>,
    /// Every declared move.
    #[serde(default)]
    pub transitions: BTreeSet<Transition>,
}

impl Lifecycle {
    /// A lifecycle starting in `initial`.
    #[must_use]
    pub fn new<S: Into<String>>(
        initial: impl Into<String>,
        states: impl IntoIterator<Item = S>,
        transitions: impl IntoIterator<Item = Transition>,
    ) -> Self {
        Self {
            initial: initial.into(),
            states: states.into_iter().map(Into::into).collect(),
            transitions: transitions.into_iter().collect(),
        }
    }
}

/// A named operation of a node type, which `!Invoke` calls by its map key.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationDefinition {
    /// Its name; the SDK files it under the same key.
    pub name: String,
    /// Its arguments, by name, each typed.
    #[serde(default)]
    pub arguments: BTreeMap<String, ValueType>,
    /// Reserved; must be empty for the operation to be invocable.
    #[serde(default)]
    pub preconditions: Vec<String>,
    /// The move it makes, if any.
    #[serde(default)]
    pub transition: Option<Transition>,
    /// Reserved; must be empty for the operation to be invocable.
    #[serde(default)]
    pub emits: Vec<String>,
}

impl OperationDefinition {
    /// An operation with no arguments that moves nothing.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: BTreeMap::new(),
            preconditions: Vec::new(),
            transition: None,
            emits: Vec::new(),
        }
    }

    /// This operation taking one more argument.
    #[must_use]
    pub fn with_argument(mut self, name: impl Into<String>, value_type: ValueType) -> Self {
        self.arguments.insert(name.into(), value_type);
        self
    }

    /// This operation making `transition`.
    #[must_use]
    pub fn with_transition(mut self, transition: Transition) -> Self {
        self.transition = Some(transition);
        self
    }
}

/// A node type, as a seed declares it and `!DefineNodeType` adds it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeType {
    /// The type's id.
    pub id: TypeId,
    /// The name a reader sees; not an identity.
    pub name: String,
    /// The types it specialises.
    #[serde(default)]
    pub parents: BTreeSet<TypeId>,
    /// The properties it declares itself, by id.
    #[serde(default)]
    pub properties: BTreeMap<PropertyId, PropertyDefinition>,
    /// Whether it has no nodes and exists to be a parent.
    #[serde(default)]
    pub abstract_type: bool,
    /// The lifecycle of its nodes, if any.
    #[serde(default)]
    pub lifecycle: Option<Lifecycle>,
    /// Its named operations, by the key `!Invoke` uses.
    #[serde(default)]
    pub operations: BTreeMap<String, OperationDefinition>,
}

impl NodeType {
    /// A concrete type with nothing declared, under a freshly minted id.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: TypeId::mint(),
            name: name.into(),
            parents: BTreeSet::new(),
            properties: BTreeMap::new(),
            abstract_type: false,
            lifecycle: None,
            operations: BTreeMap::new(),
        }
    }

    /// This type specialising one more parent.
    #[must_use]
    pub fn with_parent(mut self, parent: TypeId) -> Self {
        self.parents.insert(parent);
        self
    }

    /// This type declaring `property`, filed under its id.
    #[must_use]
    pub fn with_property(mut self, property: PropertyDefinition) -> Self {
        self.properties.insert(property.id, property);
        self
    }

    /// This type, abstract.
    #[must_use]
    pub fn as_abstract(mut self) -> Self {
        self.abstract_type = true;
        self
    }

    /// This type with `lifecycle`.
    #[must_use]
    pub fn with_lifecycle(mut self, lifecycle: Lifecycle) -> Self {
        self.lifecycle = Some(lifecycle);
        self
    }

    /// This type declaring `operation`, filed under its name.
    #[must_use]
    pub fn with_operation(mut self, operation: OperationDefinition) -> Self {
        self.operations.insert(operation.name.clone(), operation);
        self
    }
}

/// An edge type, as a seed declares it and `!DefineEdgeType` adds it. Its id is also the relation
/// id an assertion's `!Relation` predicate names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeType {
    /// The type's id.
    pub id: TypeId,
    /// The name a reader sees; not an identity.
    pub name: String,
    /// The node types an edge may start at; at least one.
    pub source_types: BTreeSet<TypeId>,
    /// The node types an edge may end at; at least one.
    pub target_types: BTreeSet<TypeId>,
    /// How many edges of this type may leave one node.
    #[serde(default)]
    pub cardinality: Cardinality,
    /// The properties an edge carries, by id.
    #[serde(default)]
    pub properties: BTreeMap<PropertyId, PropertyDefinition>,
    /// The edge type reading this one backwards, if any.
    #[serde(default)]
    pub inverse: Option<TypeId>,
    /// Whether the relation holds both ways. Recorded only.
    #[serde(default)]
    pub symmetric: bool,
    /// Whether the relation composes with itself. Recorded only.
    #[serde(default)]
    pub transitive: bool,
}

impl EdgeType {
    /// A single-cardinality edge type between these ends, under a freshly minted id.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        source_types: impl IntoIterator<Item = TypeId>,
        target_types: impl IntoIterator<Item = TypeId>,
    ) -> Self {
        Self {
            id: TypeId::mint(),
            name: name.into(),
            source_types: source_types.into_iter().collect(),
            target_types: target_types.into_iter().collect(),
            cardinality: Cardinality::One,
            properties: BTreeMap::new(),
            inverse: None,
            symmetric: false,
            transitive: false,
        }
    }

    /// This edge type with `cardinality`.
    #[must_use]
    pub fn with_cardinality(mut self, cardinality: Cardinality) -> Self {
        self.cardinality = cardinality;
        self
    }

    /// This edge type declaring `property`, filed under its id.
    #[must_use]
    pub fn with_property(mut self, property: PropertyDefinition) -> Self {
        self.properties.insert(property.id, property);
        self
    }
}
