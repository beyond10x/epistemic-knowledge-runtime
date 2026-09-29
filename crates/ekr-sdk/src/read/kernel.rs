//! The documents `ekr head`, `ekr snapshot`, `ekr ontology`, `ekr transactions` and
//! `ekr explain` print, typed (`docs/cli.md` § Verbs).
//!
//! Ids, hashes, times and names are typed; what varies by kind inside a record — an assertion's
//! object, assessment and lifecycle, an evidence entry's source, the retained records of an
//! explanation's origin links — stays JSON, read by its `kind` or tag as the page documents it.
//! Each type writes back exactly the document it was read from; `crates/ekr-sdk/tests/read.rs`
//! holds that against real `ekr` output.

use std::collections::BTreeMap;

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionId, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::document::{
    Cardinality, GraphRoot, Predicate, PropertyDefinition, Subject, TemporalRange, TransactionTime,
};

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
    pub root: GraphRoot,
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
    pub subject: Subject,
    /// What it says of it.
    pub predicate: Predicate,
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
    pub valid_time: TemporalRange,
    /// When it was recorded.
    pub transaction_time: TransactionTime,
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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
}

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
    pub properties: Vec<PropertyDefinition>,
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
    pub cardinality: Cardinality,
    /// Its properties.
    pub properties: Vec<PropertyDefinition>,
}

/// A retained transaction's state, as `ekr transactions` prints and `--state` takes it.
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
    /// The assertion asked about.
    pub assertion_id: AssertionId,
    /// The revision every link was read at.
    pub at: u64,
    /// The chain, to be read by kind, never by position.
    pub links: Vec<ExplanationLink>,
}

/// One link of an explanation, by its `kind`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ExplanationLink {
    /// The asked assertion, or one that superseded it, as it stands.
    Assertion(SnapshotAssertion),
    /// The seed it came from.
    Seed(Map<String, Value>),
    /// The retained proposal of the transaction that added it.
    Proposal(Map<String, Value>),
    /// That transaction's validation.
    Validation(Map<String, Value>),
    /// That transaction's commit receipt.
    Commit(Map<String, Value>),
    /// A later retraction or supersession of it.
    Lifecycle(Map<String, Value>),
    /// Evidence cited, with its retained bytes.
    Evidence(ExplainedEvidence),
}

/// An evidence link: the entry, its retained bytes and, when they are UTF-8, their text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ExplainedEvidence {
    /// The evidence entry.
    #[serde(flatten)]
    pub evidence: SnapshotEvidence,
    /// The retained bytes, standard padded base64.
    pub payload: String,
    /// The same bytes as text, when they are UTF-8.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}
