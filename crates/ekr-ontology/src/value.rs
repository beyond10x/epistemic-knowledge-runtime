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

/// Where a value sits inside the value that contains it: what a refusal names.
///
/// Written as the value itself and one step per level below it — `value`, `value[2]`,
/// `value.samples[0].mean`. A field name that is not a bare identifier is quoted, so
/// `value."mean reading"` is not read as two steps.
///
/// Its own type rather than a `String` because it is built from the bottom up: a refusal is raised
/// where the offending value is and gains a step as each level above it declines to hold it, which
/// is what [`inside_element`](ValuePath::inside_element) and
/// [`inside_field`](ValuePath::inside_field) do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValuePath(Vec<Step>);

/// One step down into a compound value.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Step {
    /// Into a [`Value::List`], at this position.
    Element(usize),
    /// Into a [`Value::Record`], at this field.
    Field(String),
}

impl ValuePath {
    /// The value itself, with nothing above it.
    #[must_use]
    pub const fn root() -> Self {
        Self(Vec::new())
    }

    /// This path, seen from the list one level above it: `value` becomes `value[3]`.
    #[must_use]
    pub fn inside_element(mut self, index: usize) -> Self {
        self.0.insert(0, Step::Element(index));
        self
    }

    /// This path, seen from the record one level above it: `value` becomes `value.reading`.
    #[must_use]
    pub fn inside_field(mut self, field: impl Into<String>) -> Self {
        self.0.insert(0, Step::Field(field.into()));
        self
    }
}

impl fmt::Display for ValuePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("value")?;
        for step in &self.0 {
            match step {
                Step::Element(index) => write!(f, "[{index}]")?,
                Step::Field(field) if is_bare_identifier(field) => write!(f, ".{field}")?,
                Step::Field(field) => write!(f, ".{field:?}")?,
            }
        }
        Ok(())
    }
}

/// Whether a field name can be written after a dot without being mistaken for two steps.
fn is_bare_identifier(field: &str) -> bool {
    !field.is_empty()
        && !field.starts_with(|c: char| c.is_ascii_digit())
        && field
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
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

    /// Where, if anywhere, this value holds something canonical state does not admit —
    /// `architecture-decision-record:0005-float-is-not-canonical`.
    ///
    /// `ekr_core::canonical` rule 4 admits no float: `NaN` is not equal to itself, `0.0 == -0.0`
    /// holds for two different bit patterns, and no encoding of either is both total and faithful
    /// to equality. A quantity that must be hashed is carried as an [`Integer`](Value::Integer) or
    /// a [`Decimal`](Value::Decimal), which is what [`ValueKind`] distinguishes the two for.
    ///
    /// **Recursive, and that is the whole point.** A [`List`](Value::List) or a
    /// [`Record`](Value::Record) is admissible exactly when everything inside it is, at any depth,
    /// so a float one level down does not pass because the value around it is a record. The answer
    /// is the [`ValuePath`] of the first such value in the value's *own* order — position in a
    /// list, key order in a record — rather than a bare "no", because a caller holding a hundred
    /// fields cannot act on one.
    ///
    /// `Float` stays legal here and in the transient graph. Nothing there is content-addressed,
    /// and an approximate measurement is a reasonable thing to hold before it becomes canonical;
    /// what this answers is whether it may cross into state that is.
    ///
    /// # It has no caller yet, deliberately
    ///
    /// `ekr_graph::CanonicalValue` does **not** call it: that conversion refuses structurally as it
    /// converts, and consulting a predicate first would be a second walk that can disagree with the
    /// first (`crates/ekr-graph/src/value.rs` says so where it converts). The caller that lands is
    /// the kernel's type validator, `story:transaction-and-validators`, which holds a `Value` it is
    /// not converting and must refuse an operation carrying an inadmissible one.
    ///
    /// It is here before that caller because this crate owns [`Value`] and should own the statement
    /// of what canonical state admits of one — and because
    /// `crates/ekr-graph/tests/canonical_value_and_assertion.rs` holds this answer equal to the
    /// conversion's, which is what keeps two independent walks of the same rule from drifting. That
    /// case is the justification; without it this would be a public item with nothing to say
    /// whether it is right.
    #[must_use]
    pub fn inadmissible_in_canonical_state(&self) -> Option<ValuePath> {
        match self {
            Self::Float(_) => Some(ValuePath::root()),
            Self::List(items) => items.iter().enumerate().find_map(|(index, item)| {
                item.inadmissible_in_canonical_state()
                    .map(|path| path.inside_element(index))
            }),
            Self::Record(fields) => fields.iter().find_map(|(field, value)| {
                value
                    .inadmissible_in_canonical_state()
                    .map(|path| path.inside_field(field.clone()))
            }),
            Self::String(_)
            | Self::Boolean(_)
            | Self::Integer(_)
            | Self::Decimal(_)
            | Self::Timestamp(_)
            | Self::Duration(_)
            | Self::NodeRef(_)
            | Self::Enum(_) => None,
        }
    }
}
