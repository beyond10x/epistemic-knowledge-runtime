//! Shared projection of the unordered operation set for deterministic validators.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{EdgeId, NodeId, PropertyId, TypeId};
use ekr_graph::GraphSnapshot;

use crate::transaction::{GraphOperation, GraphTransaction};

/// The edge identity and endpoints that survive the operation set.
pub(super) struct Edge {
    pub(super) type_id: TypeId,
    pub(super) source: NodeId,
}

/// Only the indexes validators need, derived by creation first and deletion last.
pub(super) struct Candidate {
    pub(super) nodes: BTreeMap<NodeId, TypeId>,
    pub(super) edges: BTreeMap<EdgeId, Edge>,
    /// Deletion targets may be created and cancelled in the same atomic transaction.
    pub(super) available_edges: BTreeSet<EdgeId>,
    pub(super) property_counts: BTreeMap<NodeId, BTreeMap<PropertyId, usize>>,
}

impl Candidate {
    pub(super) fn of(snapshot: &GraphSnapshot<'_>, proposal: &GraphTransaction) -> Self {
        let graph = snapshot.graph();
        let mut result = Self {
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
            available_edges: BTreeSet::new(),
            property_counts: graph
                .nodes
                .iter()
                .map(|(id, node)| {
                    (
                        *id,
                        node.properties
                            .keys()
                            .map(|property| (*property, 1))
                            .collect(),
                    )
                })
                .collect(),
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
        result.available_edges.extend(result.edges.keys().copied());
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

    pub(super) fn outgoing(&self, source: NodeId, type_id: TypeId) -> BTreeSet<EdgeId> {
        self.edges
            .iter()
            .filter(|(_, edge)| edge.source == source && edge.type_id == type_id)
            .map(|(id, _)| *id)
            .collect()
    }
}
