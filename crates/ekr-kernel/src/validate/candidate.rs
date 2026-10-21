//! Shared projection of the unordered operation set for deterministic validators.

use std::cell::OnceCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

use ekr_core::{AssertionId, EdgeId, NodeId, PropertyId, TypeId};
use ekr_graph::{CanonicalGraph, GraphSnapshot, Subject};

use super::Supersessions;
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
    /// Which supersession verdicts this validation gives; see [`Candidate::exact`].
    supersessions: Supersessions,
    /// The nodes the operation set's supersessions retire, each with the node it is superseded
    /// by: every `SupersedeNode` of it under [`Supersessions::Released`], and only those
    /// `Reference` admits ([`admits`]) under [`Supersessions::Exact`]. See
    /// [`Candidate::retiring`] and [`Candidate::superseding`].
    superseding: BTreeMap<NodeId, NodeId>,
    /// The edges of canonical state a supersession of [`Candidate::retiring`] retires, each with
    /// the node it is retired with; see [`Candidate::retires`].
    retired_edges: BTreeMap<EdgeId, NodeId>,
    /// The edges canonical state's supersession records hold, indexed on first use; see
    /// [`Candidate::retired_with`].
    retired_index: OnceCell<ekr_graph::RetiredEdges<'g>>,
    /// The value count of each property of each node the operation set creates or updates.
    pub(super) property_counts: BTreeMap<NodeId, BTreeMap<PropertyId, usize>>,
    /// Canonical state's assertions about edges, by edge; see [`Candidate::asserted_edges`].
    asserted_edges: Index<'g>,
    pub(super) aliases: Option<&'g super::aliases::AliasHolders>,
    /// What each merge of the operation set would do, read on first use; see
    /// [`Candidate::merge_checks`].
    merges: OnceCell<Vec<super::merge::MergeCheck>>,
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

/// Whether `Reference` admits `supersession` against `graph`, `nodes` being the nodes held once
/// the operation set's creations apply: its node is one canonical state holds and has not
/// superseded, and its `by` is the node itself — `supersede-into-itself` is `Structural`'s — or
/// one held or created and not superseded. Exactly the supersessions `Reference` raises none of
/// `supersede-unknown-node`, `node-already-superseded`, `supersede-by-unknown-node` and
/// `supersede-by-superseded-node` for under profile v4; only those retire anything in their
/// transaction there (`story:supersession-verdict-fixes-replay-safe`, D7). The evidence is not
/// read: a supersession resting on unknown evidence still names the node it would retire.
fn admits(
    graph: &CanonicalGraph,
    nodes: &Nodes<'_>,
    supersession: &crate::transaction::NodeSupersession,
) -> bool {
    let (node, by) = (supersession.node, supersession.by);
    graph.superseded_by(node).is_none()
        && graph.nodes.contains_key(&node)
        && (by == node || (graph.superseded_by(by).is_none() && nodes.contains_key(&by)))
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
    /// The candidate view of a validation giving the supersession verdicts profiles v1 to v3
    /// released: a validator's own [`Validator::validate`](super::Validator::validate) builds this.
    pub(super) fn of(snapshot: &GraphSnapshot<'g>, proposal: &GraphTransaction) -> Self {
        Self::under(snapshot, proposal, Supersessions::Released)
    }

    /// The candidate view of a validation giving `supersessions`' verdicts.
    pub(super) fn under(
        snapshot: &GraphSnapshot<'g>,
        proposal: &GraphTransaction,
        supersessions: Supersessions,
    ) -> Self {
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
            supersessions,
            superseding: BTreeMap::new(),
            retired_edges: BTreeMap::new(),
            retired_index: OnceCell::new(),
            // Only a created node's counts are ever read, and creating a node replaces whatever
            // canonical state held under its id, so canonical state's counts are not copied.
            property_counts: BTreeMap::new(),
            asserted_edges: Index::Own(OnceCell::new()),
            aliases: None,
            merges: OnceCell::new(),
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
        // A supersession retires every edge of canonical state at its node (design § 110.2):
        // one pass over the edges, and only for a transaction that supersedes. An edge between
        // two nodes the operation set supersedes is retired with its source, as commit retires
        // it. The node itself stays, so that an operation naming it is refused once, by the
        // structural check that owns that question (`superseded-node-named-in-transaction`).
        // Under profile v4 a supersession `Reference` refuses retires nothing
        // (`story:supersession-verdict-fixes-replay-safe`, D7): its node is absent or already
        // retired, or the supersession does not happen as written, and an operation naming that
        // node is refused for what it is itself. Under v1 to v3 every one retires, as released.
        for operation in &proposal.operations {
            if let GraphOperation::SupersedeNode(supersession) = operation {
                if supersessions == Supersessions::Released
                    || admits(graph, &result.nodes, supersession)
                {
                    result
                        .superseding
                        .insert(supersession.node, supersession.by);
                }
            }
        }
        if !result.superseding.is_empty() {
            for edge in graph.edges.values() {
                let (source, target) = (edge.source.node(), edge.target.node());
                let node = if result.superseding.contains_key(&source) {
                    source
                } else if result.superseding.contains_key(&target) {
                    target
                } else {
                    continue;
                };
                result.retired_edges.insert(edge.id, node);
                result.edges.remove(&edge.id);
            }
        }
        result
    }

    /// Whether this validation gives profile v4's exact supersession verdicts
    /// ([`Supersessions::Exact`]) rather than those profiles v1 to v3 released.
    pub(super) fn exact(&self) -> bool {
        self.supersessions == Supersessions::Exact
    }

    /// Whether a supersession of [`Candidate::retiring`] retires `edge`, an edge of canonical
    /// state.
    pub(super) fn retires(&self, edge: &EdgeId) -> bool {
        self.retired_edges.contains_key(edge)
    }

    /// The node a supersession of [`Candidate::retiring`] retires `edge` with, `edge` being an
    /// edge of canonical state at it: its source when both its ends are retiring.
    pub(super) fn retired_by_transaction(&self, edge: &EdgeId) -> Option<NodeId> {
        self.retired_edges.get(edge).copied()
    }

    /// The node a supersession of [`Candidate::retiring`] supersedes `node` by, if one supersedes
    /// it: what the head will name instead of `node` once the transaction commits. Read for the
    /// messages of profile v4 only.
    pub(super) fn superseding(&self, node: NodeId) -> Option<NodeId> {
        self.superseding.get(&node).copied()
    }

    /// The nodes the operation set's supersessions retire in their transaction: every
    /// `SupersedeNode`'s under profiles v1 to v3, and those `Reference` admits under v4.
    pub(super) fn retiring(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.superseding.keys().copied()
    }

    /// What each merge of `proposal` in the shape that applies would do, read off `snapshot` once
    /// for the validation this view is shared by, by the first validator that asks (design
    /// § 111.3). Empty, having read nothing, for an operation set without such a merge.
    /// `snapshot` and `proposal` must be the ones this view was built from.
    pub(super) fn merge_checks(
        &self,
        snapshot: &GraphSnapshot<'_>,
        proposal: &GraphTransaction,
    ) -> &[super::merge::MergeCheck] {
        self.merges
            .get_or_init(|| super::merge::checks(snapshot, proposal, self))
    }

    /// The superseded node `edge` was retired with, and the node that one is superseded by, if a
    /// supersession record of canonical state holds the edge: the answer
    /// [`CanonicalGraph::retired_with`] gives, looked up in one index of the records built by the
    /// first lookup of this validation, so a validation reads each record once however many
    /// edges it looks up (`story:supersede-node-review-fixes`, D9). A graph with no record builds
    /// none.
    pub(super) fn retired_with(&self, edge: EdgeId) -> Option<(NodeId, NodeId)> {
        if self.basis.superseded.is_empty() {
            return None;
        }
        self.retired_index
            .get_or_init(|| self.basis.retired_edges())
            .retired_with(edge)
    }

    /// [`Candidate::under`], reading the [`AssertedEdges`] kept with the snapshot's graph, `kept`,
    /// rather than building its own. `kept` must be the one kept with that graph.
    pub(super) fn kept(
        snapshot: &GraphSnapshot<'g>,
        proposal: &GraphTransaction,
        kept: &'g OnceLock<AssertedEdges>,
        aliases: &'g super::aliases::AliasHolders,
        supersessions: Supersessions,
    ) -> Self {
        Self {
            asserted_edges: Index::Kept(kept),
            aliases: Some(aliases),
            ..Self::under(snapshot, proposal, supersessions)
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
                superseded: BTreeMap::new(),
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
