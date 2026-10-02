//! The graph records a seed carries and a transaction proposes: nodes, edges, assertions and
//! evidence (`docs/cli.md`, "The graph section", "Assertions", "Evidence and
//! `evidence_payloads`").

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    Timestamp, TypeId,
};
use serde::{Deserialize, Serialize};

use super::value::Value;

/// A node a transaction's `!CreateNode` creates, or a seed's node before its lifecycle state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeDraft {
    /// Its id.
    pub id: NodeId,
    /// The graph root it belongs to.
    pub root_id: GraphRootId,
    /// A concrete node type.
    pub type_id: TypeId,
    /// The name a reader sees; a property, not an identity.
    pub canonical_name: String,
    /// Its values, by property id; each list non-empty.
    pub properties: BTreeMap<PropertyId, Vec<Value>>,
    /// The names a typed reference is matched against; written only when there is one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

impl NodeDraft {
    /// A node of `type_id` with no values, under a freshly minted id.
    #[must_use]
    pub fn new(root_id: GraphRootId, type_id: TypeId, canonical_name: impl Into<String>) -> Self {
        Self {
            id: NodeId::mint(),
            root_id,
            type_id,
            canonical_name: canonical_name.into(),
            properties: BTreeMap::new(),
            aliases: Vec::new(),
        }
    }

    /// This node under `id` rather than a minted one.
    #[must_use]
    pub fn with_id(mut self, id: NodeId) -> Self {
        self.id = id;
        self
    }

    /// This node carrying one more value of `property`.
    #[must_use]
    pub fn with_property(mut self, property: PropertyId, value: Value) -> Self {
        self.properties.entry(property).or_default().push(value);
        self
    }

    /// This node known by one more alias.
    #[must_use]
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }
}

/// An edge a transaction's `!CreateEdge` creates, or a seed's edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EdgeDraft {
    /// Its id.
    pub id: EdgeId,
    /// The graph root it belongs to.
    pub root_id: GraphRootId,
    /// Its edge type.
    pub type_id: TypeId,
    /// The node it starts at.
    pub source: NodeId,
    /// The node it ends at.
    pub target: NodeId,
    /// Its values, by property id.
    pub properties: BTreeMap<PropertyId, Vec<Value>>,
}

impl EdgeDraft {
    /// An edge of `type_id` from `source` to `target`, under a freshly minted id.
    #[must_use]
    pub fn new(root_id: GraphRootId, type_id: TypeId, source: NodeId, target: NodeId) -> Self {
        Self {
            id: EdgeId::mint(),
            root_id,
            type_id,
            source,
            target,
            properties: BTreeMap::new(),
        }
    }

    /// This edge under `id` rather than a minted one.
    #[must_use]
    pub fn with_id(mut self, id: EdgeId) -> Self {
        self.id = id;
        self
    }

    /// This edge carrying one more value of `property`.
    #[must_use]
    pub fn with_property(mut self, property: PropertyId, value: Value) -> Self {
        self.properties.entry(property).or_default().push(value);
        self
    }
}

/// What an assertion is about.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Subject {
    /// `!Node <id>`.
    Node(NodeId),
    /// `!Edge <id>`.
    Edge(EdgeId),
    /// `!Type <id>`.
    Type(TypeId),
}

/// What an assertion says of its subject.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Predicate {
    /// `!Property <id>`: the subject has this property value.
    Property(PropertyId),
    /// `!Relation <edge type id>`: the subject and object nodes are related.
    Relation(TypeId),
}

/// What the predicate relates the subject to.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Object {
    /// `!Value {value_kind, value}`.
    Value(Value),
    /// `!Node <id>`.
    Node(NodeId),
    /// `!Type <id>`.
    Type(TypeId),
}

/// An assertion's assessment as its proposer writes it. Committing makes it `Accepted`; a
/// proposer never writes another.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Assessment {
    /// `Proposed`.
    #[default]
    Proposed,
}

/// An assertion's lifecycle as its proposer writes it; retraction and supersession are
/// operations of their own.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssertionLifecycle {
    /// `Active`.
    #[default]
    Active,
}

/// A half-open valid time `[from, to)`; `None` is unbounded.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemporalRange {
    /// The first instant it holds, or unbounded.
    pub from: Option<Timestamp>,
    /// The first instant it no longer holds, or unbounded.
    pub to: Option<Timestamp>,
}

impl TemporalRange {
    /// Unbounded at both ends.
    pub const UNBOUNDED: Self = Self {
        from: None,
        to: None,
    };

    /// From `from` on, with no end.
    #[must_use]
    pub const fn since(from: Timestamp) -> Self {
        Self {
            from: Some(from),
            to: None,
        }
    }

    /// The range with these bounds, or `None` when `to` precedes `from`, which the readers
    /// refuse.
    #[must_use]
    pub const fn new(from: Option<Timestamp>, to: Option<Timestamp>) -> Option<Self> {
        if let (Some(from), Some(to)) = (from, to) {
            if to.millis() < from.millis() {
                return None;
            }
        }
        Some(Self { from, to })
    }
}

/// When an assertion was recorded. The kernel sets it at commit; a proposer writes
/// [`TransactionTime::UNRECORDED`].
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionTime {
    /// The first recorded instant.
    pub recorded_from: Timestamp,
    /// The last, or open.
    pub recorded_to: Option<Timestamp>,
}

impl TransactionTime {
    /// `{recorded_from: 0, recorded_to: null}`, what every proposal writes.
    pub const UNRECORDED: Self = Self {
        recorded_from: Timestamp::EPOCH,
        recorded_to: None,
    };
}

/// A claim with evidence and a valid time: a seed's assertion or `!AddAssertion`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assertion {
    /// Its id.
    pub id: AssertionId,
    /// The graph root it belongs to.
    pub root_id: GraphRootId,
    /// What it is about.
    pub subject: Subject,
    /// What it says of it.
    pub predicate: Predicate,
    /// What it relates the subject to.
    pub object: Object,
    /// The evidence it cites; at least one.
    pub evidence: BTreeSet<EvidenceId>,
    /// The host operator.
    pub proposed_by: AgentId,
    /// Always `Proposed`.
    pub assessment: Assessment,
    /// Always `Active`.
    pub lifecycle: AssertionLifecycle,
    /// When the claim is true in the world.
    pub valid_time: TemporalRange,
    /// Always [`TransactionTime::UNRECORDED`].
    pub transaction_time: TransactionTime,
}

impl Assertion {
    /// A proposed, active assertion, valid at every time and citing nothing yet, under a freshly
    /// minted id.
    #[must_use]
    pub fn new(
        root_id: GraphRootId,
        subject: Subject,
        predicate: Predicate,
        object: Object,
        proposed_by: AgentId,
    ) -> Self {
        Self {
            id: AssertionId::mint(),
            root_id,
            subject,
            predicate,
            object,
            evidence: BTreeSet::new(),
            proposed_by,
            assessment: Assessment::Proposed,
            lifecycle: AssertionLifecycle::Active,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::UNRECORDED,
        }
    }

    /// This assertion under `id` rather than a minted one.
    #[must_use]
    pub fn with_id(mut self, id: AssertionId) -> Self {
        self.id = id;
        self
    }

    /// This assertion citing one more evidence id.
    #[must_use]
    pub fn citing(mut self, evidence: EvidenceId) -> Self {
        self.evidence.insert(evidence);
        self
    }

    /// This assertion valid over `valid_time`.
    #[must_use]
    pub fn with_valid_time(mut self, valid_time: TemporalRange) -> Self {
        self.valid_time = valid_time;
        self
    }
}

/// Where a statement came from. `HumanStatement` is the one source the kernel admits today.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum EvidenceSource {
    /// `!HumanStatement {identity}`: who or what said it.
    HumanStatement {
        /// Who or what said it.
        identity: Option<String>,
    },
}

impl EvidenceSource {
    /// A statement `identity` made.
    #[must_use]
    pub fn human(identity: impl Into<String>) -> Self {
        Self::HumanStatement {
            identity: Some(identity.into()),
        }
    }
}

/// Basis points, 0 to 10000.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "u16", into = "u16")]
pub struct Confidence(u16);

/// Read as the kernel reads it: a value above ten thousand basis points is refused while decoding.
impl TryFrom<u16> for Confidence {
    type Error = String;

    fn try_from(basis_points: u16) -> Result<Self, Self::Error> {
        Self::new(basis_points).ok_or_else(|| {
            format!("{basis_points} is not a confidence: expected basis points between 0 and 10000")
        })
    }
}

impl From<Confidence> for u16 {
    fn from(confidence: Confidence) -> Self {
        confidence.0
    }
}

impl Confidence {
    /// Ten thousand basis points.
    pub const CERTAIN: Self = Self(10_000);

    /// `basis_points`, or `None` above ten thousand.
    #[must_use]
    pub const fn new(basis_points: u16) -> Option<Self> {
        if basis_points > Self::CERTAIN.0 {
            None
        } else {
            Some(Self(basis_points))
        }
    }

    /// The basis points.
    #[must_use]
    pub const fn basis_points(self) -> u16 {
        self.0
    }
}

/// An evidence entry: where a statement came from and the hash of its exact bytes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    /// Its id.
    pub id: EvidenceId,
    /// Where the statement came from.
    pub source: EvidenceSource,
    /// [`payload_hash`](super::payload_hash) of the statement's bytes.
    pub content_hash: ContentHash,
    /// The host operator.
    pub extracted_by: AgentId,
    /// When it was observed.
    pub observed_at: Timestamp,
    /// How certain it is.
    pub confidence: Confidence,
}

impl Evidence {
    /// The entry for `payload`, its hash computed here, under a freshly minted id.
    #[must_use]
    pub fn for_payload(
        payload: &[u8],
        source: EvidenceSource,
        extracted_by: AgentId,
        observed_at: Timestamp,
        confidence: Confidence,
    ) -> Self {
        Self {
            id: EvidenceId::mint(),
            source,
            content_hash: super::payload_hash(payload),
            extracted_by,
            observed_at,
            confidence,
        }
    }
}
