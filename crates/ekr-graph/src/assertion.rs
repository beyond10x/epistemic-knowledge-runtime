//! Assertions: the fundamental primitive (design § 13), their two time dimensions (§ 14), their
//! validation state (§ 17) and their status after retraction (§ 36).

use std::collections::BTreeSet;

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{
    AgentId, AssertionId, EdgeId, EvidenceId, GraphRootId, IssueId, NodeId, PropertyId,
    RevisionNumber, Timestamp, TypeId,
};
use serde::{Deserialize, Serialize};

use crate::value::CanonicalValue;

/// What an assertion is about: `ekr.graph.SubjectKind` plus the identity it names.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Subject {
    /// A node.
    Node(NodeId),
    /// An edge.
    Edge(EdgeId),
    /// A type in the ontology — a schema-level claim shares the provenance model of every other.
    Type(TypeId),
}

impl Canonical for Subject {
    /// The variant marker, then the id it names.
    ///
    /// The marker is what separates the three, and is not optional: rule 5 of
    /// `ekr_core::canonical` makes a newtype structural, so a [`NodeId`], an [`EdgeId`] and a
    /// [`TypeId`] over one UUID encode identically. Without the tag, an assertion about a node
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

/// What is being said about the subject: `ekr.graph.PredicateKind` plus the identity it names.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Predicate {
    /// A declared property of the subject.
    Property(PropertyId),
    /// A relation, named by its edge type.
    Relation(TypeId),
}

impl Canonical for Predicate {
    /// The variant marker, then the id it names — tagged for the reason [`Subject`] is.
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

/// What the predicate relates the subject to: `ekr.graph.ObjectKind` plus its payload.
///
/// Generic over the literal it may carry, defaulting to [`CanonicalValue`]:
/// `architecture-decision-record:0005-float-is-not-canonical` as amended. A canonical claim is
/// content-addressed and `Value::Float` has no encoding, so `Object` on its own admits no float;
/// a candidate claim in a transient root is `Object<ekr_ontology::Value>` and may say something
/// approximate.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Object<V = CanonicalValue> {
    /// A literal value, typed by the ontology.
    Value(V),
    /// Another node.
    Node(NodeId),
    /// A type in the ontology.
    Type(TypeId),
}

impl<V: Canonical> Canonical for Object<V> {
    /// The variant marker, then the payload — tagged for the reason [`Subject`] is.
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

/// A half-open interval of valid time, `[from, to)`, with either end optionally unbounded:
/// `valid_from` and `valid_to` of `ekr.graph.Assertion`, both declared `Optional<Timestamp>`.
///
/// Transaction time is [`TransactionTime`] and not this type: its start is required, and one type
/// with two `Option`s could not say so.
///
/// Design § 14 gives an assertion two ranges and says only that they are ranges. Half-open is
/// the choice this crate makes, and the § 65 example is the reason: Alice's tenure ends on
/// 2026-03-12 and Bob's begins on it, so a closed upper bound would return two chief executives
/// for the one instant the example is named after. Half-open makes consecutive ranges partition
/// the timeline instead of overlapping at their joins.
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
    /// [`Confidence::from_basis_points`](crate::Confidence::from_basis_points) already sets in
    /// this crate: an interval whose end precedes its start contains no instant, so an assertion
    /// carrying one sits in canonical state and is returned by
    /// [`GraphSnapshot::valid_at`](crate::GraphSnapshot::valid_at) at no `t` at all — a record
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
    /// [`Encoder::option`](ekr_core::canonical::Encoder::option) writes a different tag for
    /// `None` than for any `Some`, so an unbounded range and one bounded at the epoch do not meet.
    fn encode(&self, out: &mut Encoder) {
        out.option(self.from.as_ref());
        out.option(self.to.as_ref());
    }
}

/// A range whose end precedes its start.
///
/// Its own type rather than a bare `None`, for the reason
/// [`ConfidenceOutOfRange`](crate::ConfidenceOutOfRange) has one: a value that arrives through
/// serde has to be refused with something a reader can print.
#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("{to} precedes {from}: a range whose end precedes its start contains no instant")]
pub struct InvertedRange {
    from: Timestamp,
    to: Timestamp,
}

/// The wire shape of a [`TemporalRange`], so that serde goes through the same refusal a caller
/// does. Without it, an inverted range is unconstructible in Rust and arrives from a document.
#[derive(Deserialize)]
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

/// When the runtime believed an assertion: `recorded_from` and `recorded_to` of
/// `ekr.graph.Assertion`, design § 14.2.
///
/// Its own type rather than a second [`TemporalRange`], because the two dimensions are not the
/// same shape. `graph.yaml:256-259` declares `recorded_from` as a required `Timestamp` and
/// `recorded_to` as `Optional<Timestamp>`: **a record the runtime holds always has a moment it
/// began holding it.** Valid time has no such floor — a fact may have been true for as long as
/// anyone knows — which is why that one keeps two `Option`s.
///
/// Carrying transaction time in `TemporalRange` made a record with no start representable, and
/// [`Assertion::is_current`] then reported as the runtime's current belief a record it had no
/// record of ever forming. With the start required, that state does not exist to be filtered.
///
/// Half-open at the upper end, `[recorded_from, recorded_to)`, exactly as valid time is.
///
/// There is no `held_at`. The transaction-time axis of a bitemporal read is not built in P1 —
/// [`GraphSnapshot::valid_at`](crate::GraphSnapshot::valid_at) is the only read, and it asks about
/// valid time — so a method answering it would be public surface with no caller, kept alive by the
/// cases written about it. It arrives with design § 47's `QueryScope` in P4, together with the
/// read that needs it.
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
    /// The same refusal [`TemporalRange::new`] makes, for the same reason and by the same pattern:
    /// a belief with `recorded_to < recorded_from` was held at no instant, including its own
    /// `recorded_from`. `recorded_to == recorded_from` is admitted — a belief formed and corrected
    /// inside one millisecond is an ordinary thing for a runtime whose commit order is carried by
    /// [`RevisionNumber`] rather than by a clock.
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
    /// [`Assertion::is_current`] means by "has not stopped believing it".
    ///
    /// Only the upper bound is read, and after this type exists that is correct rather than
    /// merely convenient: there is no lower bound to forget.
    #[must_use]
    pub const fn is_open(&self) -> bool {
        self.recorded_to.is_none()
    }
}

/// The wire shape of a [`TransactionTime`], so that serde goes through the same refusal.
#[derive(Deserialize)]
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
    /// is the difference between this type and [`TemporalRange`].
    fn encode(&self, out: &mut Encoder) {
        self.recorded_from.encode(out);
        out.option(self.recorded_to.as_ref());
    }
}

/// Why a canonical assertion was retracted.
///
/// Design § 36 names the type and gives it no variants, and nothing else in the design or in
/// `systems/ekr/domains/graph.yaml` enumerates them — the domain has no field for a retraction
/// reason at all. So this is the stated reason as text rather than a taxonomy invented here; the
/// vocabulary arrives with the retraction command, which is P3.
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

/// How far validation got, and how it ended: design § 17 and § 36, projected from
/// `ekr.graph.ValidationState`.
///
/// # The domain carries the state names and one payload
///
/// `graph.yaml` declares this as a flat enumeration of the seven names, and says of the payloads:
///
/// > Only one state's payload has a carrier beside it: a Superseded names the superseding
/// > assertion in Assertion.superseded_by.
///
/// The crate holds every payload where design § 17 puts it — in the variant, where a `Rejected`
/// without reasons is unrepresentable rather than merely unusual — so six of the seven have no
/// projection into the domain at all. The gap is
/// `task:graph-domain-carries-validation-state-payloads`;
/// `crates/ekr-graph/tests/domain_projection.rs` reads the sentence above out of the document at
/// run time and holds this quote to it, so the two cannot drift apart again.
///
/// `Retracted` merges § 17's `reason` with § 36's `at_revision`: the two sections describe one
/// event, and a retraction that does not say which revision it happened at cannot be replayed.
/// It also replaces the `Accepted { validators }` it retracts, so the validator set a retraction
/// acts on does not survive it — design § 36 asks for the history to remain, § 17 gives one enum,
/// and reconciling them is a design decision, filed as
/// `task:assertion-retraction-erases-its-acceptance` and blocking
/// `story:commit-and-revision-lineage`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
    /// Tagged for the reason [`Subject`] is, and here the collision is not hypothetical:
    /// `Superseded { by }` and a one-element `Disputed { competing_assertions }` both come down to
    /// one [`AssertionId`], and `Proposed` carries nothing at all. The state an assertion is in is
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

/// Whether canonical state *removed* an assertion: design § 36.
///
/// A narrower question than it looks, and a different one from [`ValidationState`]. § 36 is about
/// removal — retraction and supersession — so it has nothing to say about a record that never
/// entered canonical state. A `Rejected` or `Proposed` assertion is therefore `Active` here, which
/// does not mean it is knowledge: [`Assertion::is_current`] excludes it by its validation state,
/// which is the field that answers whether it ever crossed the boundary.
///
/// Derived from [`ValidationState`] by [`Assertion::status`] rather than stored beside it. Design
/// § 17 gives one enum and § 36 gives another, and two fields that can disagree about the same
/// record is a defect waiting to be written.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

/// One claim, with its evidence and its two time dimensions: design § 13–14.
///
/// No constructor: every field is required and several are unordered collections, so a caller
/// builds it as a literal. There is no writer in this crate — mutation of canonical state arrives
/// as a transaction the kernel commits.
///
/// Generic over the value its object may carry, defaulting to [`CanonicalValue`], with the same
/// consequence [`Node`](crate::Node) carries: `Assertion<CanonicalValue>` has a content address
/// and `Assertion<ekr_ontology::Value>` — what a transient root holds — does not.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Assertion<V = CanonicalValue> {
    /// The assertion's stable id.
    pub id: AssertionId,
    /// The graph root that owns it.
    pub root_id: GraphRootId,
    /// What it is about.
    pub subject: Subject,
    /// What it says.
    pub predicate: Predicate,
    /// What it says it about.
    pub object: Object<V>,
    /// The evidence it rests on. Design § 13; `graph.yaml` gives each link its own
    /// [`Support`](crate::Support) entity so that one can be addressed and withdrawn.
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
    /// [`AssertionStatus::Active`], because § 36's question is about removal and a record that was
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
    /// * it crossed the integrity boundary — [`ValidationState::is_accepted`];
    /// * the runtime has not stopped believing it — [`TransactionTime::is_open`].
    ///
    /// **Two and not three.** A `matches!(self.status(), AssertionStatus::Active)` conjunct stood
    /// here and could never change the answer: [`status`](Assertion::status) is non-`Active` only
    /// for `Retracted` and `Superseded`, `is_accepted` is true only for `Accepted`, and because
    /// the status is *derived* from the same field those sets are disjoint by construction. It was
    /// a clause no input could reach, which reads as a check and is not one.
    /// `crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs` pins the two conjuncts, so a
    /// third arriving without a case that reaches it turns red.
    ///
    /// It says nothing about *when* the claim holds. That is valid time, and it is the question
    /// [`GraphSnapshot::valid_at`](crate::GraphSnapshot::valid_at) asks.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.validation.is_accepted() && self.transaction_time.is_open()
    }
}

impl<V: Canonical> Canonical for Assertion<V> {
    /// The ten fields in declaration order.
    ///
    /// Structural, with no discriminant, for the reason [`Root`](crate::Root) has none: an
    /// assertion is not a sum type, and rule 5 of `ekr_core::canonical` says a composite value's
    /// own field structure is what distinguishes it. The order is the contract — moving a field
    /// moves every address that has ever named this assertion, and
    /// `story:commit-and-revision-lineage` writes the first one anybody keeps.
    ///
    /// **Total**, with no fallible path. It is total because it exists only where `V` is
    /// [`Canonical`], and the `V` canonical state holds is [`CanonicalValue`], which has no float
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
