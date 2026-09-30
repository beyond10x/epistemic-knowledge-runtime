//! Shared projection of the unordered operation set for deterministic validators.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AssertionId, EdgeId, NodeId, PropertyId, TypeId};
use ekr_graph::{CanonicalGraph, GraphSnapshot, Subject};

use crate::transaction::{GraphOperation, GraphTransaction};

/// The edge identity and endpoints that survive the operation set.
pub(super) struct Edge {
    pub(super) type_id: TypeId,
    pub(super) source: NodeId,
}

/// Only the indexes validators need, derived by creation first and deletion last.
pub(super) struct Candidate<'g> {
    /// The canonical state the operation set applies to.
    basis: &'g CanonicalGraph,
    pub(super) nodes: BTreeMap<NodeId, TypeId>,
    pub(super) edges: BTreeMap<EdgeId, Edge>,
    /// The edges the operation set creates; see [`Candidate::available`].
    created_edges: BTreeSet<EdgeId>,
    /// The value count of each property of each node the operation set creates or updates.
    pub(super) property_counts: BTreeMap<NodeId, BTreeMap<PropertyId, usize>>,
    /// Canonical state's assertions about edges, by edge; see [`Candidate::asserted_edges`].
    asserted_edges: OnceCell<BTreeMap<EdgeId, Vec<AssertionId>>>,
}

thread_local! {
    static BUILT: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many candidate views the calling thread has built; see [`super::candidates_built`].
pub(super) fn built() -> u64 {
    BUILT.with(std::cell::Cell::get)
}

impl<'g> Candidate<'g> {
    pub(super) fn of(snapshot: &GraphSnapshot<'g>, proposal: &GraphTransaction) -> Self {
        BUILT.with(|count| count.set(count.get() + 1));
        let graph = snapshot.graph();
        let mut result = Self {
            basis: graph,
            nodes: graph
                .nodes
                .iter()
                .map(|(id, node)| (*id, node.type_id))
                .collect(),
            edges: graph
                .edges
                .iter()
                .map(|(id, edge)| {
                    (
                        *id,
                        Edge {
                            type_id: edge.type_id,
                            source: edge.source.node(),
                        },
                    )
                })
                .collect(),
            created_edges: BTreeSet::new(),
            // Only a created node's counts are ever read, and creating a node replaces whatever
            // canonical state held under its id, so canonical state's counts are not copied.
            property_counts: BTreeMap::new(),
            asserted_edges: OnceCell::new(),
        };
        for operation in &proposal.operations {
            match operation {
                GraphOperation::CreateNode(draft) => {
                    result.nodes.insert(draft.id, draft.type_id);
                    result.property_counts.insert(
                        draft.id,
                        draft
                            .properties
                            .iter()
                            .map(|(id, values)| (*id, values.len()))
                            .collect(),
                    );
                }
                GraphOperation::CreateEdge(draft) => {
                    result.created_edges.insert(draft.id);
                    result.edges.insert(
                        draft.id,
                        Edge {
                            type_id: draft.type_id,
                            source: draft.source,
                        },
                    );
                }
                _ => {}
            }
        }
        for operation in &proposal.operations {
            match operation {
                GraphOperation::DeleteEdge(id) => {
                    result.edges.remove(id);
                }
                GraphOperation::UpdateProperty(mutation) => {
                    result
                        .property_counts
                        .entry(mutation.node)
                        .or_default()
                        .insert(mutation.property, mutation.values.len());
                }
                _ => {}
            }
        }
        result
    }

    /// Whether `edge` is there to delete: canonical state holds it or the operation set creates
    /// it, since a deletion target may be created and cancelled in the same atomic transaction.
    pub(super) fn available(&self, edge: &EdgeId) -> bool {
        self.basis.edges.contains_key(edge) || self.created_edges.contains(edge)
    }

    /// Every assertion canonical state holds about an edge, by that edge, each list in the order
    /// canonical state holds the assertions: built from one pass over the assertions, on first
    /// use, and read by every validator of the validation this view is shared by.
    pub(super) fn asserted_edges(&self) -> &BTreeMap<EdgeId, Vec<AssertionId>> {
        self.asserted_edges.get_or_init(|| {
            let mut index: BTreeMap<EdgeId, Vec<AssertionId>> = BTreeMap::new();
            for (id, assertion) in &self.basis.assertions {
                if let Subject::Edge(edge) = assertion.subject {
                    index.entry(edge.id()).or_default().push(*id);
                }
            }
            index
        })
    }

    /// How many edges of each type leave each source once the operation set applies: one pass
    /// over [`Candidate::edges`], so a validator asking about many created edges reads the graph
    /// once rather than once per edge. An edge id created twice is counted once, under the source
    /// and type of the `CreateEdge` that applied last, and a deleted edge is not counted, exactly
    /// as [`Candidate::outgoing`] lists them.
    pub(super) fn outgoing_counts(&self) -> BTreeMap<(NodeId, TypeId), usize> {
        let mut counts = BTreeMap::new();
        for edge in self.edges.values() {
            *counts.entry((edge.source, edge.type_id)).or_insert(0) += 1;
        }
        counts
    }

    /// The edges of `type_id` that leave `source`, in id order: a whole-graph scan, for the
    /// message of a refusal that has already been decided by [`Candidate::outgoing_counts`].
    pub(super) fn outgoing(&self, source: NodeId, type_id: TypeId) -> BTreeSet<EdgeId> {
        self.edges
            .iter()
            .filter(|(_, edge)| edge.source == source && edge.type_id == type_id)
            .map(|(id, _)| *id)
            .collect()
    }
}
