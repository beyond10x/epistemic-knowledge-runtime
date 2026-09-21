//! Typed values: design § 11.3, where a runtime value mirrors its declared type.
//!
//! `employer = "OpenAI"` is semantically weaker than `employer = NodeRef(organization/openai)`,
//! and the difference is only checkable if a value carries its shape rather than defaulting to
//! arbitrary JSON. So [`ValueType`] and [`Value`] are two enumerations of the same eleven kinds,
//! and [`ValueKind`] — `ekr.ontology.ValueKind` of `systems/ekr/domains/ontology.yaml` — is the
//! discriminant both answer with.
//!
//! [`Cardinality`] is beside the type, not inside it: design § 11.2 gives a property definition a
//! `value_type` *and* a `cardinality`, so how many values a property carries is a different
//! question from what each value is. `ValueType::List` is the other thing — one value that is a
//! sequence.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use ekr_core::{NodeId, Timestamp, TypeId};
use serde::{Deserialize, Serialize};

/// How many values a property may carry: `ekr.ontology.Cardinality` of
/// `systems/ekr/domains/ontology.yaml`.
///
/// Design § 11.2 names `Cardinality` and § 12 names `EdgeCardinality`, and defines neither; the
/// ESS domain carries an `UNMAPPED:` marker saying so and serves both with this enumeration until
/// the design says otherwise.
#[derive(
    Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Cardinality {
    /// At most one value.
    #[default]
    One,
    /// Any number of values.
    Many,
}

impl Cardinality {
    /// Whether this cardinality permits `count` values.
    ///
    /// Presence is a separate question: `PropertyDefinition::required` is what makes zero values
    /// a refusal, so `One` permits zero here and an optional property with no value is well
    /// typed.
    #[must_use]
    pub const fn permits(self, count: usize) -> bool {
        match self {
            Self::One => count <= 1,
            Self::Many => true,
        }
    }
}

impl fmt::Display for Cardinality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::One => f.write_str("One"),
            Self::Many => f.write_str("Many"),
        }
    }
}

/// The shape a value has, without its parameters: `ekr.ontology.ValueKind` of
/// `systems/ekr/domains/ontology.yaml`.
///
/// It is what a [`ValueType`] and a [`Value`] are compared by, and what a refusal names.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ValueKind {
    /// Text.
    String,
    /// A truth value.
    Boolean,
    /// A whole number.
    Integer,
    /// An approximate number, which is never hashed — `ekr_core::canonical` rule 4.
    Float,
    /// An exact number held as text, which is.
    Decimal,
    /// A point in time.
    Timestamp,
    /// A length of time.
    Duration,
    /// A reference to a node, constrained to declared types.
    NodeRef,
    /// One of a declared set of variants.
    Enum,
    /// A sequence of values of one element type.
    List,
    /// A set of named fields, each with its own type.
    Record,
}

impl fmt::Display for ValueKind {
    /// The variant's own name, as `systems/ekr/domains/ontology.yaml` spells it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::String => "String",
            Self::Boolean => "Boolean",
            Self::Integer => "Integer",
            Self::Float => "Float",
            Self::Decimal => "Decimal",
            Self::Timestamp => "Timestamp",
            Self::Duration => "Duration",
            Self::NodeRef => "NodeRef",
            Self::Enum => "Enum",
            Self::List => "List",
            Self::Record => "Record",
        };
        f.write_str(name)
    }
}

/// A declared type: design § 11.3.
///
/// Serialised with `value_kind` as the discriminant and `parameters` as the content: a schema
/// document reads `value_kind: NodeRef` with `parameters: { allowed_types: [...] }`, and a scalar
/// kind carries no parameters at all. **That shape is this crate's own.**
///
/// `systems/ekr/domains/ontology.yaml` carries a *flattened projection* of it and declares no
/// `parameters` key: its `ekr.ontology.PropertyDefinition` has `value_kind` beside
/// `ref_allowed_types` and `enum_variants`, with no recursive `value_type` field at all. The
/// flattening is a limit of `ess/1`, whose types are not recursive, so a `ValueType` containing
/// another `ValueType` cannot be expressed there; `graph.yaml` flattens `TypedValue` the same way
/// for the same reason.
///
/// **The projection is partial, and covers two of the four compound kinds.** `ref_allowed_types`
/// carries a `NodeRef`'s parameters and `enum_variants` carries an `Enum`'s. There is no field for
/// a `List`'s element type and none for a `Record`'s fields, so a property whose declared type is
/// a `List` or a `Record` is constructible here, is accepted by
/// [`Ontology::load`](crate::schema::Ontology::load), and has no representation in the domain
/// today — which is a
/// gap a store persisting a `PropertyDefinition` will meet.
/// `task:ess-domain-carries-compound-value-types` carries it.
///
/// So this is a boundary that is doing its job for `NodeRef` and `Enum` and is incomplete for
/// `List` and `Record` — not drift to be reconciled by making the two shapes identical.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "value_kind", content = "parameters")]
pub enum ValueType {
    /// Text.
    String,
    /// A truth value.
    Boolean,
    /// A whole number.
    Integer,
    /// An approximate number.
    Float,
    /// An exact number, held as text so that it has one representation.
    Decimal,
    /// A point in time.
    Timestamp,
    /// A length of time.
    Duration,
    /// A reference to a node of one of the allowed types, or of a type that conforms to one.
    NodeRef {
        /// The types a value of this type may point at. Empty is refused at load: a reference to
        /// nothing is not a reference.
        allowed_types: BTreeSet<TypeId>,
    },
    /// One of a declared set of variants.
    Enum {
        /// The variants a value of this type may be. Empty is refused at load: a type no value
        /// inhabits is not a type.
        variants: BTreeSet<String>,
    },
    /// A sequence whose every element has this type.
    List(Box<ValueType>),
    /// A set of named fields, each with its own type. Exactly these fields, no more and no fewer.
    Record(BTreeMap<String, ValueType>),
}

impl ValueType {
    /// The kind this type declares.
    #[must_use]
    pub const fn kind(&self) -> ValueKind {
        match self {
            Self::String => ValueKind::String,
            Self::Boolean => ValueKind::Boolean,
            Self::Integer => ValueKind::Integer,
            Self::Float => ValueKind::Float,
            Self::Decimal => ValueKind::Decimal,
            Self::Timestamp => ValueKind::Timestamp,
            Self::Duration => ValueKind::Duration,
            Self::NodeRef { .. } => ValueKind::NodeRef,
            Self::Enum { .. } => ValueKind::Enum,
            Self::List(_) => ValueKind::List,
            Self::Record(_) => ValueKind::Record,
        }
    }
}

/// A runtime value, mirroring its declared [`ValueType`]: design § 11.3.
///
/// Serialised the same way its type is, with `value_kind` as the discriminant, so that a value and
/// the type it claims to satisfy are read the same way by anything that reads both.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "value_kind", content = "value")]
pub enum Value {
    /// Text.
    String(String),
    /// A truth value.
    Boolean(bool),
    /// A whole number.
    Integer(i64),
    /// An approximate number.
    Float(f64),
    /// An exact number, as text.
    Decimal(String),
    /// A point in time, as the milliseconds-since-epoch newtype ADR 0004 settled.
    Timestamp(Timestamp),
    /// A length of time.
    Duration(i64),
    /// A reference to a node.
    NodeRef(NodeId),
    /// One variant of an enumeration.
    Enum(String),
    /// A sequence of values.
    List(Vec<Value>),
    /// A set of named fields.
    Record(BTreeMap<String, Value>),
}

impl Value {
    /// The kind this value has, which is what its declared type must also have.
    #[must_use]
    pub const fn kind(&self) -> ValueKind {
        match self {
            Self::String(_) => ValueKind::String,
            Self::Boolean(_) => ValueKind::Boolean,
            Self::Integer(_) => ValueKind::Integer,
            Self::Float(_) => ValueKind::Float,
            Self::Decimal(_) => ValueKind::Decimal,
            Self::Timestamp(_) => ValueKind::Timestamp,
            Self::Duration(_) => ValueKind::Duration,
            Self::NodeRef(_) => ValueKind::NodeRef,
            Self::Enum(_) => ValueKind::Enum,
            Self::List(_) => ValueKind::List,
            Self::Record(_) => ValueKind::Record,
        }
    }
}
