//! Shared projection of the unordered operation set for deterministic validators.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

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
    pub(super) nodes: Nodes<'g>,
    pub(super) edges: BTreeMap<EdgeId, Edge>,
    /// The edges the operation set creates; see [`Candidate::available`].
    created_edges: BTreeSet<EdgeId>,
    /// The value count of each property of each node the operation set creates or updates.
    pub(super) property_counts: BTreeMap<NodeId, BTreeMap<PropertyId, usize>>,
    /// Canonical state's assertions about edges, by edge; see [`Candidate::asserted_edges`].
    asserted_edges: Index<'g>,
    pub(super) aliases: Option<&'g super::aliases::AliasHolders>,
}

/// Existing node types stay in the verified graph; only creations belong to this candidate.
pub(super) struct Nodes<'g> {
    basis: &'g BTreeMap<NodeId, ekr_graph::Node>,
    created: BTreeMap<NodeId, TypeId>,
}

impl Nodes<'_> {
    pub(super) fn get(&self, id: &NodeId) -> Option<&TypeId> {
        self.created
            .get(id)
            .or_else(|| self.basis.get(id).map(|node| &node.type_id))
    }

    pub(super) fn contains_key(&self, id: &NodeId) -> bool {
        self.get(id).is_some()
    }

    fn insert(&mut self, id: NodeId, type_id: TypeId) {
        self.created.insert(id, type_id);
    }
}

impl ekr_ontology::NodeTypes for Nodes<'_> {
    fn type_of(&self, node: NodeId) -> Option<TypeId> {
        self.get(&node).copied()
    }
}

/// Canonical state's assertions about edges, by edge, each list in the order canonical state holds
/// the assertions.
pub(crate) type AssertedEdges = BTreeMap<EdgeId, Vec<AssertionId>>;

/// Where the [`AssertedEdges`] of one graph is kept with that graph, so that every validation
/// against it reads one index: built by the first that reads it.
pub(crate) type AssertedEdgesCell = Arc<OnceLock<AssertedEdges>>;

/// Where a candidate view finds the [`AssertedEdges`] of the graph it applies to.
enum Index<'g> {
    /// Its own, built for this view alone.
    Own(OnceCell<AssertedEdges>),
    /// The one kept with the graph.
    Kept(&'g OnceLock<AssertedEdges>),
}

thread_local! {
    static BUILT: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    static INDEXED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many candidate views the calling thread has built; see [`super::candidates_built`].
pub(super) fn built() -> u64 {
    BUILT.with(std::cell::Cell::get)
}

/// How many per-edge assertion indexes the calling thread has built; see
/// [`super::edge_indexes_built`].
pub(super) fn indexed() -> u64 {
    INDEXED.with(std::cell::Cell::get)
}

impl<'g> Candidate<'g> {
    pub(super) fn of(snapshot: &GraphSnapshot<'g>, proposal: &GraphTransaction) -> Self {
        BUILT.with(|count| count.set(count.get() + 1));
        let graph = snapshot.graph();
        let mut result = Self {
            basis: graph,
            nodes: Nodes {
                basis: &graph.nodes,
                created: BTreeMap::new(),
            },
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
            asserted_edges: Index::Own(OnceCell::new()),
            aliases: None,
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

    /// [`Candidate::of`], reading the [`AssertedEdges`] kept with the snapshot's graph, `kept`,
    /// rather than building its own. `kept` must be the one kept with that graph.
    pub(super) fn kept(
        snapshot: &GraphSnapshot<'g>,
        proposal: &GraphTransaction,
        kept: &'g OnceLock<AssertedEdges>,
        aliases: &'g super::aliases::AliasHolders,
    ) -> Self {
        Self {
            asserted_edges: Index::Kept(kept),
            aliases: Some(aliases),
            ..Self::of(snapshot, proposal)
        }
    }

    /// Whether `edge` is there to delete: canonical state holds it or the operation set creates
    /// it, since a deletion target may be created and cancelled in the same atomic transaction.
    pub(super) fn available(&self, edge: &EdgeId) -> bool {
        self.basis.edges.contains_key(edge) || self.created_edges.contains(edge)
    }

    /// Every assertion canonical state holds about an edge, by that edge, each list in the order
    /// canonical state holds the assertions: built from one pass over the assertions, on first
    /// use, and read by every validator of the validation this view is shared by — and, where the
    /// index is kept with the graph ([`Candidate::kept`]), by every later validation against it.
    pub(super) fn asserted_edges(&self) -> &AssertedEdges {
        let build = || {
            INDEXED.with(|count| count.set(count.get() + 1));
            let mut index = AssertedEdges::new();
            for (id, assertion) in &self.basis.assertions {
                if let Subject::Edge(edge) = assertion.subject {
                    index.entry(edge.id()).or_default().push(*id);
                }
            }
            index
        };
        match &self.asserted_edges {
            Index::Own(index) => index.get_or_init(build),
            Index::Kept(index) => index.get_or_init(build),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_candidate_borrows_existing_node_types_at_every_graph_size() {
        let mut copies = Vec::new();
        for size in [4, 16] {
            let mut seed = crate::SeedDocument::from_yaml(include_str!(
                "../../tests/fixtures/seed-minimal-v2.yaml"
            ))
            .unwrap();
            let type_id = TypeId::mint();
            seed.ontology
                .node_types
                .push(ekr_ontology::NodeType::new(type_id, "Subject"));
            let mut graph = CanonicalGraph {
                root: seed.graph.root,
                revision: seed.graph.revision,
                ontology: ekr_ontology::Ontology::load(seed.ontology).unwrap(),
                nodes: BTreeMap::new(),
                edges: BTreeMap::new(),
                assertions: BTreeMap::new(),
                attachments: BTreeMap::new(),
                evidence: BTreeMap::new(),
            };
            for at in 0..size {
                let id = NodeId::mint();
                graph.nodes.insert(
                    id,
                    ekr_graph::Node::new(id, graph.root.id, type_id, format!("subject {at}")),
                );
            }
            let created = NodeId::mint();
            let transaction = GraphTransaction {
                id: ekr_core::TransactionId::mint(),
                proposer: ekr_core::AgentId::mint(),
                operations: vec![GraphOperation::CreateNode(crate::NodeDraft {
                    id: created,
                    root_id: graph.root.id,
                    type_id,
                    canonical_name: "created".into(),
                    properties: BTreeMap::new(),
                    aliases: Vec::new(),
                })],
                evidence: BTreeSet::new(),
                schema_version: None,
            };
            let view = Candidate::of(&GraphSnapshot::of(&graph), &transaction);
            assert_eq!(view.nodes.get(&created), Some(&type_id));
            assert_eq!(view.nodes.get(&NodeId::mint()), None);
            copies.push(
                graph
                    .nodes
                    .iter()
                    .filter(|(id, node)| {
                        let observed = view.nodes.get(id).unwrap();
                        assert_eq!(*observed, node.type_id);
                        !std::ptr::eq(observed, &node.type_id)
                    })
                    .count(),
            );
        }
        assert_eq!(
            copies,
            [0, 0],
            "unchanged node types copied into one validation candidate"
        );
    }
}
