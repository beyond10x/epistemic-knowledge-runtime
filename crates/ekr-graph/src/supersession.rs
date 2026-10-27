//! A node retired in favour of the node that replaces it: `ekr.graph.SupersededNode`
//! (`story:supersede-node`, design § 110,
//! `architecture-decision-record:0017-a-node-is-superseded-by-another`).
//!
//! A duplicate node — a second node for an entity the store already held — is not deleted and not
//! rewritten. A `SupersedeNode` of a committed transaction moves it, whole, out of the head's
//! nodes, together with every edge that had it at either end, and files it here under its own id,
//! naming the node that replaces it, the revision that retired it, the reason and the evidence. A
//! reader that builds from [`CanonicalGraph::nodes`](crate::CanonicalGraph::nodes) and
//! [`edges`](crate::CanonicalGraph::edges) omits it without a filter of its own; every earlier
//! revision still holds it as it was.
//!
//! A `MergeEntity` absorbs a node the same way, into the same record, and rewrites every reference
//! to it to the node kept instead of retiring its edges (`story:merge-entity-applies`, design
//! § 111, `architecture-decision-record:0018-merge-and-split-identity`): the record then carries
//! the [`MergeLineage`] a split that reverts the merge reads.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{AssertionId, EdgeId, NodeId, PropertyId, RevisionNumber};
use serde::{Deserialize, Serialize};

use crate::assertion::{Object, Subject};
use crate::canonical::ValueSpace;
use crate::edge::Edge;
use crate::node::Node;
use crate::value::CanonicalValue;

/// One superseded node: the node, the edges retired with it, the node it is superseded by, the
/// revision, the reason and the evidence. Filed under the node's id in
/// [`CanonicalGraph::superseded`](crate::CanonicalGraph::superseded).
///
/// Generic over the value space for the reason [`Node`] and [`Edge`] are: canonical state holds
/// the default, whose `by` and `evidence` are canonical references, and a graph document holds
/// the transient instantiation, which carries ids.
///
/// `by` is a node the head holds, never a superseded one: a supersession of the node a record
/// names repoints that record in the same revision, so a reader follows at most one pointer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(
    feature = "schema",
    schemars(bound = "V: schemars::JsonSchema, V::NodeRef: schemars::JsonSchema, \
        V::EdgeRef: schemars::JsonSchema, V::EvidenceRef: schemars::JsonSchema")
)]
#[serde(deny_unknown_fields, bound(deserialize = "V: Deserialize<'de>"))]
pub struct SupersededNode<V: ValueSpace = CanonicalValue> {
    /// The node, exactly as the head held it before the supersession: its root, type, name,
    /// aliases, lifecycle state and properties.
    #[serde(bound(deserialize = "V: Deserialize<'de>"))]
    pub node: Node<V>,
    /// The edges retired with the node, by id, as the head held them: every edge that had it at
    /// either end, except that an edge between two nodes one transaction supersedes is retired
    /// once, into the record of its source, and the record of its target does not hold it
    /// (design § 110.2). For a merge, only the edges it retired: each joining the absorbed node
    /// and the node kept, and each that collapsed onto an edge of the node kept (design § 111.2);
    /// every other edge at the absorbed node stayed in the head, rewritten.
    #[serde(
        deserialize_with = "ekr_core::decode::unique_map",
        bound(deserialize = "V: Deserialize<'de>")
    )]
    pub edges: BTreeMap<EdgeId, Edge<V>>,
    /// The node it is superseded by: one the head holds.
    pub by: V::NodeRef,
    /// The revision whose commit superseded it. A repointed record keeps it.
    pub revision: RevisionNumber,
    /// Why, as the supersession gave it: never empty.
    pub reason: String,
    /// The evidence the supersession rests on: at least one entry canonical state retains.
    #[cfg_attr(feature = "schema", schemars(with = "Vec<V::EvidenceRef>"))]
    #[serde(deserialize_with = "ekr_core::decode::unique_set")]
    pub evidence: BTreeSet<V::EvidenceRef>,
    /// What a merge moved, rewrote and retired, when a `MergeEntity` absorbed the node: `None` for
    /// a node a `SupersedeNode` retired. Absent from the document and from the encoding when
    /// `None`, so a supersession record reads and hashes as it did before the field existed.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        bound(deserialize = "V: Deserialize<'de>")
    )]
    pub merge: Option<MergeLineage<V>>,
}

impl SupersededNode {
    /// The id of the node superseded.
    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.node.id
    }

    /// The edges of the head a merge of the node touched: each rewritten in place, and each an edge
    /// of the node collapsed onto, in id order. None for a node a `SupersedeNode` retired.
    #[must_use]
    pub fn merged_edges(&self) -> BTreeSet<EdgeId> {
        self.merge.as_ref().map_or_else(BTreeSet::new, |lineage| {
            lineage
                .edges
                .keys()
                .chain(lineage.collapsed.values())
                .copied()
                .collect()
        })
    }

    /// The assertions a merge of the node rewrote, in id order. None for a node a `SupersedeNode`
    /// retired.
    #[must_use]
    pub fn merged_assertions(&self) -> BTreeSet<AssertionId> {
        self.merge.as_ref().map_or_else(BTreeSet::new, |lineage| {
            lineage.assertions.keys().copied().collect()
        })
    }
}

impl Canonical for SupersededNode {
    /// The six fields in declaration order, the edges as a map and the evidence as a set; then,
    /// only for a merge, its lineage as a tagged `Some`. A record without one encodes to exactly
    /// the bytes it encoded to before the lineage existed, so no root of a store that never merges
    /// moves, and the `Some` tag cannot be read as the next record's key or what follows the map.
    fn encode(&self, out: &mut Encoder) {
        self.node.encode(out);
        out.map(self.edges.iter());
        self.by.encode(out);
        self.revision.encode(out);
        self.reason.encode(out);
        out.set(self.evidence.iter());
        if let Some(lineage) = &self.merge {
            out.option(Some(lineage));
        }
    }
}

/// Which end of an edge named the node a merge absorbed: `ekr.graph.MergedEnds`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum MergedEnds {
    /// Its source.
    Source,
    /// Its target.
    Target,
    /// Both: an edge from the absorbed node to itself.
    Both,
}

impl Canonical for MergedEnds {
    /// The variant marker: a sum type, tagged.
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::Source => out.variant(0),
            Self::Target => out.variant(1),
            Self::Both => out.variant(2),
        }
    }
}

/// What a merge rewrote in one assertion: each field it rewrote, with what that field held before
/// (`ekr.graph.RewrittenAssertionRecord`). A field the merge left alone is `None`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(
    feature = "schema",
    schemars(bound = "V: schemars::JsonSchema, V::NodeRef: schemars::JsonSchema, \
        V::EdgeRef: schemars::JsonSchema")
)]
#[serde(deny_unknown_fields, bound(deserialize = "V: Deserialize<'de>"))]
pub struct RewrittenAssertion<V: ValueSpace = CanonicalValue> {
    /// The subject before: the absorbed node, or an edge of it that collapsed onto an edge of the
    /// node kept.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<Subject<V::NodeRef, V::EdgeRef>>,
    /// The object before: the absorbed node, or a value holding a `NodeRef` to it at any depth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub object: Option<Object<V, V::NodeRef>>,
}

impl Canonical for RewrittenAssertion {
    /// The two fields in declaration order, each as an option.
    fn encode(&self, out: &mut Encoder) {
        out.option(self.subject.as_ref());
        out.option(self.object.as_ref());
    }
}

/// What a merge moved, rewrote and retired: the lineage a split that reverts the merge reads
/// (`architecture-decision-record:0018-merge-and-split-identity`, decision 5, and
/// `ekr.graph.MergeLineageRecord`). Held in the absorbed node's record, which holds the node and
/// the edges the merge retired, whole.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[cfg_attr(
    feature = "schema",
    schemars(bound = "V: schemars::JsonSchema, V::NodeRef: schemars::JsonSchema, \
        V::EdgeRef: schemars::JsonSchema")
)]
#[serde(deny_unknown_fields)]
pub struct MergeLineage<V: ValueSpace = CanonicalValue> {
    /// (a) Each edge rewritten in place, by id, with the end or ends that named the absorbed node.
    /// It keeps its id, and the head holds it, its ends naming the node kept.
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    pub edges: BTreeMap<EdgeId, MergedEnds>,
    /// (b) Each edge of the absorbed node that collapsed onto an edge the node kept already held,
    /// by id, with that edge's id: same type, direction and other end, and equal properties. The
    /// collapsed edge is held whole in the record's `edges`.
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    pub collapsed: BTreeMap<EdgeId, EdgeId>,
    /// (c) Each edge joining the absorbed node and the node kept, retired: held whole in the
    /// record's `edges`, as a supersession retires an edge.
    #[serde(deserialize_with = "ekr_core::decode::unique_set")]
    pub retired: BTreeSet<EdgeId>,
    /// (d) Each assertion rewritten, by id, with each field rewritten and what it held before.
    #[serde(
        deserialize_with = "ekr_core::decode::unique_map",
        bound(deserialize = "V: Deserialize<'de>")
    )]
    pub assertions: BTreeMap<AssertionId, RewrittenAssertion<V>>,
    /// (e) Each property the absorbed node held and the node kept did not, which moved to the
    /// node kept. The absorbed node keeps every value in the record.
    #[serde(deserialize_with = "ekr_core::decode::unique_set")]
    pub moved_properties: BTreeSet<PropertyId>,
    /// Each property of a node of the head that held a `NodeRef` to the absorbed node, at any
    /// depth, by node and property, with its values before the merge rewrote them.
    #[serde(
        deserialize_with = "ekr_core::decode::unique_map",
        bound(deserialize = "V: Deserialize<'de>")
    )]
    pub node_values: BTreeMap<NodeId, BTreeMap<PropertyId, Vec<V>>>,
    /// The same for each property of an edge of the head, by edge and property.
    #[serde(
        deserialize_with = "ekr_core::decode::unique_map",
        bound(deserialize = "V: Deserialize<'de>")
    )]
    pub edge_values: BTreeMap<EdgeId, BTreeMap<PropertyId, Vec<V>>>,
}

impl<V: ValueSpace> Default for MergeLineage<V> {
    fn default() -> Self {
        Self {
            edges: BTreeMap::new(),
            collapsed: BTreeMap::new(),
            retired: BTreeSet::new(),
            assertions: BTreeMap::new(),
            moved_properties: BTreeSet::new(),
            node_values: BTreeMap::new(),
            edge_values: BTreeMap::new(),
        }
    }
}

impl Canonical for MergeLineage {
    /// The seven fields in declaration order: each map in key order, each set in element order.
    fn encode(&self, out: &mut Encoder) {
        out.map(self.edges.iter());
        out.map(self.collapsed.iter());
        out.set(self.retired.iter());
        out.map(self.assertions.iter());
        out.set(self.moved_properties.iter());
        out.map(self.node_values.iter());
        out.map(self.edge_values.iter());
    }
}

/// Every superseded node of a graph, by the node's id.
pub type Superseded = BTreeMap<NodeId, SupersededNode>;

thread_local! {
    static RECORDS_READ: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Counts one supersession record read by a lookup of a retired edge.
pub(crate) fn read_record() {
    RECORDS_READ.with(|count| count.set(count.get() + 1));
}

/// How many supersession records the calling thread has read to find the record an edge was
/// retired into: one per record [`RetiredEdges::of`] indexes, and one per record
/// [`CanonicalGraph::retired_with`](crate::CanonicalGraph::retired_with) passes on its way.
///
/// Test instrumentation, as `ekr_store::knowledge_roots_hashed` is: it lets a test show that a
/// validation or a read looks retired edges up in one index rather than scanning every record per
/// lookup (`story:supersede-node-review-fixes`, D9). Counted per thread because validation and
/// reads run on their caller's thread, and tests in one binary run on several.
#[doc(hidden)]
#[must_use]
pub fn supersession_records_read() -> u64 {
    RECORDS_READ.with(std::cell::Cell::get)
}

/// Every edge the supersession records of one graph hold, by the edge's id, with the record that
/// holds it: built in one pass over the records, so a reader that looks many retired edges up —
/// a validation, a sample — reads each record once rather than once per lookup
/// (`story:supersede-node-review-fixes`, D9).
///
/// An edge is held by exactly one record: an edge between two nodes one transaction supersedes is
/// retired into its source's record (design § 110.2).
#[derive(Clone, Debug, Default)]
pub struct RetiredEdges<'g>(BTreeMap<EdgeId, &'g SupersededNode>);

impl<'g> RetiredEdges<'g> {
    /// The index of `superseded`'s retired edges. An edge two records held — which no commit
    /// makes — is filed under the first in node-id order, the record
    /// [`CanonicalGraph::retired_with`](crate::CanonicalGraph::retired_with)'s scan finds, so the
    /// two answer alike for every record set.
    #[must_use]
    pub fn of(superseded: &'g Superseded) -> Self {
        let mut index = BTreeMap::new();
        for record in superseded.values() {
            read_record();
            for edge in record.edges.keys() {
                index.entry(*edge).or_insert(record);
            }
        }
        Self(index)
    }

    /// The record `edge` was retired into, if one holds it.
    #[must_use]
    pub fn record(&self, edge: EdgeId) -> Option<&'g SupersededNode> {
        self.0.get(&edge).copied()
    }

    /// The superseded node `edge` was retired with, and the node that one is superseded by, as
    /// [`CanonicalGraph::retired_with`](crate::CanonicalGraph::retired_with) answers.
    #[must_use]
    pub fn retired_with(&self, edge: EdgeId) -> Option<(NodeId, NodeId)> {
        self.record(edge)
            .map(|record| (record.node.id, record.by.node()))
    }

    /// The edge as the record holds it.
    #[must_use]
    pub fn edge(&self, edge: EdgeId) -> Option<&'g Edge> {
        self.record(edge)?.edges.get(&edge)
    }
}
