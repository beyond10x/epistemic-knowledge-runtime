//! A private, revision-bound lookup for structural alias refusals. Public captured indexes are
//! immutable; this scratch index may move only between graphs whose exact coordinates it knows.
use std::collections::BTreeMap;

use ekr_core::{NodeId, RevisionId, TypeId};
use ekr_graph::{CanonicalGraph, Node, Root};

use crate::{GraphOperation, GraphTransaction};

#[derive(Debug, Default)]
pub(crate) struct AliasHolders(BTreeMap<TypeId, BTreeMap<String, NodeId>>);

impl AliasHolders {
    pub(crate) fn get(&self, type_id: TypeId, alias: &str) -> Option<NodeId> {
        self.0.get(&type_id)?.get(alias).copied()
    }

    fn add(&mut self, node: &Node) {
        for alias in &node.aliases {
            self.0
                .entry(node.type_id)
                .or_default()
                .entry(alias.clone())
                .and_modify(|held| *held = (*held).min(node.id))
                .or_insert(node.id);
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct AliasCache {
    at: Option<(RevisionId, Root)>,
    holders: AliasHolders,
}

impl AliasCache {
    /// A cold, historical or diverging read rebuilds from its own verified graph. Coordinates
    /// include the complete roots and revision identity, never just a revision number.
    pub(crate) fn at(
        &mut self,
        id: RevisionId,
        root: Root,
        graph: &CanonicalGraph,
    ) -> &AliasHolders {
        if self.at != Some((id, root)) {
            let mut holders = AliasHolders::default();
            for node in graph.nodes.values() {
                #[cfg(test)]
                super::ALIAS_NODES_VISITED.with(|count| count.set(count.get() + 1));
                if node.root_id == graph.root.id {
                    holders.add(node);
                }
            }
            self.holders = holders;
            self.at = Some((id, root));
        }
        &self.holders
    }

    /// Advance only from the exact indexed basis of a verified commit. No operation removes a
    /// node or alias; creations and additions are the only changes to this lookup. Enumerating
    /// every operation makes a future alias-changing operation require an explicit decision.
    pub(crate) fn advance(
        &mut self,
        prior: (RevisionId, Root),
        next: (RevisionId, Root),
        graph: &CanonicalGraph,
        tx: &GraphTransaction<ekr_graph::CanonicalValue>,
    ) {
        if self.at != Some(prior) {
            return;
        }
        for operation in &tx.operations {
            let changed = match operation {
                GraphOperation::CreateNode(node) => Some(node.id),
                GraphOperation::AddAlias(addition) => Some(addition.node),
                GraphOperation::UpdateProperty(_)
                | GraphOperation::CreateEdge(_)
                | GraphOperation::DeleteEdge(_)
                | GraphOperation::AddAssertion(_)
                | GraphOperation::RetractAssertion(_)
                | GraphOperation::DefineNodeType(_)
                | GraphOperation::DefineEdgeType(_)
                | GraphOperation::ModifyProperty(_)
                | GraphOperation::MergeEntity(_)
                | GraphOperation::Invoke { .. }
                | GraphOperation::SupersedeAssertion(_)
                | GraphOperation::AddEvidence(_)
                | GraphOperation::WidenEdgeType(_) => None,
            };
            if let Some(node) = changed.and_then(|id| graph.nodes.get(&id)) {
                if node.root_id == graph.root.id {
                    self.holders.add(node);
                }
            }
        }
        self.at = Some(next);
    }
}
