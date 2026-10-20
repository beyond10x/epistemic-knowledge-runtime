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

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{EdgeId, NodeId, RevisionNumber};
use serde::{Deserialize, Serialize};

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
        V::EvidenceRef: schemars::JsonSchema")
)]
#[serde(deny_unknown_fields)]
pub struct SupersededNode<V: ValueSpace = CanonicalValue> {
    /// The node, exactly as the head held it before the supersession: its root, type, name,
    /// aliases, lifecycle state and properties.
    #[serde(bound(deserialize = "V: Deserialize<'de>"))]
    pub node: Node<V>,
    /// The edges retired with the node, by id, as the head held them: every edge that had it at
    /// either end, except that an edge between two nodes one transaction supersedes is retired
    /// once, into the record of its source, and the record of its target does not hold it
    /// (design § 110.2).
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
}

impl SupersededNode {
    /// The id of the node superseded.
    #[must_use]
    pub const fn id(&self) -> NodeId {
        self.node.id
    }
}

impl Canonical for SupersededNode {
    /// The six fields in declaration order, the edges as a map and the evidence as a set.
    fn encode(&self, out: &mut Encoder) {
        self.node.encode(out);
        out.map(self.edges.iter());
        self.by.encode(out);
        self.revision.encode(out);
        self.reason.encode(out);
        out.set(self.evidence.iter());
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
