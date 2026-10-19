//! The structural checks of a `SupersedeNode` (`story:supersede-node`, design § 110,
//! `architecture-decision-record:0017-a-node-is-superseded-by-another`): a supersession names two
//! different nodes, gives a reason and rests on evidence, and nothing else in its transaction
//! names the node it retires or an edge retired with it.
//!
//! A node or a `by` canonical state does not hold, one already superseded, and evidence nothing
//! retains or adds are `Reference`'s refusals and are not repeated here. A transaction without a
//! `SupersedeNode` reads nothing here, so it is validated exactly as before the kind existed.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{EdgeId, NodeId};
use ekr_graph::{GraphSnapshot, Object, Subject};

use super::issue;
use crate::transaction::NodeSupersession;
use crate::{GraphOperation, GraphTransaction, ValidationIssue, ValidatorName};

/// A supersession of a node by itself.
const SUPERSEDE_INTO_ITSELF: &str = "supersede-into-itself";

/// A supersession whose reason is empty or only whitespace.
const SUPERSESSION_WITHOUT_REASON: &str = "supersession-without-reason";

/// A supersession that names no evidence.
const SUPERSESSION_WITHOUT_EVIDENCE: &str = "supersession-without-evidence";

/// Another operation of the transaction names the node a supersession retires or one of its
/// edges, the node is superseded twice, or the node it is superseded by is superseded too.
const SUPERSEDED_NODE_NAMED_IN_TRANSACTION: &str = "superseded-node-named-in-transaction";

/// The supersessions of `tx`, in operation order.
pub(crate) fn supersessions(tx: &GraphTransaction) -> impl Iterator<Item = &NodeSupersession> {
    tx.operations
        .iter()
        .filter_map(|operation| match operation {
            GraphOperation::SupersedeNode(supersession) => Some(supersession),
            _ => None,
        })
}

/// The nodes and edges `operation` names, for an operation other than a `SupersedeNode`.
fn named(operation: &GraphOperation) -> (Vec<NodeId>, Vec<EdgeId>) {
    match operation {
        GraphOperation::UpdateProperty(mutation) => (vec![mutation.node], Vec::new()),
        GraphOperation::CreateEdge(draft) => (vec![draft.source, draft.target], Vec::new()),
        GraphOperation::DeleteEdge(edge) => (Vec::new(), vec![*edge]),
        GraphOperation::AddAssertion(assertion) => {
            let mut nodes = Vec::new();
            let mut edges = Vec::new();
            match assertion.subject {
                Subject::Node(node) => nodes.push(node),
                Subject::Edge(edge) => edges.push(edge),
                Subject::Type(_) => {}
            }
            if let Object::Node(node) = assertion.object {
                nodes.push(node);
            }
            (nodes, edges)
        }
        GraphOperation::MergeEntity(merge) => (vec![merge.absorbed, merge.into], Vec::new()),
        GraphOperation::Invoke { node, .. } => (vec![*node], Vec::new()),
        GraphOperation::AddAlias(addition) => (vec![addition.node], Vec::new()),
        GraphOperation::CreateNode(_)
        | GraphOperation::RetractAssertion(_)
        | GraphOperation::SupersedeAssertion(_)
        | GraphOperation::DefineNodeType(_)
        | GraphOperation::DefineEdgeType(_)
        | GraphOperation::ModifyProperty(_)
        | GraphOperation::WidenEdgeType(_)
        | GraphOperation::AddEvidence(_)
        | GraphOperation::AttachEvidence(_)
        | GraphOperation::CiteEvidence(_)
        | GraphOperation::SupersedeNode(_) => (Vec::new(), Vec::new()),
    }
}

/// The issues of `tx`'s supersessions against `snapshot`, in operation order.
pub(super) fn check(snapshot: &GraphSnapshot<'_>, tx: &GraphTransaction) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut retiring: BTreeSet<NodeId> = BTreeSet::new();
    for supersession in supersessions(tx) {
        let (node, by) = (supersession.node, supersession.by);
        if node == by {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                SUPERSEDE_INTO_ITSELF,
                format!(
                    "node {node} is named as both the node superseded and the node it is \
                     superseded by; a supersession retires one node in favour of another"
                ),
            ));
        }
        if supersession.reason.trim().is_empty() {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                SUPERSESSION_WITHOUT_REASON,
                format!(
                    "the supersession of node {node} by node {by} gives no reason; say why the \
                     node is retired"
                ),
            ));
        }
        if supersession.evidence.is_empty() {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                SUPERSESSION_WITHOUT_EVIDENCE,
                format!(
                    "the supersession of node {node} by node {by} names no evidence; a \
                     supersession rests on at least one evidence entry"
                ),
            ));
        }
        if !retiring.insert(node) {
            issues.push(named_in_transaction(
                tx,
                format!(
                    "node {node} is superseded by more than one SupersedeNode of the \
                     transaction; supersede it once"
                ),
            ));
        }
    }
    if retiring.is_empty() {
        return issues;
    }
    for supersession in supersessions(tx) {
        let (node, by) = (supersession.node, supersession.by);
        if by != node && retiring.contains(&by) {
            issues.push(named_in_transaction(
                tx,
                format!(
                    "node {node} is superseded by node {by}, which the same transaction \
                     supersedes; supersede both by the node that is kept"
                ),
            ));
        }
    }
    // The edges a supersession of this transaction retires: one pass over the head's edges.
    let state = snapshot.graph();
    let retired: BTreeMap<EdgeId, NodeId> = state
        .edges
        .values()
        .filter_map(|edge| {
            let (source, target) = (edge.source.node(), edge.target.node());
            if retiring.contains(&source) {
                Some((edge.id, source))
            } else if retiring.contains(&target) {
                Some((edge.id, target))
            } else {
                None
            }
        })
        .collect();
    for operation in &tx.operations {
        let (nodes, edges) = named(operation);
        let node = nodes
            .into_iter()
            .find(|node| retiring.contains(node))
            .or_else(|| edges.iter().find_map(|edge| retired.get(edge).copied()));
        if let Some(node) = node {
            issues.push(named_in_transaction(
                tx,
                format!(
                    "a {} of the transaction names node {node}, or an edge at it, which the same \
                     transaction supersedes; a superseded node is named by no other operation \
                     of its transaction",
                    kind(operation)
                ),
            ));
        }
    }
    issues
}

/// The operation's kind, as its YAML tag names it.
const fn kind(operation: &GraphOperation) -> &'static str {
    match operation {
        GraphOperation::CreateNode(_) => "CreateNode",
        GraphOperation::UpdateProperty(_) => "UpdateProperty",
        GraphOperation::CreateEdge(_) => "CreateEdge",
        GraphOperation::DeleteEdge(_) => "DeleteEdge",
        GraphOperation::AddAssertion(_) => "AddAssertion",
        GraphOperation::RetractAssertion(_) => "RetractAssertion",
        GraphOperation::DefineNodeType(_) => "DefineNodeType",
        GraphOperation::DefineEdgeType(_) => "DefineEdgeType",
        GraphOperation::ModifyProperty(_) => "ModifyProperty",
        GraphOperation::MergeEntity(_) => "MergeEntity",
        GraphOperation::Invoke { .. } => "Invoke",
        GraphOperation::SupersedeAssertion(_) => "SupersedeAssertion",
        GraphOperation::AddEvidence(_) => "AddEvidence",
        GraphOperation::WidenEdgeType(_) => "WidenEdgeType",
        GraphOperation::AddAlias(_) => "AddAlias",
        GraphOperation::AttachEvidence(_) => "AttachEvidence",
        GraphOperation::CiteEvidence(_) => "CiteEvidence",
        GraphOperation::SupersedeNode(_) => "SupersedeNode",
    }
}

fn named_in_transaction(tx: &GraphTransaction, message: String) -> ValidationIssue {
    issue(
        tx,
        ValidatorName::Structural,
        SUPERSEDED_NODE_NAMED_IN_TRANSACTION,
        message,
    )
}
