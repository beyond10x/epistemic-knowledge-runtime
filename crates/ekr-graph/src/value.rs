//! The values canonical state holds: `architecture-decision-record:0005-float-is-not-canonical`.
//!
//! [`ekr_ontology::Value`] has a `Float` variant and canonical state cannot hold one —
//! `crates/ekr-core/src/canonical.rs` rule 4 admits no float, because `NaN` is not equal to itself
//! and `0.0 == -0.0` holds for two different bit patterns, so no encoding of either is both total
//! and faithful to equality. A quantity that must be hashed is carried as an `Integer` or a
//! `Decimal`.
//!
//! # Unrepresentable, not refused
//!
//! [`CanonicalValue`] is not a wrapper around a checked `Value`: it is the same ten kinds **minus
//! the one that cannot be encoded**. That is what makes [`Canonical::encode`] total — there is no
//! arm for a float to reach, so there is no unreachable arm, no fallible `content_hash` and no
//! error a validator elsewhere is trusted to have made impossible. The alternative, an error that
//! validation makes unreachable, is a claim nobody can check; ADR 0005 rejects it, and this crate
//! has taken the shape twice before — `AGENTS.md` invariant 2 for the canonical/transient membrane
//! and wave p1-04's `TransactionTime` for a belief the runtime never formed.
//!
//! The one way in **from an [`ekr_ontology::Value`]** is
//! [`TryFrom<Value>`](CanonicalValue::try_from), and it is also the way in from a document: the
//! type deserialises *through* the same conversion, so a stored record carrying a float is refused
//! at the boundary a Rust caller is refused at. The variants themselves are public and a caller
//! may write one directly — `CanonicalValue::Decimal("0.1".to_owned())` is an ordinary value — and
//! that is safe for the same reason the conversion is total: the shape is the invariant, and there
//! is no variant a float can inhabit. Serialisation goes back through
//! [`Value`], so what a canonical value writes is what the value it came from wrote — the wire
//! shape is unchanged by this type existing.

use std::collections::BTreeMap;

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::Timestamp;
use ekr_ontology::{Value, ValuePath};
use serde::{Deserialize, Serialize};

use crate::canonical::CanonicalRef;
use crate::node::Node;

/// A value canonical state admits: every kind [`Value`] has except `Float`, at every depth.
///
/// Ten variants against `Value`'s eleven. A [`List`](CanonicalValue::List) holds canonical values
/// and a [`Record`](CanonicalValue::Record) maps to them, so "no float at any depth" is the
/// type's shape rather than a rule a walker enforces.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "Value", try_from = "Value")]
pub enum CanonicalValue {
    /// Text.
    String(String),
    /// A truth value.
    Boolean(bool),
    /// A whole number.
    Integer(i64),
    /// An exact number, as text — what a quantity that must be hashed is carried as.
    Decimal(String),
    /// A point in time.
    Timestamp(Timestamp),
    /// A length of time.
    Duration(i64),
    /// A reference to a node canonical state holds.
    ///
    /// A [`CanonicalRef`] and not a bare id:
    /// `architecture-decision-record:0008-canonical-state-references-are-typed`. Design § 11.3
    /// gives a node reference its own value kind precisely so that it is *checkable*, and a value
    /// inside canonical state reaching a candidate is the crossing AGENTS.md invariant 2 says is
    /// unrepresentable. It writes and reads as the bare id it wraps, so the wire shape — a
    /// [`Value::NodeRef`] — is unchanged.
    NodeRef(CanonicalRef<Node>),
    /// One variant of an enumeration.
    Enum(String),
    /// A sequence of values, each itself admissible.
    List(Vec<CanonicalValue>),
    /// A set of named fields, each itself admissible.
    Record(BTreeMap<String, CanonicalValue>),
}

/// A value canonical state does not admit, and where inside the value it sits.
///
/// Its own type rather than a bare `None`, for the reason
/// [`ConfidenceOutOfRange`](crate::ConfidenceOutOfRange) and
/// [`InvertedRange`](crate::InvertedRange) have one: a value that arrives through serde has to be
/// refused with something a reader can print. The path is what makes it actionable — a caller
/// holding a record of a hundred fields cannot act on "a float is in here somewhere".
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error(
    "{path} is a Float, which canonical state does not admit: carry a quantity that must be \
     hashed as an Integer or a Decimal"
)]
pub struct InadmissibleValue {
    path: ValuePath,
}

impl InadmissibleValue {
    /// Where the refused value sits inside the value that was offered.
    #[must_use]
    pub const fn path(&self) -> &ValuePath {
        &self.path
    }

    /// The value offered was itself the one canonical state refuses.
    fn here() -> Self {
        Self {
            path: ValuePath::root(),
        }
    }

    /// The same refusal, seen from the list one level above it.
    fn inside_element(self, index: usize) -> Self {
        Self {
            path: self.path.inside_element(index),
        }
    }

    /// The same refusal, seen from the record one level above it.
    fn inside_field(self, field: String) -> Self {
        Self {
            path: self.path.inside_field(field),
        }
    }
}

impl TryFrom<Value> for CanonicalValue {
    type Error = InadmissibleValue;

    /// The only way an [`ekr_ontology::Value`] becomes one of these. A caller that already knows
    /// which kind it has writes the variant instead; no variant admits a float, so neither route
    /// needs checking.
    ///
    /// Recursive through itself, so the refusal is raised where the offending value is and gains a
    /// step for each level that declines to hold it. There is no second walk deciding
    /// admissibility before this one converts: one traversal both answers and builds, so the two
    /// cannot disagree. `ekr_ontology::Value::inadmissible_in_canonical_state` states the same
    /// rule over a `Value` for callers above this crate that hold one and are not converting it —
    /// the kernel's type validator is the first — and
    /// `crates/ekr-graph/tests/canonical_value_and_assertion.rs` holds the two answers equal.
    ///
    /// # Errors
    ///
    /// [`InadmissibleValue`], naming the path to the first value canonical state refuses.
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Float(_) => Err(InadmissibleValue::here()),
            Value::String(text) => Ok(Self::String(text)),
            Value::Boolean(flag) => Ok(Self::Boolean(flag)),
            Value::Integer(number) => Ok(Self::Integer(number)),
            Value::Decimal(text) => Ok(Self::Decimal(text)),
            Value::Timestamp(at) => Ok(Self::Timestamp(at)),
            Value::Duration(millis) => Ok(Self::Duration(millis)),
            Value::NodeRef(node) => Ok(Self::NodeRef(CanonicalRef::new(node))),
            Value::Enum(name) => Ok(Self::Enum(name)),
            Value::List(items) => items
                .into_iter()
                .enumerate()
                .map(|(index, item)| {
                    Self::try_from(item).map_err(|refusal| refusal.inside_element(index))
                })
                .collect::<Result<Vec<Self>, Self::Error>>()
                .map(Self::List),
            Value::Record(fields) => fields
                .into_iter()
                .map(|(field, value)| match Self::try_from(value) {
                    Ok(held) => Ok((field, held)),
                    Err(refusal) => Err(refusal.inside_field(field)),
                })
                .collect::<Result<BTreeMap<String, Self>, Self::Error>>()
                .map(Self::Record),
        }
    }
}

impl From<CanonicalValue> for Value {
    /// The way back out, which is total: every canonical value is a value.
    ///
    /// It is what the type serialises through, and what a caller holding the ontology's type
    /// checker — which asks about a `Value` — converts to before asking.
    fn from(value: CanonicalValue) -> Self {
        match value {
            CanonicalValue::String(text) => Self::String(text),
            CanonicalValue::Boolean(flag) => Self::Boolean(flag),
            CanonicalValue::Integer(number) => Self::Integer(number),
            CanonicalValue::Decimal(text) => Self::Decimal(text),
            CanonicalValue::Timestamp(at) => Self::Timestamp(at),
            CanonicalValue::Duration(millis) => Self::Duration(millis),
            CanonicalValue::NodeRef(node) => Self::NodeRef(node.node()),
            CanonicalValue::Enum(name) => Self::Enum(name),
            CanonicalValue::List(items) => Self::List(items.into_iter().map(Self::from).collect()),
            CanonicalValue::Record(fields) => Self::Record(
                fields
                    .into_iter()
                    .map(|(field, value)| (field, Self::from(value)))
                    .collect(),
            ),
        }
    }
}

impl Canonical for CanonicalValue {
    /// The variant marker, then the variant's payload.
    ///
    /// A sum type, so it carries a tag: rule 5 of `ekr_core::canonical`. Without one,
    /// `String("1")` and `Decimal("1")` — and `Integer(1)` and `Duration(1)` — would share an
    /// encoding, and two values a reader distinguishes would share an address.
    ///
    /// The index is a literal and it is the contract: **changing a number moves every content
    /// address that contains that variant**, while reordering the declaration moves nothing.
    /// Nothing derives one from the other, so
    /// `crates/ekr-graph/tests/canonical_value_and_assertion.rs` reads this file and holds the
    /// declaration order equal to the numbering — for this type and for every other sum type the
    /// crate encodes.
    ///
    /// Total, with no arm for a value that cannot be encoded, because no such value inhabits the
    /// type.
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::String(text) => {
                out.variant(0);
                text.encode(out);
            }
            Self::Boolean(flag) => {
                out.variant(1);
                flag.encode(out);
            }
            Self::Integer(number) => {
                out.variant(2);
                number.encode(out);
            }
            Self::Decimal(text) => {
                out.variant(3);
                text.encode(out);
            }
            Self::Timestamp(at) => {
                out.variant(4);
                at.encode(out);
            }
            Self::Duration(millis) => {
                out.variant(5);
                millis.encode(out);
            }
            Self::NodeRef(node) => {
                out.variant(6);
                node.encode(out);
            }
            Self::Enum(name) => {
                out.variant(7);
                name.encode(out);
            }
            Self::List(items) => {
                out.variant(8);
                items.encode(out);
            }
            Self::Record(fields) => {
                out.variant(9);
                fields.encode(out);
            }
        }
    }
}
