//! Assertions: the fundamental primitive (design § 13), their two time dimensions (§ 14), their
//! validation state (§ 17) and their status after retraction (§ 36).

use std::collections::BTreeSet;

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{
    AgentId, AssertionId, EdgeId, EvidenceId, GraphRootId, IssueId, PropertyId, RevisionNumber,
    Timestamp, TypeId,
};
use serde::{Deserialize, Serialize};

use crate::canonical::{CanonicalRef, ValueSpace};
use crate::node::Node;
use crate::value::CanonicalValue;

/// What an assertion is about: `ekr.graph.SubjectKind` plus the identity it names.
///
/// Generic over the reference its node arm carries, defaulting to
/// [`CanonicalRef<Node>`](crate::CanonicalRef):
/// `architecture-decision-record:0008-canonical-state-references-are-typed`. The parameter is the
/// *reference* and not the value, because that is all this type holds — an
/// [`Assertion`] instantiates it as `Subject<V::NodeRef>` for the space its value belongs to, and
/// a canonical claim about a candidate is therefore not a value of this type.
///
/// **It carries no value, so it cannot carry [`Object`]'s defect**, which was a reference default
/// disagreeing with a value parameter beside it. There is nothing here for a default to disagree
/// with: `Subject` on its own is canonical state's, exactly as [`Node`] and [`Edge`](crate::Edge)
/// on their own are, and a transient root's is `Subject<NodeId>`. Rust has no
/// way to give this type `Object`'s shape either — a `V` it never held would be a parameter that is
/// never used, which does not compile.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Subject<R = CanonicalRef<Node>> {
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
    /// `ekr_core::canonical` makes a newtype structural, so a [`NodeId`](ekr_core::NodeId), an [`EdgeId`] and a
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
/// # Its node arm carries a reference, and the default is the one its value's space holds
///
/// `architecture-decision-record:0008-canonical-state-references-are-typed`. The second parameter
/// defaults **through [`ValueSpace`]** rather than to [`CanonicalRef<Node>`] outright, and that is
/// not a stylistic choice: `R` and `V` are the only two parameters in this crate that can disagree
/// about which side of the membrane a value is on, and a default naming the canonical reference
/// makes `Object<ekr_ontology::Value>` — the spelling this type's own documentation gives for a
/// claim in a transient root — a transient claim demanding a canonical reference. Measured by the
/// adversary of wave p1-06. Tying the default to the trait that binds a value to its reference
/// makes the pair unable to disagree unless a caller writes both out.
///
/// [`Assertion`] writes `Object<V, V::NodeRef>` explicitly and was never affected, which is why
/// nothing else caught it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Object<V: ValueSpace = CanonicalValue, R = <V as ValueSpace>::NodeRef> {
    /// A literal value, typed by the ontology.
    Value(V),
    /// Another node.
    Node(R),
    /// A type in the ontology.
    Type(TypeId),
}

impl<V: ValueSpace + Canonical, R: Canonical> Canonical for Object<V, R> {
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

/// The independent assessment of a claim (amendment 88).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Assessment {
    /// No verdict has been attributed yet.
    Proposed,
    /// Deterministic checks are in progress.
    Validating {
        /// Finished checks.
        completed: u32,
        /// Required checks.
        required: u32,
    },
    /// Accepted by these actual validators.
    Accepted {
        /// The validating agents, without duplicates.
        #[serde(deserialize_with = "ekr_core::decode::unique_set")]
        validators: BTreeSet<AgentId>,
    },
    /// Refused, retaining the ordered issue identities.
    Rejected {
        /// Issues raised by validation.
        issues: Vec<IssueId>,
    },
    /// Competing assertions remain unresolved.
    Disputed {
        /// Ordered competing identities.
        competing_assertions: Vec<AssertionId>,
    },
}
impl Assessment {
    /// The declared assessment kind.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Proposed => "Proposed",
            Self::Validating { .. } => "Validating",
            Self::Accepted { .. } => "Accepted",
            Self::Rejected { .. } => "Rejected",
            Self::Disputed { .. } => "Disputed",
        }
    }
    /// Whether validation accepted this claim, independently of its lifecycle.
    #[must_use]
    pub const fn is_accepted(&self) -> bool {
        matches!(self, Self::Accepted { .. })
    }
}
impl Canonical for Assessment {
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::Proposed => out.variant(0),
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
        }
    }
}

/// Stored withdrawal state. Neither withdrawal nor replacement erases assessment or provenance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum AssertionLifecycle {
    /// Still active in the selected revision.
    Active,
    /// Withdrawn from every valid time beginning with this committed revision.
    Retracted {
        /// First revision containing the withdrawal.
        at_revision: RevisionNumber,
        /// Recorded explanation.
        reason: RetractionReason,
    },
    /// Replaced at a valid-time boundary; the earlier interval remains queryable.
    Superseded {
        /// The accepted replacing assertion.
        by: AssertionId,
        /// First revision containing the replacement.
        at_revision: RevisionNumber,
        /// The replacement's valid-time start.
        effective_from: Timestamp,
    },
}
impl AssertionLifecycle {
    /// The declared lifecycle kind.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Active => "Active",
            Self::Retracted { .. } => "Retracted",
            Self::Superseded { .. } => "Superseded",
        }
    }
}
impl Canonical for AssertionLifecycle {
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::Active => out.variant(0),
            Self::Retracted {
                at_revision,
                reason,
            } => {
                out.variant(1);
                at_revision.encode(out);
                reason.encode(out);
            }
            Self::Superseded {
                by,
                at_revision,
                effective_from,
            } => {
                out.variant(2);
                by.encode(out);
                at_revision.encode(out);
                effective_from.encode(out);
            }
        }
    }
}

/// One claim with independent assessment, lifecycle, provenance and both time dimensions.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assertion<V: ValueSpace = CanonicalValue> {
    /// Stable assertion identity.
    pub id: AssertionId,
    /// Owning graph root.
    pub root_id: GraphRootId,
    /// What the claim describes.
    pub subject: Subject<V::NodeRef>,
    /// The declared relation or property.
    pub predicate: Predicate,
    /// The claimed object.
    pub object: Object<V, V::NodeRef>,
    /// Retained supporting evidence.
    #[serde(deserialize_with = "ekr_core::decode::unique_set")]
    pub evidence: BTreeSet<EvidenceId>,
    /// Authenticated proposer.
    pub proposed_by: AgentId,
    /// Verdict, retained through subsequent withdrawal.
    pub assessment: Assessment,
    /// Independent stored withdrawal state.
    pub lifecycle: AssertionLifecycle,
    /// The half-open interval in the represented world.
    pub valid_time: TemporalRange,
    /// The recorded belief interval.
    pub transaction_time: TransactionTime,
}
impl<V: ValueSpace> Assertion<V> {
    /// The stored lifecycle, independent of assessment.
    #[must_use]
    pub fn status(&self) -> AssertionLifecycle {
        self.lifecycle.clone()
    }
    /// Accepted, active and not closed in recorded time.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.assessment.is_accepted()
            && matches!(self.lifecycle, AssertionLifecycle::Active)
            && self.transaction_time.is_open()
    }
    /// Belief visible at valid time in the selected immutable revision.
    #[must_use]
    pub fn valid_at(&self, at: Timestamp) -> bool {
        self.assessment.is_accepted()
            && self.valid_time.contains(at)
            && (matches!(self.lifecycle, AssertionLifecycle::Superseded { .. })
                || matches!(self.lifecycle, AssertionLifecycle::Active)
                    && self.transaction_time.is_open())
    }
}
impl<V: ValueSpace + Canonical> Canonical for Assertion<V> {
    fn encode(&self, out: &mut Encoder) {
        self.id.encode(out);
        self.root_id.encode(out);
        self.subject.encode(out);
        self.predicate.encode(out);
        self.object.encode(out);
        self.evidence.encode(out);
        self.proposed_by.encode(out);
        self.assessment.encode(out);
        self.lifecycle.encode(out);
        self.valid_time.encode(out);
        self.transaction_time.encode(out);
    }
}
