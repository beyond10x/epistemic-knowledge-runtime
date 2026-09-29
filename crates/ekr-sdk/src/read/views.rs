//! The `ekr.views` documents, typed: what `ekr view`, `ekr mcp` and the session's views verbs
//! answer (`docs/cli.md` § `ekr session`, "The `ekr.views` reads").
//!
//! Each type writes back exactly the JSON it was read from: every field the format carries, and
//! an optional one only when the document had it. `crates/ekr-sdk/tests/read.rs` holds that
//! against every document the `ekr-views` conformance fixture stores render, so a field a format
//! gains without an update here fails there. A reader ignores a field it does not know, so a
//! newer `ekr` does not break an older consumer.

use std::collections::BTreeMap;

use ekr_core::{AssertionId, EdgeId, NodeId, TransactionId, TypeId};
use serde::{Deserialize, Serialize};

// ---- shared records ------------------------------------------------------------------------------

/// A property value as the views write it: `{"kind": …, "value": …}`. Ids and decimals are text,
/// times and durations milliseconds.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum ViewValue {
    /// Text.
    String(String),
    /// True or false.
    Boolean(bool),
    /// A whole number.
    Integer(i64),
    /// A decimal, as its text.
    Decimal(String),
    /// Milliseconds since the Unix epoch.
    Timestamp(i64),
    /// Milliseconds.
    Duration(i64),
    /// A node's id, as text.
    NodeRef(String),
    /// One of an enum's variants.
    Enum(String),
    /// A list of values.
    List(Vec<ViewValue>),
    /// Named values.
    Record(BTreeMap<String, ViewValue>),
}

/// `ekr.graph.AssessmentProjection`: an assertion's assessment, with exactly the payload its kind
/// carries.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewAssessment {
    /// `Proposed`, `Accepted`, `Rejected`, `Disputed`, …
    pub kind: String,
    /// Validations completed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<u32>,
    /// Validations required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<u32>,
    /// The validators, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validators: Option<Vec<String>>,
    /// The issues a rejection names.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issues: Option<Vec<String>>,
    /// The assertions a dispute names, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub competing_assertions: Option<Vec<String>>,
}

/// An assertion's lifecycle: `Active`, or what retracted or superseded it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewLifecycle {
    /// `Active`, `Retracted` or `Superseded`.
    pub kind: String,
    /// The revision that changed it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at_revision: Option<u64>,
    /// Why it was retracted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// The assertion that superseded it, by id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<String>,
    /// When the supersession takes effect, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_from: Option<i64>,
}

/// An assertion as the views project it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewAssertion {
    /// Its id.
    pub id: AssertionId,
    /// `Property` or `Relation`.
    pub predicate_kind: String,
    /// The property's or edge type's id.
    pub predicate: String,
    /// `Value`, `Node` or `Type`.
    pub object_kind: String,
    /// The value, for a `Value` object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_value: Option<ViewValue>,
    /// The node's or type's id, for any other object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object_ref: Option<String>,
    /// Its assessment.
    pub assessment: ViewAssessment,
    /// Its lifecycle.
    pub lifecycle: ViewLifecycle,
    /// Valid from, in milliseconds; unbounded when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_from: Option<i64>,
    /// Valid to, in milliseconds; unbounded when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_to: Option<i64>,
    /// Recorded from, in milliseconds.
    pub recorded_from: i64,
    /// Recorded to, in milliseconds; still recorded when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorded_to: Option<i64>,
    /// The evidence it cites, by id.
    pub evidence: Vec<String>,
}

/// An edge with its properties and the assertions about it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewEdge {
    /// Its id.
    pub id: EdgeId,
    /// Its source node.
    pub source: NodeId,
    /// Its target node.
    pub target: NodeId,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its values, by property id.
    pub props: BTreeMap<String, Vec<ViewValue>>,
    /// The assertions about it, by id order.
    pub assertions: Vec<ViewAssertion>,
}

/// `ekr.views.NodeSummary`: what a viewer needs to draw a node.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeSummary {
    /// The node.
    pub id: NodeId,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its canonical name.
    pub name: String,
    /// Its edges, self-loops not counted.
    pub degree: u64,
}

// ---- ekr.graph-overview/1 ----------------------------------------------------------------------

/// `ekr.graph-overview/1`: a revision's counts, schema, types, roles, timeline and top nodes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overview {
    /// The revision and its counts.
    pub meta: OverviewMeta,
    /// The ontology in force.
    pub ontology: ViewOntology,
    /// The schema's history up to the revision.
    pub schema: OverviewSchema,
    /// Nodes per node type.
    pub node_types: Vec<TypeCount>,
    /// Edges per edge type.
    pub edge_types: Vec<TypeCount>,
    /// The node types' roles in time.
    pub roles: OverviewRoles,
    /// Dated assertions per time bucket.
    pub timeline: OverviewTimeline,
    /// The highest-degree nodes, at most `meta.limit`.
    pub top: Vec<NodeSummary>,
}

/// The overview's meta.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverviewMeta {
    /// `ekr.graph-overview/1`.
    pub format: String,
    /// The revision read.
    pub revision: u64,
    /// Its nodes.
    pub node_count: u64,
    /// Its edges.
    pub edge_count: u64,
    /// Its assertions.
    pub assertion_count: u64,
    /// Its evidence entries.
    pub evidence_count: u64,
    /// The `top` limit asked for.
    pub limit: u64,
}

/// The ontology as the views project it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewOntology {
    /// Its node types.
    pub node_types: Vec<ViewNodeType>,
    /// Its edge types.
    pub edge_types: Vec<ViewEdgeType>,
    /// Every property either declares.
    pub properties: Vec<ViewProperty>,
}

/// A node type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewNodeType {
    /// Its id.
    pub id: TypeId,
    /// Its name.
    pub name: String,
    /// Its properties, by id.
    pub properties: Vec<String>,
    /// The assertions about it.
    pub assertions: Vec<ViewAssertion>,
}

/// An edge type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewEdgeType {
    /// Its id.
    pub id: TypeId,
    /// Its name.
    pub name: String,
    /// The node types it may start at, by id.
    pub source_types: Vec<String>,
    /// The node types it may end at, by id.
    pub target_types: Vec<String>,
    /// Its properties, by id.
    pub properties: Vec<String>,
    /// The assertions about it.
    pub assertions: Vec<ViewAssertion>,
}

/// A property declaration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewProperty {
    /// Its id.
    pub id: String,
    /// Its name.
    pub name: String,
    /// Its value kind.
    pub value_kind: String,
}

/// The schema's history up to the revision.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverviewSchema {
    /// Each schema version and what it added and removed.
    pub versions: Vec<SchemaVersionChange>,
    /// Each revision and the version it is valid against.
    pub revisions: Vec<OverviewRevision>,
    /// Nodes no revision record names.
    pub unrecorded_nodes: u64,
    /// Edges no revision record names.
    pub unrecorded_edges: u64,
}

/// One schema version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaVersionChange {
    /// Its number: 0 at the seed.
    pub number: u64,
    /// Its id.
    pub id: String,
    /// The version it was derived from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent: Option<String>,
    /// The first revision valid against it.
    pub revision: u64,
    /// What it added.
    pub added: Vec<SchemaMember>,
    /// What it removed.
    pub removed: Vec<SchemaMember>,
}

/// A type or property a schema version added or removed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaMember {
    /// Its id.
    pub id: String,
    /// What it is.
    pub kind: String,
    /// Its name.
    pub name: String,
}

/// One revision of the schema history.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverviewRevision {
    /// Its number.
    pub number: u64,
    /// When it was committed, in milliseconds.
    pub committed_at: i64,
    /// The transaction that made it; absent for the seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<TransactionId>,
    /// The schema version it is valid against.
    pub schema_version: String,
    /// Its nodes.
    pub nodes: u64,
    /// Its edges.
    pub edges: u64,
    /// Its assertions.
    pub assertions: u64,
}

/// How many of one type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeCount {
    /// The type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// How many.
    pub count: u64,
}

/// The node types' roles in time.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverviewRoles {
    /// Each node type's timing.
    pub types: Vec<TypeTiming>,
    /// The type whose nodes are observations, when one is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_type: Option<TypeId>,
}

/// One node type's timing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeTiming {
    /// The type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its nodes.
    pub nodes: u64,
    /// Of those, the ones with a dated assertion.
    pub timestamped: u64,
    /// Of those, the ones judged.
    pub judged: u64,
    /// Of those, the ones whose dates fall within an hour.
    pub within_hour: u64,
    /// Of those, the ones dated to an instant.
    pub instant: u64,
    /// The node types its nodes neighbour.
    pub neighbour_types: u64,
    /// Whether its nodes are events.
    pub event: bool,
}

/// Dated assertions per time bucket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverviewTimeline {
    /// The bucket width, in milliseconds.
    pub bucket_ms: u64,
    /// The first dated instant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<i64>,
    /// The last dated instant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<i64>,
    /// Assertions without a date.
    pub undated: u64,
    /// The buckets.
    pub buckets: Vec<TimelineBucket>,
}

/// One bucket of one type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineBucket {
    /// Its start, in milliseconds.
    pub start: i64,
    /// The type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its assertions.
    pub assertions: u64,
}

// ---- ekr.node-matches/1 ------------------------------------------------------------------------

/// `ekr.node-matches/1`: the nodes whose name or an alias contains a text, exact matches first.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMatches {
    /// The search and its totals.
    pub meta: MatchesMeta,
    /// The matches, at most the limit.
    pub matches: Vec<NodeMatch>,
}

/// The search and its totals.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MatchesMeta {
    /// `ekr.node-matches/1`.
    pub format: String,
    /// The revision read.
    pub revision: u64,
    /// The text searched for.
    pub text: String,
    /// Every match, before the limit.
    pub total: u64,
    /// The exact ones among them.
    pub exact_total: u64,
}

/// One match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeMatch {
    /// The node.
    pub id: NodeId,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its canonical name.
    pub name: String,
    /// Its degree.
    pub degree: u64,
    /// How it matched.
    pub tier: MatchTier,
    /// What matched.
    pub field: MatchField,
    /// The alias that matched, when one did.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// How a match compared: `Exact` orders first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MatchTier {
    /// The text byte for byte.
    Exact,
    /// The text case-folded.
    Folded,
}

/// What matched.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MatchField {
    /// The canonical name.
    Name,
    /// An alias.
    Alias,
}

// ---- ekr.node-detail/1 -------------------------------------------------------------------------

/// `ekr.node-detail/1`: one node, every assertion about it or naming it, its edges and
/// neighbours.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeDetail {
    /// The revision.
    pub meta: DetailMeta,
    /// The node.
    pub node: DetailNode,
    /// The assertions about it.
    pub assertions: Vec<ViewAssertion>,
    /// The assertions whose object it is.
    pub referencing: Vec<ReferencingAssertion>,
    /// Its edges.
    pub edges: Vec<ViewEdge>,
    /// The nodes those edges reach.
    pub neighbours: Vec<NodeSummary>,
}

/// The detail's meta.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailMeta {
    /// `ekr.node-detail/1`.
    pub format: String,
    /// The revision read.
    pub revision: u64,
}

/// The node described.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetailNode {
    /// Its id.
    pub id: NodeId,
    /// Its canonical name.
    pub name: String,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its aliases.
    pub aliases: Vec<String>,
    /// Its lifecycle state, when its type has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    /// Its values, by property id.
    pub props: BTreeMap<String, Vec<ViewValue>>,
    /// Its degree.
    pub degree: u64,
}

/// An assertion that names the node, with its own subject.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferencingAssertion {
    /// `Node`, `Edge` or `Type`.
    pub subject_kind: String,
    /// Its subject's id.
    pub subject: String,
    /// The assertion.
    pub assertion: ViewAssertion,
}

// ---- ekr.graph-slice/1 -------------------------------------------------------------------------

/// `ekr.graph-slice/1`: one page of a neighbourhood.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Slice {
    /// The request and the neighbourhood's totals.
    pub meta: SliceMeta,
    /// The page's nodes.
    pub nodes: Vec<SliceNode>,
    /// The page's edges, each after both its ends.
    pub edges: Vec<SliceEdge>,
    /// The next page's cursor; absent on the last page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<u64>,
}

/// The slice's meta.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceMeta {
    /// `ekr.graph-slice/1`.
    pub format: String,
    /// The revision read.
    pub revision: u64,
    /// The seeds, as a set.
    pub seeds: Vec<NodeId>,
    /// The depth asked for.
    pub depth: u64,
    /// The page's cursor.
    pub after: u64,
    /// Nodes of the whole neighbourhood.
    pub node_total: u64,
    /// Edges of the whole neighbourhood.
    pub edge_total: u64,
}

/// A node of a neighbourhood.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceNode {
    /// The node.
    pub id: NodeId,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its canonical name.
    pub name: String,
    /// Its degree over the whole revision.
    pub degree: u64,
    /// Hops from the nearest seed.
    pub distance: u64,
}

/// An edge of a neighbourhood.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SliceEdge {
    /// The edge.
    pub id: EdgeId,
    /// Its source.
    pub source: NodeId,
    /// Its target.
    pub target: NodeId,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// The assertions about it.
    pub assertions: Vec<AssertionId>,
}

// ---- ekr.graph-timeline/1 ----------------------------------------------------------------------

/// `ekr.graph-timeline/1`: rows of subjects with their events counted per bucket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Timeline {
    /// The request and its totals.
    pub meta: TimelineMeta,
    /// The row types, ranked.
    pub row_types: Vec<TimelineRowType>,
    /// The rows, the most active first.
    pub rows: Vec<TimelineRow>,
    /// Every row's events per bucket.
    pub strip: Vec<TimelineCell>,
    /// The subject's events, with a subject.
    pub events: Vec<TimelineEvent>,
}

/// The timeline's meta.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineMeta {
    /// `ekr.graph-timeline/1`.
    pub format: String,
    /// The revision read.
    pub revision: u64,
    /// The row type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub row_type: Option<TypeId>,
    /// The subject asked for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<NodeId>,
    /// The hops asked for.
    pub hops: u64,
    /// The limit asked for.
    pub limit: u64,
    /// The bucket width, in milliseconds.
    pub bucket_ms: u64,
    /// The first event's start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<i64>,
    /// The last event's end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<i64>,
    /// Subjects of the row type.
    pub subjects: u64,
    /// Those with an event.
    pub active: u64,
    /// Events in all.
    pub events: u64,
}

/// A row type and its rank.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineRowType {
    /// The type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its nodes.
    pub nodes: u64,
    /// Those an event reaches.
    pub reached: u64,
    /// Its rank's weight.
    pub weight: u64,
}

/// One subject's row.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineRow {
    /// The subject.
    pub id: NodeId,
    /// Its name.
    pub name: String,
    /// Its events.
    pub total: u64,
    /// Its first event's start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first: Option<i64>,
    /// Its last event's end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last: Option<i64>,
    /// Its events per bucket and type.
    pub cells: Vec<TimelineCell>,
}

/// Events of one type in one bucket.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineCell {
    /// The bucket's start, in milliseconds.
    pub start: i64,
    /// The event type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its events.
    pub events: u64,
}

/// One event of the subject.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// The event node.
    pub id: NodeId,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its name.
    pub name: String,
    /// Its start, in milliseconds.
    pub start: i64,
    /// Its end, in milliseconds.
    pub end: i64,
    /// Hops from the subject.
    pub distance: u64,
    /// The path from the subject, one step per hop.
    pub path: Vec<TimelineStep>,
}

/// One hop of an event's path.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineStep {
    /// The edge crossed.
    pub edge: EdgeId,
    /// Its type.
    pub edge_type: TypeId,
    /// Whether it was crossed from source to target.
    pub forward: bool,
    /// The node reached.
    pub node: NodeId,
    /// Its type.
    #[serde(rename = "type")]
    pub type_id: TypeId,
    /// Its name.
    pub name: String,
}

// ---- ekr.graph-changes/1 -----------------------------------------------------------------------

/// `ekr.graph-changes/1`: one page of what changed after a since.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    /// The since, the revision read and the totals.
    pub meta: ChangesMeta,
    /// The changes, by revision, kind and id.
    pub changes: Vec<GraphChange>,
    /// The next page's cursor; absent on the last page.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<u64>,
    /// Changes after this page.
    pub remaining: u64,
}

/// The changes' meta: exactly one since.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangesMeta {
    /// `ekr.graph-changes/1`.
    pub format: String,
    /// The last revision read.
    pub revision: u64,
    /// A revision since.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since_revision: Option<i64>,
    /// A valid-time since, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since_valid: Option<i64>,
    /// A transaction-time since, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub since_recorded: Option<i64>,
    /// The limit asked for.
    pub limit: u64,
    /// The page's cursor.
    pub after: u64,
    /// Every change chosen, before the limit.
    pub total: u64,
}

/// One change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphChange {
    /// The revision that made it.
    pub revision: u64,
    /// When that revision was committed, in milliseconds.
    pub recorded_at: i64,
    /// What changed.
    pub change: ChangeKind,
    /// The node's, edge's or assertion's id.
    pub id: String,
    /// The node's or edge's type.
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub type_id: Option<TypeId>,
    /// The node's name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The edge's source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<NodeId>,
    /// The edge's target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<NodeId>,
    /// The assertion's subject kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_kind: Option<String>,
    /// The assertion's subject's id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    /// The assertion that superseded this one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by: Option<AssertionId>,
    /// The assertion's valid time, in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valid_time: Option<i64>,
    /// The evidence it cites, by id.
    pub evidence: Vec<String>,
}

/// What a change did, in the format's order within a revision.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ChangeKind {
    /// A node was created.
    NodeCreated,
    /// An edge was created.
    EdgeCreated,
    /// An assertion was added.
    AssertionAdded,
    /// An assertion was superseded.
    AssertionSuperseded,
    /// An assertion was retracted.
    AssertionRetracted,
}
