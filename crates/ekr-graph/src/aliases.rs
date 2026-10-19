//! A graph's nodes by type and alias: what a typed reference is resolved against.

use std::collections::BTreeMap;

use ekr_core::{NodeId, TypeId};

use crate::canonical::CanonicalGraph;

/// The nodes of one canonical graph's own root, by node type and alias: what a typed reference is
/// resolved against, built once for a graph and then looked up rather than scanned.
///
/// It holds a node under its `type_id` and each of its `aliases`, byte for byte, and only a node
/// whose `root_id` is the graph's root. `canonical_name` is not an alias and is not held
/// (AGENTS.md invariant 3). A function of the graph alone: two equal graphs index equally.
///
/// A superseded node of that root (`story:supersede-node`) is held as the node its record names:
/// its aliases, under its own type, list the node it is superseded by, so a typed reference to
/// the old key finds the node kept, whatever that node's type. Each list holds a node once, in id
/// order, and never a superseded one.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AliasIndex {
    nodes: BTreeMap<TypeId, BTreeMap<String, Vec<NodeId>>>,
}

impl AliasIndex {
    /// Indexes every node of `graph`'s own root, in one pass over its nodes, and every superseded
    /// node of it, in one pass over its supersession records.
    #[must_use]
    pub fn of(graph: &CanonicalGraph) -> Self {
        let mut nodes: BTreeMap<TypeId, BTreeMap<String, Vec<NodeId>>> = BTreeMap::new();
        for node in graph
            .nodes
            .values()
            .filter(|node| node.root_id == graph.root.id)
        {
            let by_alias = nodes.entry(node.type_id).or_default();
            for alias in &node.aliases {
                let held = by_alias.entry(alias.clone()).or_default();
                // Nodes arrive in id order, so each list is in id order and a node's repeated
                // alias is its own last entry.
                if held.last() != Some(&node.id) {
                    held.push(node.id);
                }
            }
        }
        let mut touched = Vec::new();
        for record in graph
            .superseded
            .values()
            .filter(|record| record.node.root_id == graph.root.id)
        {
            let by_alias = nodes.entry(record.node.type_id).or_default();
            for alias in &record.node.aliases {
                by_alias
                    .entry(alias.clone())
                    .or_default()
                    .push(record.by.node());
                touched.push((record.node.type_id, alias));
            }
        }
        // A kept node may arrive out of id order, and more than once: through the node itself
        // and through each node superseded by it.
        for (type_id, alias) in touched {
            if let Some(held) = nodes
                .get_mut(&type_id)
                .and_then(|by_alias| by_alias.get_mut(alias.as_str()))
            {
                held.sort_unstable();
                held.dedup();
            }
        }
        Self { nodes }
    }

    /// The nodes of type `type_id` one of whose aliases is exactly `alias`, in id order.
    #[must_use]
    pub fn nodes(&self, type_id: TypeId, alias: &str) -> &[NodeId] {
        self.nodes
            .get(&type_id)
            .and_then(|by_alias| by_alias.get(alias))
            .map_or(&[], Vec::as_slice)
    }
}
