//! The documents `ekr head`, `ekr snapshot`, `ekr ontology`, `ekr transactions` and
//! `ekr explain` print, typed (`docs/cli.md` § Verbs).
//!
//! Ids, hashes, times and names are typed; what varies by kind inside a record — an assertion's
//! object, assessment and lifecycle, an evidence entry's source, the retained records of an
//! explanation's origin links — stays JSON, read by its `kind` or tag as the page documents it.
//! Each type writes back exactly the document it was read from; `crates/ekr-sdk/tests/read.rs`
//! holds that against real `ekr` output. Every record here is the read side's own and ignores a
//! field it does not know, so a newer `ekr` does not break an older consumer; the strict types of
//! [`crate::document`] are for the documents a consumer writes.

use std::collections::BTreeMap;

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionId, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// A revision's root: its number and the hashes of its state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Root {
    /// Zero at the seed, one more per commit.
    pub revision: u64,
    /// The root before it; `None` at the seed.
    pub parent: Option<ContentHash>,
    /// The ontology state.
    pub ontology_root: ContentHash,
    /// The graph state.
    pub knowledge_root: ContentHash,
    /// The retained evidence.
    pub evidence_root: ContentHash,
    /// The agent registry.
    pub agent_root: ContentHash,
    /// The transaction that produced it.
    pub transaction: ContentHash,
}

/// `ekr head`: the newest revision and its root.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Head {
    /// The head revision.
    pub revision: u64,
    /// Its root.
    pub root: Root,
}

/// `ekr snapshot`: the graph at one revision.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    /// The revision's identity.
    pub revision_id: RevisionId,
    /// Its root.
    pub root: Root,
    /// The graph, as an `ekr.graph-document/2`.
    pub graph: SnapshotGraphDocument,
    /// The instant `--valid-at` asked for.
    pub valid_at: Option<Timestamp>,
    /// The assertions believed at `valid_at`, by id; `None` without `--valid-at`.
    pub matching_assertions: Option<Vec<AssertionId>>,
}

/// The `ekr.graph-document/2` envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotGraphDocument {
    /// `ekr.graph-document/2`.
    pub format: String,
    /// The graph.
    pub graph: SnapshotGraph,
}

/// The canonical graph, every map keyed by the id its entries carry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotGraph {
    /// The root every entity names.
    pub root: SnapshotRoot,
    /// The revision, where the document records it.
    pub revision: Option<u64>,
    /// Its nodes.
    pub nodes: BTreeMap<NodeId, SnapshotNode>,
    /// Its edges.
    pub edges: BTreeMap<EdgeId, SnapshotEdge>,
    /// Every assertion, retracted and superseded ones included.
    pub assertions: BTreeMap<AssertionId, SnapshotAssertion>,
    /// Its evidence entries.
    pub evidence: BTreeMap<EvidenceId, SnapshotEvidence>,
    /// The evidence attached to its assertions after they were added, by assertion id, each list
    /// in evidence-id order. Empty, and absent from the document, when there is none.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub attachments: BTreeMap<AssertionId, Vec<SnapshotAttachment>>,
}

/// One piece of evidence attached to an assertion after it was added.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotAttachment {
    /// The evidence attached.
    pub evidence: EvidenceId,
    /// The revision that attached it.
    pub revision: u64,
}

/// A canonical node.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotNode {
    /// Its id.
    pub id: NodeId,
    /// Its graph root.
    pub root_id: GraphRootId,
    /// Its type.
    pub type_id: TypeId,
    /// Its canonical name.
    pub canonical_name: String,
    /// Its values, by property id, each as its tagged JSON.
    pub properties: BTreeMap<PropertyId, Vec<Value>>,
    /// Its aliases.
    pub aliases: Vec<String>,
    /// Its lifecycle state, when its type has a lifecycle.
    pub type_state: Option<String>,
}

/// A canonical edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotEdge {
    /// Its id.
    pub id: EdgeId,
    /// Its graph root.
    pub root_id: GraphRootId,
    /// Its type.
    pub type_id: TypeId,
    /// Its source.
    pub source: NodeId,
    /// Its target.
    pub target: NodeId,
    /// Its values, by property id, each as its tagged JSON.
    pub properties: BTreeMap<PropertyId, Vec<Value>>,
}

/// A canonical assertion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotAssertion {
    /// Its id.
    pub id: AssertionId,
    /// Its graph root.
    pub root_id: GraphRootId,
    /// What it is about.
    pub subject: SnapshotSubject,
    /// What it says of it.
    pub predicate: SnapshotPredicate,
    /// Its object: `{"Value": …}`, `{"Node": <id>}` or `{"Type": <id>}`.
    pub object: Value,
    /// The evidence it cites.
    pub evidence: Vec<EvidenceId>,
    /// Who proposed it.
    pub proposed_by: AgentId,
    /// Its assessment: `"Proposed"`, `{"Accepted": {"validators": […]}}`, …
    pub assessment: Value,
    /// Its lifecycle: `"Active"`, or a retraction or supersession.
    pub lifecycle: Value,
    /// When it holds.
    pub valid_time: ValidTime,
    /// When it was recorded.
    pub transaction_time: RecordedTime,
}

/// A canonical evidence entry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotEvidence {
    /// Its id.
    pub id: EvidenceId,
    /// Where it came from: `{"HumanStatement": {…}}`, …
    pub source: Value,
    /// Its payload's content hash.
    pub content_hash: ContentHash,
    /// Who extracted it.
    pub extracted_by: AgentId,
    /// When it was observed.
    pub observed_at: Timestamp,
    /// Its confidence in basis points.
    pub confidence: u16,
}

/// `ekr ontology`: the schema in force at a revision, by name and id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ontology {
    /// The revision read.
    pub revision: u64,
    /// The schema version in force.
    pub schema_version: SchemaVersionId,
    /// Its number: 0 at the seed.
    pub schema_version_number: u64,
    /// The version it was derived from; `None` at the seed.
    pub schema_version_parent: Option<SchemaVersionId>,
    /// Its node types.
    pub node_types: Vec<OntologyNodeType>,
    /// Its edge types.
    pub edge_types: Vec<OntologyEdgeType>,
    /// Immutable schema transactions' cited evidence through this revision, omitted when empty.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supporting_evidence: Vec<super::SchemaEvidenceEntry>,
}

// The generated evidence entry contains only integer revisions and string identities, all Eq.
impl Eq for Ontology {}

/// A type by id, with its name when the ontology names it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedType {
    /// Its id.
    pub id: TypeId,
    /// Its name.
    pub name: Option<String>,
}

/// A node type of `ekr ontology`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OntologyNodeType {
    /// Its id.
    pub id: TypeId,
    /// Its name.
    pub name: String,
    /// Its parents.
    pub parents: Vec<NamedType>,
    /// Whether it is abstract.
    pub abstract_type: bool,
    /// Its properties.
    pub properties: Vec<OntologyProperty>,
}

/// An edge type of `ekr ontology`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OntologyEdgeType {
    /// Its id.
    pub id: TypeId,
    /// Its name.
    pub name: String,
    /// The node types it may start at.
    pub source_types: Vec<NamedType>,
    /// The node types it may end at.
    pub target_types: Vec<NamedType>,
    /// How many it may have per source.
    pub cardinality: OntologyCardinality,
    /// Its properties.
    pub properties: Vec<OntologyProperty>,
}

/// A retained transaction's state, as `ekr transactions` prints and `--state` takes it. A state a
/// newer `ekr` adds reads as [`TransactionState::Other`] and writes back as `Other`; `ekr` names
/// no state `Other`, so passing it to `--state` is refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TransactionState {
    /// Proposed, not yet validated.
    Proposed,
    /// Validated, not yet committed.
    Validated,
    /// Committed.
    Committed,
    /// Refused for good by validation.
    Rejected,
    /// The head moved after validation.
    Stale,
    /// A state this SDK does not know, added by a newer `ekr`.
    #[serde(other)]
    Other,
}

impl TransactionState {
    /// The name `--state` takes.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "Proposed",
            Self::Validated => "Validated",
            Self::Committed => "Committed",
            Self::Rejected => "Rejected",
            Self::Stale => "Stale",
            Self::Other => "Other",
        }
    }
}

/// `ekr transactions`: every retained transaction listed, in id order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Transactions(pub Vec<ListedTransaction>);

/// One retained transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListedTransaction {
    /// Its id.
    pub transaction_id: TransactionId,
    /// Its state.
    pub state: TransactionState,
    /// Who proposed it.
    pub proposer: AgentId,
    /// When it was proposed.
    pub submitted_at: Timestamp,
    /// How many operations it holds.
    pub operation_count: u64,
}

/// `ekr explain`: an assertion, where it came from, what later changed it, and its evidence.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Explanation {
    /// The answer's format, `ekr.explanation/2`: proposals, commit receipts and evidence named
    /// by hash. Empty for the unversioned answer of an `ekr` before it, which embedded each
    /// whole record.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub format: String,
    /// The assertion asked about.
    pub assertion_id: AssertionId,
    /// The revision every link was read at.
    pub at: u64,
    /// The chain, to be read by kind, never by position.
    pub links: Vec<ExplanationLink>,
}

/// One link of an explanation, by its `kind`. A kind a newer `ekr` adds reads as
/// [`ExplanationLink::Other`], its fields dropped, and writes back as `{"kind": "Other"}`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ExplanationLink {
    /// The asked assertion, or one that superseded it, as it stands.
    Assertion(SnapshotAssertion),
    /// The seed it came from.
    Seed(Map<String, Value>),
    /// The transaction that added it: its proposal by reference, with the operations about the
    /// assertion, and with `documents` the whole proposal record as `record`.
    Proposal(Map<String, Value>),
    /// That transaction's validation.
    Validation(Map<String, Value>),
    /// That transaction's commit by reference, and with `documents` its receipt as `receipt`.
    Commit(Map<String, Value>),
    /// A later retraction or supersession of it, with its commit by reference.
    Lifecycle(Map<String, Value>),
    /// Evidence attached to it after it was added, with the attaching commit by reference.
    Attachment(ExplainedAttachment),
    /// A signed correction, its immutable record and exact record address.
    HumanAnswer(Box<crate::contracts::EkrKernelExplainedAnswer>),
    /// Exact retained mapping bytes from a committed schema application.
    Mapping(Box<crate::contracts::EkrIntegrateRetainedMappingRecord>),
    /// An assertion's committed evidence-to-mapping correspondence.
    Derivation(Box<crate::contracts::EkrIntegrateCanonicalDerivationRecord>),
    /// Evidence cited, by its content hash, and with `documents` its retained bytes.
    Evidence(ExplainedEvidence),
    /// A kind this SDK does not know, added by a newer `ekr`.
    #[serde(other)]
    Other,
}

/// An attachment link: which evidence was attached to which assertion, at which revision, and the
/// commit that attached it — by reference, and with the documents its receipt as `receipt`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplainedAttachment {
    /// The assertion the evidence is attached to.
    pub assertion_id: AssertionId,
    /// The evidence attached; one of the explanation's Evidence links.
    pub evidence_id: EvidenceId,
    /// The revision that attached it.
    pub revision: u64,
    /// The attaching commit, as a `Commit` link carries it.
    pub commit: Map<String, Value>,
}

/// An evidence link: the entry and, when the explanation was read with its documents
/// ([`super::Reader::explain_documents`]), its retained bytes and, when they are UTF-8, their text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplainedEvidence {
    /// The evidence entry.
    #[serde(flatten)]
    pub evidence: SnapshotEvidence,
    /// The retained bytes, standard padded base64, when read with the documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<String>,
    /// The same bytes as text, when they are UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// The graph root a snapshot's entities name, as `ekr snapshot` prints it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotRoot {
    /// Its id.
    pub id: GraphRootId,
    /// Its space: `Canonical`.
    pub space: String,
    /// The schema version it was created under.
    pub schema_version_id: SchemaVersionId,
    /// The root it was derived from; `None` for the seed's.
    pub parent: Option<GraphRootId>,
    /// When it was created.
    pub created_at: Timestamp,
}

/// When an assertion holds: half-open, `from` included and `to` not; `None` is unbounded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidTime {
    /// From, included.
    pub from: Option<Timestamp>,
    /// To, excluded.
    pub to: Option<Timestamp>,
}

/// When an assertion was recorded: from its commit, to the commit that ended it, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedTime {
    /// Recorded from.
    pub recorded_from: Timestamp,
    /// Recorded to; `None` while it is still recorded.
    pub recorded_to: Option<Timestamp>,
}

/// A property declaration as `ekr ontology` prints it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OntologyProperty {
    /// Its id: the key a document writes its values under.
    pub id: PropertyId,
    /// Its name.
    pub name: String,
    /// Its value type.
    pub value_type: OntologyValueType,
    /// How many values a node or edge may hold.
    pub cardinality: OntologyCardinality,
    /// Whether a value is required.
    pub required: bool,
    /// Its constraints.
    pub constraints: Vec<String>,
}

/// A declared value type, `{value_kind, parameters}`, as `ekr ontology` prints it. A kind a
/// newer `ekr` adds reads as [`OntologyValueType::Other`], its parameters dropped, and writes
/// back as `{"value_kind": "Other"}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OntologyValueType {
    /// Text.
    String,
    /// A truth value.
    Boolean,
    /// A signed 64-bit whole number.
    Integer,
    /// An approximate number.
    Float,
    /// An exact number held as text.
    Decimal,
    /// Milliseconds since the Unix epoch.
    Timestamp,
    /// A signed whole number.
    Duration,
    /// A reference to a node of one of these types.
    NodeRef {
        /// The node types a value may point at.
        allowed_types: Vec<TypeId>,
    },
    /// One of these variants.
    Enum {
        /// The variants.
        variants: Vec<String>,
    },
    /// A sequence of elements of this type.
    List(Box<OntologyValueType>),
    /// Exactly these fields.
    Record(BTreeMap<String, OntologyValueType>),
    /// A kind this SDK does not know, added by a newer `ekr`.
    Other,
}

/// [`OntologyValueType`]'s wire shape, for serde to derive its reader and writer from. Private,
/// so the only public reader is the tolerant `Deserialize` impl.
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(
    remote = "OntologyValueType",
    tag = "value_kind",
    content = "parameters"
)]
enum OntologyValueTypeWire {
    String,
    Boolean,
    Integer,
    Float,
    Decimal,
    Timestamp,
    Duration,
    NodeRef {
        allowed_types: Vec<TypeId>,
    },
    Enum {
        variants: Vec<String>,
    },
    List(Box<OntologyValueType>),
    Record(BTreeMap<String, OntologyValueType>),
    #[serde(other)]
    Other,
}

impl Serialize for OntologyValueType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        OntologyValueTypeWire::serialize(self, serializer)
    }
}

impl OntologyValueType {
    /// Reads a value type as the `Deserialize` impl does, so `OntologyValueType::deserialize(…)`
    /// without the trait in scope is the same tolerant reader: a kind a newer `ekr` adds is
    /// `Other`.
    ///
    /// # Errors
    ///
    /// A document that is no value type: a known kind whose parameters are wrong, or no
    /// `value_kind`.
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        <Self as Deserialize<'de>>::deserialize(deserializer)
    }
}

impl<'de> Deserialize<'de> for OntologyValueType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        super::tolerant(
            deserializer,
            Some("parameters"),
            OntologyValueTypeWire::deserialize,
            |read| *read == Self::Other,
        )
    }
}

/// How many values a property may carry, or how many edges of a type may leave one node, as
/// `ekr ontology` prints it. The read side's own: a cardinality a newer `ekr` adds reads as
/// [`OntologyCardinality::Other`] and writes back as `Other`. A document a consumer writes takes
/// [`crate::document::Cardinality`], which has no `Other`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OntologyCardinality {
    /// At most one.
    One,
    /// Any number.
    Many,
    /// A cardinality this SDK does not know, added by a newer `ekr`.
    #[serde(other)]
    Other,
}

/// What a snapshot assertion is about, `{"Node": <id>}`, `{"Edge": <id>}` or `{"Type": <id>}`.
/// The read side's own: a subject kind a newer `ekr` adds reads as [`SnapshotSubject::Other`],
/// its id dropped, and writes back as `"Other"`. A document a consumer writes takes
/// [`crate::document::Subject`], which has no `Other`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SnapshotSubject {
    /// A node.
    Node(NodeId),
    /// An edge.
    Edge(EdgeId),
    /// A type.
    Type(TypeId),
    /// A kind this SDK does not know, added by a newer `ekr`.
    Other,
}

/// [`SnapshotSubject`]'s wire shape. Private, so the only public reader is the tolerant one.
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(remote = "SnapshotSubject")]
enum SnapshotSubjectWire {
    Node(NodeId),
    Edge(EdgeId),
    Type(TypeId),
    #[serde(other)]
    Other,
}

impl Serialize for SnapshotSubject {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SnapshotSubjectWire::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for SnapshotSubject {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        super::tolerant(
            deserializer,
            None,
            SnapshotSubjectWire::deserialize,
            |read| *read == Self::Other,
        )
    }
}

/// What a snapshot assertion says of its subject, `{"Property": <id>}` or `{"Relation": <id>}`.
/// The read side's own: a predicate kind a newer `ekr` adds reads as
/// [`SnapshotPredicate::Other`], its id dropped, and writes back as `"Other"`. A document a
/// consumer writes takes [`crate::document::Predicate`], which has no `Other`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SnapshotPredicate {
    /// The subject has this property's value.
    Property(PropertyId),
    /// The subject and object nodes are related by an edge of this type.
    Relation(TypeId),
    /// A kind this SDK does not know, added by a newer `ekr`.
    Other,
}

/// [`SnapshotPredicate`]'s wire shape. Private, so the only public reader is the tolerant one.
#[allow(dead_code)]
#[derive(Serialize, Deserialize)]
#[serde(remote = "SnapshotPredicate")]
enum SnapshotPredicateWire {
    Property(PropertyId),
    Relation(TypeId),
    #[serde(other)]
    Other,
}

impl Serialize for SnapshotPredicate {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        SnapshotPredicateWire::serialize(self, serializer)
    }
}

impl<'de> Deserialize<'de> for SnapshotPredicate {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        super::tolerant(
            deserializer,
            None,
            SnapshotPredicateWire::deserialize,
            |read| *read == Self::Other,
        )
    }
}
