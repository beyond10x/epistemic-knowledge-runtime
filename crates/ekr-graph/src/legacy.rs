//! Frozen encodings from source commit 73ab8b0a5aa5c670bacbc4c1abdca02f87b62bf0.
//! These historical data transfer types never construct canonical state capabilities.
//! Field order, discriminants and scalar property shape intentionally remain unchanged.
//! Only ekr-core scalar identities, timestamps, hashes and canonical primitives are shared.
//! Immutable vectors pin those primitives too; no current graph or ontology codec is called.

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Exact source revision whose representation is frozen here.
pub const SOURCE_COMMIT: &str = "73ab8b0a5aa5c670bacbc4c1abdca02f87b62bf0";
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "value_kind", content = "value", deny_unknown_fields)]
/// The original ten hashable value kinds, with node references represented only by identity.
pub enum Value {
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
    /// Original bare node identity, without a claim that its target has been admitted.
    NodeRef(NodeId),
    /// One variant of an enumeration.
    Enum(String),
    /// A sequence of values, each itself admissible.
    List(Vec<Value>),
    /// A set of user-named fields; reserved-looking names remain ordinary keys.
    Record(#[serde(deserialize_with = "unique_map")] BTreeMap<String, Value>),
}
impl Canonical for Value {
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

/// Frozen original-format representation or operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(deserialize = "V: Deserialize<'de>"))]
pub struct Node<V = Value> {
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
    #[serde(deserialize_with = "unique_map")]
    pub properties: BTreeMap<PropertyId, V>,
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
    /// Bounded on `V`, so this exists for `Node<Value>` and not for `Node<Value>`: the
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

/// Frozen original-format representation or operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(bound(deserialize = "V: Deserialize<'de>"))]
pub struct Edge<V = Value> {
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
    /// Its property values, by the property's id — keyed as `Node::properties`(crate::Node) is,
    /// and for the same reason.
    #[serde(deserialize_with = "unique_map")]
    pub properties: BTreeMap<PropertyId, V>,
}

impl<V> Edge<V> {
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

impl<V: Canonical> Canonical for Edge<V> {
    /// The six fields in declaration order, structural and bounded on `V` exactly as
    /// `Node`(crate::Node)'s is.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.root_id.encode(out);
        self.type_id.encode(out);
        self.source.encode(out);
        self.target.encode(out);
        self.properties.encode(out);
    }
}

/// Frozen original-format representation or operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Subject<R = NodeId> {
    /// A node.
    Node(R),
    /// An edge.
    Edge(EdgeId),
    /// A type in the ontology — a schema-level claim shares the provenance model of every other.
    Type(TypeId),
}

impl<R: Canonical> Canonical for Subject<R> {
    /// The variant marker, then the id it names.
    ///
    /// The marker is what separates the three, and is not optional: rule 5 of
    /// `ekr_core::canonical` makes a newtype structural, so a `NodeId`(ekr_core::NodeId), an `EdgeId` and a
    /// `TypeId` over one UUID encode identically. Without the tag, an assertion about a node
    /// and an assertion about the edge that happened to share its bits would share an address.
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::Node(node) => {
                out.variant(0);
                node.encode(out);
            }
            Self::Edge(edge) => {
                out.variant(1);
                edge.encode(out);
            }
            Self::Type(type_id) => {
                out.variant(2);
                type_id.encode(out);
            }
        }
    }
}

/// Frozen original-format representation or operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Predicate {
    /// A declared property of the subject.
    Property(PropertyId),
    /// A relation, named by its edge type.
    Relation(TypeId),
}

impl Canonical for Predicate {
    /// The variant marker, then the id it names — tagged for the reason `Subject` is.
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::Property(property) => {
                out.variant(0);
                property.encode(out);
            }
            Self::Relation(type_id) => {
                out.variant(1);
                type_id.encode(out);
            }
        }
    }
}

/// Frozen original-format representation or operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Object<V = Value, R = NodeId> {
    /// A literal value, typed by the ontology.
    Value(V),
    /// Another node.
    Node(R),
    /// A type in the ontology.
    Type(TypeId),
}

impl<V: Canonical, R: Canonical> Canonical for Object<V, R> {
    /// The variant marker, then the payload — tagged for the reason `Subject` is.
    ///
    /// Bounded on `V`: an object carrying a value canonical state does not admit has no encoding
    /// at all, rather than an encoding that refuses at run time.
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::Value(value) => {
                out.variant(0);
                value.encode(out);
            }
            Self::Node(node) => {
                out.variant(1);
                node.encode(out);
            }
            Self::Type(type_id) => {
                out.variant(2);
                type_id.encode(out);
            }
        }
    }
}

/// Frozen original-format representation or operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "TemporalRangeFields")]
pub struct TemporalRange {
    /// The first instant in the range, or `None` for "as far back as the record goes".
    pub from: Option<Timestamp>,
    /// The first instant *after* the range, or `None` for an open end.
    pub to: Option<Timestamp>,
}

impl TemporalRange {
    /// Unbounded at both ends.
    pub const UNBOUNDED: Self = Self {
        from: None,
        to: None,
    };

    /// A range with the bounds given, or `None` if `to` precedes `from`.
    ///
    /// Refused here rather than deferred, which is the pattern
    /// `Confidence::from_basis_points`(crate::Confidence::from_basis_points) already sets in
    /// this crate: an interval whose end precedes its start contains no instant, so an assertion
    /// carrying one sits in canonical state and is returned by
    /// `GraphSnapshot::valid_at`(crate::GraphSnapshot::valid_at) at no `t` at all — a record
    /// nothing can surface and nothing reports. Design § 20's validator list does not include
    /// temporal ordering and neither does AGENTS.md invariant 7, so deferring it would leave it
    /// owned by nobody.
    ///
    /// `to == from` is **admitted**: `[t, t)` is the empty half-open interval, which is what a
    /// fact recorded and corrected within one millisecond produces. Refusing it would be a claim
    /// about clock resolution that nothing in P1 supports.
    #[must_use]
    pub const fn new(from: Option<Timestamp>, to: Option<Timestamp>) -> Option<Self> {
        if let (Some(from), Some(to)) = (from, to) {
            if to.millis() < from.millis() {
                return None;
            }
        }
        Some(Self { from, to })
    }

    /// A range that begins at `from` and has not ended.
    #[must_use]
    pub const fn since(from: Timestamp) -> Self {
        Self {
            from: Some(from),
            to: None,
        }
    }

    /// Whether `at` falls in the range: `from <= at < to`, with an absent bound unbounded.
    #[must_use]
    pub fn contains(&self, at: Timestamp) -> bool {
        self.from.is_none_or(|from| from <= at) && self.to.is_none_or(|to| at < to)
    }
}

impl Canonical for TemporalRange {
    /// The two bounds, in declaration order, each as an option.
    ///
    /// Structural and untagged: a struct, not a sum type, so rule 5 of `ekr_core::canonical`
    /// leaves its field order to distinguish it. Absence is not emptiness —
    /// `Encoder::option`(ekr_core::canonical::Encoder::option) writes a different tag for
    /// `None` than for any `Some`, so an unbounded range and one bounded at the epoch do not meet.
    fn encode(&self, out: &mut Encoder) {
        out.option(self.from.as_ref());
        out.option(self.to.as_ref());
    }
}

/// Frozen original-format representation or operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{to} precedes {from}: a range whose end precedes its start contains no instant")]
pub struct InvertedRange {
    from: Timestamp,
    to: Timestamp,
}

/// Frozen original-format representation or operation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TemporalRangeFields {
    from: Option<Timestamp>,
    to: Option<Timestamp>,
}

impl TryFrom<TemporalRangeFields> for TemporalRange {
    type Error = InvertedRange;

    fn try_from(fields: TemporalRangeFields) -> Result<Self, Self::Error> {
        Self::new(fields.from, fields.to).ok_or(InvertedRange {
            from: fields.from.unwrap_or(Timestamp::EPOCH),
            to: fields.to.unwrap_or(Timestamp::EPOCH),
        })
    }
}

/// Frozen original-format representation or operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "TransactionTimeFields")]
pub struct TransactionTime {
    /// When the runtime began holding the record. Not optional: `ekr.graph.Assertion.recorded_from`
    /// is declared `Timestamp`, and a belief with no beginning is not a belief.
    pub recorded_from: Timestamp,
    /// When it stopped, or `None` while it still holds it.
    pub recorded_to: Option<Timestamp>,
}

impl TransactionTime {
    /// A belief formed at `recorded_from` and not since withdrawn.
    #[must_use]
    pub const fn since(recorded_from: Timestamp) -> Self {
        Self {
            recorded_from,
            recorded_to: None,
        }
    }

    /// A belief formed at `recorded_from` and withdrawn at `recorded_to`, or `None` if it was
    /// withdrawn before it was formed.
    ///
    /// The same refusal `TemporalRange::new` makes, for the same reason and by the same pattern:
    /// a belief with `recorded_to < recorded_from` was held at no instant, including its own
    /// `recorded_from`. `recorded_to == recorded_from` is admitted — a belief formed and corrected
    /// inside one millisecond is an ordinary thing for a runtime whose commit order is carried by
    /// `RevisionNumber` rather than by a clock.
    #[must_use]
    pub const fn new(recorded_from: Timestamp, recorded_to: Option<Timestamp>) -> Option<Self> {
        if let Some(to) = recorded_to {
            if to.millis() < recorded_from.millis() {
                return None;
            }
        }
        Some(Self {
            recorded_from,
            recorded_to,
        })
    }

    /// Whether the runtime still holds the record — which is what
    /// `Assertion::is_current` means by "has not stopped believing it".
    ///
    /// Only the upper bound is read, and after this type exists that is correct rather than
    /// merely convenient: there is no lower bound to forget.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.recorded_to.is_none()
    }
}

/// Frozen original-format representation or operation.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TransactionTimeFields {
    recorded_from: Timestamp,
    recorded_to: Option<Timestamp>,
}

impl TryFrom<TransactionTimeFields> for TransactionTime {
    type Error = InvertedRange;

    fn try_from(fields: TransactionTimeFields) -> Result<Self, Self::Error> {
        Self::new(fields.recorded_from, fields.recorded_to).ok_or(InvertedRange {
            from: fields.recorded_from,
            to: fields.recorded_to.unwrap_or(fields.recorded_from),
        })
    }
}

impl Canonical for TransactionTime {
    /// The start, then the end as an option — the two fields in declaration order.
    ///
    /// The start is not an option and does not encode as one: `recorded_from` is required, which
    /// is the difference between this type and `TemporalRange`.
    fn encode(&self, out: &mut Encoder) {
        self.recorded_from.encode(out);
        out.option(self.recorded_to.as_ref());
    }
}

/// Frozen original-format representation or operation.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RetractionReason(String);

impl RetractionReason {
    /// The reason as it was stated.
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self(reason.into())
    }

    /// The stated reason.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Canonical for RetractionReason {
    /// The bytes of the text it wraps, with no discriminant: a newtype is structural, which is
    /// rule 5 of `ekr_core::canonical`.
    fn encode(&self, out: &mut Encoder) {
        self.0.encode(out);
    }
}

/// Frozen original-format representation or operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum ValidationState {
    /// An agent has proposed it and nothing has looked at it.
    Proposed,
    /// Validation is under way.
    Validating {
        /// Validators that have finished.
        completed: u32,
        /// Validators the policy requires.
        required: u32,
    },
    /// It crossed the integrity boundary. The agents whose validation it rests on are named:
    /// "an agent said so" is not provenance, but *which* agents said so is.
    Accepted {
        /// The validators that accepted it.
        #[serde(deserialize_with = "unique_set")]
        validators: BTreeSet<AgentId>,
    },
    /// Validation refused it.
    ///
    /// The issues are named by id rather than held inline: `ekr.kernel.ValidationIssue` is the
    /// kernel's record, and the kernel sits above this crate. Design § 17 writes
    /// `Vec<ValidationIssue>`, which would invert the dependency.
    Rejected {
        /// The issues raised against it.
        issues: Vec<IssueId>,
    },
    /// Another assertion contradicts it and neither has won.
    Disputed {
        /// The assertions it is in conflict with.
        competing_assertions: Vec<AssertionId>,
    },
    /// A later assertion replaced it.
    Superseded {
        /// The assertion that replaced it.
        by: AssertionId,
    },
    /// Canonical state no longer treats it as true, without erasing that it once did.
    Retracted {
        /// The revision the retraction was committed at (design § 36).
        at_revision: RevisionNumber,
        /// Why (design § 17).
        reason: RetractionReason,
    },
}

impl ValidationState {
    /// The state's name, as `ekr.graph.ValidationState` declares it.
    ///
    /// The flat name is what the domain, a view and a store column carry; the payload stays in the
    /// variant.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Proposed => "Proposed",
            Self::Validating { .. } => "Validating",
            Self::Accepted { .. } => "Accepted",
            Self::Rejected { .. } => "Rejected",
            Self::Disputed { .. } => "Disputed",
            Self::Superseded { .. } => "Superseded",
            Self::Retracted { .. } => "Retracted",
        }
    }

    /// Whether this state is one canonical knowledge is read from.
    ///
    /// Design § 21: the canonical core is what "has crossed the system's highest integrity
    /// boundary". A `Proposed` or `Disputed` record is in the graph and has not crossed it.
    #[must_use]
    pub const fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted { .. })
    }
}

impl Canonical for ValidationState {
    /// The variant marker, then the variant's fields in declaration order.
    ///
    /// Tagged for the reason `Subject` is, and here the collision is not hypothetical:
    /// `Superseded { by }` and a one-element `Disputed { competing_assertions }` both come down to
    /// one `AssertionId`, and `Proposed` carries nothing at all. The state an assertion is in is
    /// part of what it *is*, so it is part of its address.
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::Proposed => {
                out.variant(0);
            }
            Self::Validating {
                completed,
                required,
            } => {
                out.variant(1);
                completed.encode(out);
                required.encode(out);
            }
            Self::Accepted { validators } => {
                out.variant(2);
                validators.encode(out);
            }
            Self::Rejected { issues } => {
                out.variant(3);
                issues.encode(out);
            }
            Self::Disputed {
                competing_assertions,
            } => {
                out.variant(4);
                competing_assertions.encode(out);
            }
            Self::Superseded { by } => {
                out.variant(5);
                by.encode(out);
            }
            Self::Retracted {
                at_revision,
                reason,
            } => {
                out.variant(6);
                at_revision.encode(out);
                reason.encode(out);
            }
        }
    }
}

/// Frozen original-format representation or operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AssertionStatus {
    /// Not withdrawn and not replaced. Says nothing about whether it was ever accepted.
    Active,
    /// Withdrawn at a revision, for a reason, with the history of having held it intact.
    Retracted {
        /// The revision the retraction was committed at.
        at_revision: RevisionNumber,
        /// Why.
        reason: RetractionReason,
    },
    /// Replaced by a later assertion.
    Superseded {
        /// The assertion that replaced it.
        by: AssertionId,
    },
}

/// Frozen original-format representation or operation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assertion<V = Value> {
    /// The assertion's stable id.
    pub id: AssertionId,
    /// The graph root that owns it.
    pub root_id: GraphRootId,
    /// What it is about.
    pub subject: Subject<NodeId>,
    /// What it says.
    pub predicate: Predicate,
    /// What it says it about.
    pub object: Object<V, NodeId>,
    /// The evidence it rests on. Design § 13; `graph.yaml` gives each link its own
    /// `Support`(crate::Support) entity so that one can be addressed and withdrawn.
    #[serde(deserialize_with = "unique_set")]
    pub evidence: BTreeSet<EvidenceId>,
    /// The agent that proposed it. Agents propose; they never commit.
    pub proposed_by: AgentId,
    /// How far validation got, and how it ended.
    pub validation: ValidationState,
    /// When it was true in the represented world (design § 14.1).
    pub valid_time: TemporalRange,
    /// When the runtime believed it (design § 14.2). Closing `recorded_to` is how a correction
    /// removes a belief without deleting the history of having held it.
    pub transaction_time: TransactionTime,
}

impl<V> Assertion<V> {
    /// Whether canonical state removed it: design § 36.
    ///
    /// `Retracted` and `Superseded` map to themselves; every other validation state maps to
    /// `AssertionStatus::Active`, because § 36's question is about removal and a record that was
    /// never canonical was never removed.
    #[must_use]
    pub fn status(&self) -> AssertionStatus {
        match &self.validation {
            ValidationState::Retracted {
                at_revision,
                reason,
            } => AssertionStatus::Retracted {
                at_revision: *at_revision,
                reason: reason.clone(),
            },
            ValidationState::Superseded { by } => AssertionStatus::Superseded { by: *by },
            _ => AssertionStatus::Active,
        }
    }

    /// Whether this record is the runtime's current, canonical belief.
    ///
    /// Two things:
    ///
    /// * it crossed the integrity boundary — `ValidationState::is_accepted`;
    /// * the runtime has not stopped believing it — `TransactionTime::is_open`.
    ///
    /// **Two and not three.** A `matches!(self.status(), AssertionStatus::Active)` conjunct stood
    /// here and could never change the answer: `status`(Assertion::status) is non-`Active` only
    /// for `Retracted` and `Superseded`, `is_accepted` is true only for `Accepted`, and because
    /// the status is *derived* from the same field those sets are disjoint by construction. It was
    /// a clause no input could reach, which reads as a check and is not one.
    /// `crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs` pins the two conjuncts, so a
    /// third arriving without a case that reaches it turns red.
    ///
    /// It says nothing about *when* the claim holds. That is valid time, and it is the question
    /// `GraphSnapshot::valid_at`(crate::GraphSnapshot::valid_at) asks.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.validation.is_accepted() && self.transaction_time.is_open()
    }
}

impl<V: Canonical> Canonical for Assertion<V> {
    /// The ten fields in declaration order.
    ///
    /// Structural, with no discriminant, for the reason `Root`(crate::Root) has none: an
    /// assertion is not a sum type, and rule 5 of `ekr_core::canonical` says a composite value's
    /// own field structure is what distinguishes it. The order is the contract — moving a field
    /// moves every address that has ever named this assertion, and
    /// `story:commit-and-revision-lineage` writes the first one anybody keeps.
    ///
    /// **Total**, with no fallible path. It is total because it exists only where `V` is
    /// `Canonical`, and the `V` canonical state holds is `Value`, which has no float
    /// in it at any depth: the one thing that could not be encoded cannot be present, so nothing
    /// here has to decide what to do about it.
    /// `crates/ekr-graph/tests/canonical_value_and_assertion.rs` holds every field to reaching
    /// these bytes, by reading the declaration rather than by a list kept beside it.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.root_id.encode(out);
        self.subject.encode(out);
        self.predicate.encode(out);
        self.object.encode(out);
        self.evidence.encode(out);
        self.proposed_by.encode(out);
        self.validation.encode(out);
        self.valid_time.encode(out);
        self.transaction_time.encode(out);
    }
}

/// Frozen original-format representation or operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Space {
    /// Integrated knowledge that has crossed the highest integrity boundary.
    Canonical,
    /// Working state that has not, and may never.
    Transient,
}

/// Frozen original-format representation or operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphRoot {
    /// Its stable id — `root_id` in the domain, which names an identity per entity.
    pub id: GraphRootId,
    /// The space it belongs to.
    pub space: Space,
    /// The schema version its contents are typed by.
    pub schema_version_id: SchemaVersionId,
    /// The root it was derived from, if any.
    pub parent: Option<GraphRootId>,
    /// When it was created.
    pub created_at: Timestamp,
}

/// Frozen original-format representation or operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Root {
    /// Its position in the lineage: zero at the seed, one per commit.
    pub revision: RevisionNumber,
    /// The hash of the root this one follows, or `None` at the seed.
    pub parent: Option<ContentHash>,
    /// The ontology state at this revision.
    pub ontology_root: ContentHash,
    /// The graph state at this revision.
    pub knowledge_root: ContentHash,
    /// The retained evidence at this revision.
    pub evidence_root: ContentHash,
    /// The agent registry at this revision.
    pub agent_root: ContentHash,
    /// The transaction that produced it.
    pub transaction: ContentHash,
}

impl Canonical for Root {
    /// The seven fields in declaration order.
    ///
    /// Structural, with no discriminant: a `Root` is not a sum type, and rule 5 of
    /// `ekr_core::canonical` says a composite value's own field structure is what distinguishes
    /// it. The order is the contract — moving a field moves every revision address ever recorded.
    fn encode(&self, out: &mut Encoder) {
        self.revision.encode(out);
        out.option(self.parent.as_ref());
        self.ontology_root.encode(out);
        self.knowledge_root.encode(out);
        self.evidence_root.encode(out);
        self.agent_root.encode(out);
        self.transaction.encode(out);
    }
}

/// Frozen original-format representation or operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", deny_unknown_fields)]
pub enum RevisionEvent {
    /// The lineage began: `ekr.kernel.Seeded`.
    Seeded {
        /// The seed revision.
        revision_id: RevisionId,
        /// The content address of the seed state.
        seed_hash: ContentHash,
    },
    /// An agent proposed a transaction: `ekr.kernel.TransactionProposed`.
    TransactionProposed {
        /// The transaction.
        transaction_id: TransactionId,
        /// The agent that proposed it. Agents propose; they never commit.
        proposer: AgentId,
        /// The content address of its operations.
        operations_hash: ContentHash,
    },
    /// Validation accepted it: `ekr.kernel.TransactionValidated`.
    TransactionValidated {
        /// The transaction.
        transaction_id: TransactionId,
        /// The revision it was validated against — design § 71, and what makes § 72's stale
        /// commit detectable.
        against: RevisionNumber,
        /// The content address of the validation result.
        validation_hash: ContentHash,
    },
    /// Validation refused it: `ekr.kernel.TransactionRejected`.
    TransactionRejected {
        /// The transaction.
        transaction_id: TransactionId,
        /// How many issues were raised. The issues themselves are the kernel's records.
        issues: u32,
    },
    /// Canonical state moved under it before it committed: `ekr.kernel.TransactionStale`.
    TransactionStale {
        /// The transaction.
        transaction_id: TransactionId,
        /// The revision it was validated against.
        validated_against: RevisionNumber,
        /// The revision canonical state is at now.
        current: RevisionNumber,
    },
    /// It committed, and the lineage advanced: `ekr.kernel.RevisionCommitted`.
    RevisionCommitted {
        /// The transaction.
        transaction_id: TransactionId,
        /// The revision it produced.
        revision_id: RevisionId,
        /// That revision's position in the lineage.
        number: RevisionNumber,
        /// The content address of the graph state at it.
        knowledge_root: ContentHash,
    },
}

impl RevisionEvent {
    /// The number the canonical encoding tags this variant with.
    ///
    /// Hand-maintained, and part of the contract rather than an implementation detail: every
    /// content address containing a revision event contains this number, so changing one
    /// invalidates every address ever recorded for that variant. The declaration order below is
    /// kept equal to it by a case rather than by the compiler — nothing here derives one from the
    /// other.
    #[must_use]
    pub const fn variant_index(&self) -> u32 {
        match self {
            Self::Seeded { .. } => 0,
            Self::TransactionProposed { .. } => 1,
            Self::TransactionValidated { .. } => 2,
            Self::TransactionRejected { .. } => 3,
            Self::TransactionStale { .. } => 4,
            Self::RevisionCommitted { .. } => 5,
        }
    }

    /// The event's name in the ESS domain, fully qualified.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Seeded { .. } => "ekr.kernel.Seeded",
            Self::TransactionProposed { .. } => "ekr.kernel.TransactionProposed",
            Self::TransactionValidated { .. } => "ekr.kernel.TransactionValidated",
            Self::TransactionRejected { .. } => "ekr.kernel.TransactionRejected",
            Self::TransactionStale { .. } => "ekr.kernel.TransactionStale",
            Self::RevisionCommitted { .. } => "ekr.kernel.RevisionCommitted",
        }
    }
}

impl Canonical for RevisionEvent {
    /// The variant marker, then the variant's fields in declaration order.
    ///
    /// The marker is not optional and is not a convenience: `RevisionId` and `TransactionId` are
    /// newtypes over the same UUID shape and therefore encode identically, so two variants whose
    /// field lists happened to line up would share a content address. That the six do not line up
    /// today is an accident of this version of `kernel.yaml`, and rule 5 refuses to rest on it.
    fn encode(&self, out: &mut Encoder) {
        out.variant(self.variant_index());
        match self {
            Self::Seeded {
                revision_id,
                seed_hash,
            } => {
                revision_id.encode(out);
                seed_hash.encode(out);
            }
            Self::TransactionProposed {
                transaction_id,
                proposer,
                operations_hash,
            } => {
                transaction_id.encode(out);
                proposer.encode(out);
                operations_hash.encode(out);
            }
            Self::TransactionValidated {
                transaction_id,
                against,
                validation_hash,
            } => {
                transaction_id.encode(out);
                against.encode(out);
                validation_hash.encode(out);
            }
            Self::TransactionRejected {
                transaction_id,
                issues,
            } => {
                transaction_id.encode(out);
                issues.encode(out);
            }
            Self::TransactionStale {
                transaction_id,
                validated_against,
                current,
            } => {
                transaction_id.encode(out);
                validated_against.encode(out);
                current.encode(out);
            }
            Self::RevisionCommitted {
                transaction_id,
                revision_id,
                number,
                knowledge_root,
            } => {
                transaction_id.encode(out);
                revision_id.encode(out);
                number.encode(out);
                knowledge_root.encode(out);
            }
        }
    }
}

/// Frozen original-format representation or operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum EvidenceKind {
    /// A URL.
    Url,
    /// A document, optionally a section of one.
    Document,
    /// A record in an external database.
    DatabaseRecord,
    /// Another assertion in this graph.
    GraphAssertion,
    /// An observation the runtime captured.
    Observation,
    /// A statement by a person.
    HumanStatement,
}

/// Frozen original-format representation or operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum EvidenceSource {
    /// A URL.
    Url(String),
    /// A document, optionally a section of one.
    Document {
        /// The document's identifier in its own system.
        document_id: String,
        /// The section within it, if the evidence is narrower than the whole.
        section: Option<String>,
    },
    /// A record in an external database.
    DatabaseRecord {
        /// The database.
        database: String,
        /// The table.
        table: String,
        /// The record's key.
        key: String,
    },
    /// Another assertion in this graph.
    GraphAssertion(AssertionId),
    /// An observation the runtime captured.
    Observation(ObservationId),
    /// A statement by a person, identified where policy allows it.
    HumanStatement {
        /// Who said it, where the source's authorisation permits recording that.
        identity: Option<String>,
    },
}

impl Canonical for EvidenceSource {
    /// The variant marker, then the variant's fields in declaration order.
    ///
    /// A sum type, so it carries a tag: rule 5 of `ekr_core::canonical`. The collision it prevents
    /// is in front of us — `GraphAssertion(AssertionId)` and `Observation(ObservationId)` are two
    /// id newtypes, which rule 5 makes encode identically, and `Url(String)` would meet a
    /// `Document` carrying no section.
    ///
    /// The index is a literal and it is the contract: changing a number moves every content
    /// address that contains that variant, and reordering the declaration moves nothing.
    /// `crates/ekr-graph/tests/canonical_value_and_assertion.rs` holds the two in step.
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::Url(url) => {
                out.variant(0);
                url.encode(out);
            }
            Self::Document {
                document_id,
                section,
            } => {
                out.variant(1);
                document_id.encode(out);
                out.option(section.as_ref());
            }
            Self::DatabaseRecord {
                database,
                table,
                key,
            } => {
                out.variant(2);
                database.encode(out);
                table.encode(out);
                key.encode(out);
            }
            Self::GraphAssertion(assertion) => {
                out.variant(3);
                assertion.encode(out);
            }
            Self::Observation(observation) => {
                out.variant(4);
                observation.encode(out);
            }
            Self::HumanStatement { identity } => {
                out.variant(5);
                out.option(identity.as_ref());
            }
        }
    }
}

impl EvidenceSource {
    /// The kind, as `ekr.graph.Evidence.kind` carries it.
    #[must_use]
    pub const fn kind(&self) -> EvidenceKind {
        match self {
            Self::Url(_) => EvidenceKind::Url,
            Self::Document { .. } => EvidenceKind::Document,
            Self::DatabaseRecord { .. } => EvidenceKind::DatabaseRecord,
            Self::GraphAssertion(_) => EvidenceKind::GraphAssertion,
            Self::Observation(_) => EvidenceKind::Observation,
            Self::HumanStatement { .. } => EvidenceKind::HumanStatement,
        }
    }

    /// The locator, as `ekr.graph.Evidence.locator` carries it: enough to find the source again.
    ///
    /// A `HumanStatement` with no recorded identity has an empty locator, which is the honest
    /// answer — the statement is still evidence and there is nothing to point at.
    #[must_use]
    pub fn locator(&self) -> String {
        match self {
            Self::Url(url) => url.clone(),
            Self::Document { document_id, .. } => document_id.clone(),
            Self::DatabaseRecord {
                database,
                table,
                key,
            } => format!("{database}/{table}/{key}"),
            Self::GraphAssertion(id) => id.to_string(),
            Self::Observation(id) => id.to_string(),
            Self::HumanStatement { identity } => identity.clone().unwrap_or_default(),
        }
    }

    /// The section, as `ekr.graph.Evidence.section` carries it. Only a document has one.
    #[must_use]
    pub fn section(&self) -> Option<&str> {
        match self {
            Self::Document { section, .. } => section.as_deref(),
            _ => None,
        }
    }

    /// The observation this evidence was extracted from, as
    /// `ekr.graph.Evidence.observation_id` carries it. Only an `Observation` source has one.
    #[must_use]
    pub const fn observation(&self) -> Option<ObservationId> {
        match self {
            Self::Observation(id) => Some(*id),
            _ => None,
        }
    }
}

/// Frozen original-format representation or operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct Confidence(u16);

impl Confidence {
    /// The upper bound: ten thousand basis points.
    pub const CERTAIN: Self = Self(10_000);

    /// A confidence of `basis_points`, or `None` above ten thousand.
    #[must_use]
    pub const fn from_basis_points(basis_points: u16) -> Option<Self> {
        if basis_points > Self::CERTAIN.0 {
            None
        } else {
            Some(Self(basis_points))
        }
    }

    /// The basis points, as `ekr.graph.Evidence.confidence_bp` carries them.
    #[must_use]
    pub const fn basis_points(self) -> u16 {
        self.0
    }
}

impl Canonical for Confidence {
    /// The basis points it wraps, with no discriminant: a newtype is structural, which is rule 5
    /// of `ekr_core::canonical`. Basis points are exactly why this type is not an `f64` — rule 4
    /// admits no float, and evidence is content-addressed.
    fn encode(&self, out: &mut Encoder) {
        self.0.encode(out);
    }
}

impl From<Confidence> for u16 {
    fn from(confidence: Confidence) -> Self {
        confidence.0
    }
}

impl TryFrom<u16> for Confidence {
    type Error = ConfidenceOutOfRange;

    fn try_from(basis_points: u16) -> Result<Self, Self::Error> {
        Self::from_basis_points(basis_points).ok_or(ConfidenceOutOfRange(basis_points))
    }
}

/// Frozen original-format representation or operation.
#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{0} is not a confidence: expected basis points between 0 and 10000")]
pub struct ConfidenceOutOfRange(u16);

/// Frozen original-format representation or operation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// Its stable id, so that a claim can cite it and a retention policy can find it.
    pub id: EvidenceId,
    /// Where it came from.
    pub source: EvidenceSource,
    /// The content address of what was read, so that "the source changed" is detectable.
    pub content_hash: ContentHash,
    /// The agent that extracted it.
    pub extracted_by: AgentId,
    /// When the source was observed.
    pub observed_at: Timestamp,
    /// How much weight the extractor puts on it.
    pub confidence: Confidence,
}

impl Canonical for Evidence {
    /// The six fields in declaration order, structural and untagged.
    ///
    /// `Root.evidence_root` is "the retained evidence at this revision" (design § 34), and this is
    /// what it is an address over. `content_hash` is the address of what was *read* and is a field
    /// like any other here: it says the source has not changed, not that this record has not.
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.source.encode(out);
        self.content_hash.encode(out);
        self.extracted_by.encode(out);
        self.observed_at.encode(out);
        self.confidence.encode(out);
    }
}

/// Deserializes a historical set without silently discarding repeated members.
///
/// # Errors
/// Duplicate members are refused as `duplicate-member`.
pub fn unique_set<'de, D, T>(deserializer: D) -> Result<BTreeSet<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Ord,
{
    let values = Vec::<T>::deserialize(deserializer)?;
    let mut result = BTreeSet::new();
    for value in values {
        if !result.insert(value) {
            return Err(serde::de::Error::custom("duplicate-member"));
        }
    }
    Ok(result)
}
/// Deserializes a historical map without silently discarding repeated keys.
///
/// # Errors
/// Duplicate keys are refused as `duplicate-key`.
pub fn unique_map<'de, D, K, V>(deserializer: D) -> Result<BTreeMap<K, V>, D::Error>
where
    D: serde::Deserializer<'de>,
    K: Deserialize<'de> + Ord,
    V: Deserialize<'de>,
{
    struct Map<K, V>(std::marker::PhantomData<(K, V)>);
    impl<'de, K: Deserialize<'de> + Ord, V: Deserialize<'de>> serde::de::Visitor<'de> for Map<K, V> {
        type Value = BTreeMap<K, V>;
        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("a map with unique keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut map: A,
        ) -> Result<Self::Value, A::Error> {
            let mut result = BTreeMap::new();
            while let Some((key, value)) = map.next_entry()? {
                if result.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate-key"));
                }
            }
            Ok(result)
        }
    }
    deserializer.deserialize_map(Map(std::marker::PhantomData))
}
