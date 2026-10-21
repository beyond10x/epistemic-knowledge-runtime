//! The structural checks of a `SupersedeNode` (`story:supersede-node`, design § 110,
//! `architecture-decision-record:0017-a-node-is-superseded-by-another`): a supersession names two
//! different nodes, gives a reason and rests on evidence, and nothing else in its transaction
//! names the node it retires or an edge retired with it.
//!
//! A node or a `by` canonical state does not hold, one already superseded, and evidence nothing
//! retains or adds are `Reference`'s refusals and are not repeated here. A transaction without a
//! `SupersedeNode` reads nothing here, so it is validated exactly as before the kind existed.

use std::collections::BTreeSet;

use ekr_core::{EdgeId, NodeId};
use ekr_graph::{Object, Subject};
use ekr_ontology::Value;

use super::candidate::Candidate;
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

/// Every node a `NodeRef` inside `value` names, at any depth, pushed onto `into`.
fn referenced(value: &Value, into: &mut Vec<NodeId>) {
    match value {
        Value::NodeRef(node) => into.push(*node),
        Value::List(items) => {
            for item in items {
                referenced(item, into);
            }
        }
        Value::Record(fields) => {
            for field in fields.values() {
                referenced(field, into);
            }
        }
        Value::String(_)
        | Value::Boolean(_)
        | Value::Integer(_)
        | Value::Float(_)
        | Value::Decimal(_)
        | Value::Timestamp(_)
        | Value::Duration(_)
        | Value::Enum(_) => {}
    }
}

/// Every node the `NodeRef`s of `values` name.
fn referenced_in<'a>(values: impl IntoIterator<Item = &'a Value>) -> Vec<NodeId> {
    let mut nodes = Vec::new();
    for value in values {
        referenced(value, &mut nodes);
    }
    nodes
}

/// The nodes and edges `operation` names, for an operation other than a `SupersedeNode`: as an
/// endpoint, a target or a subject, and as a `NodeRef` inside any value it writes, which names a
/// node as surely as an edge does (design § 110.3).
///
/// A `MergeEntity` in the shape that applies names none here: it retires the node it absorbs as a
/// `SupersedeNode` retires one, and what it may share a transaction with is
/// [`super::merge`]'s question (design § 111.3). The bare P1 shape names its two nodes, as it did
/// before the other shape existed, so a transaction carrying one is refused in the words it was.
pub(super) fn named(operation: &GraphOperation) -> (Vec<NodeId>, Vec<EdgeId>) {
    match operation {
        GraphOperation::CreateNode(draft) => (
            referenced_in(draft.properties.values().flatten()),
            Vec::new(),
        ),
        GraphOperation::UpdateProperty(mutation) => {
            let mut nodes = vec![mutation.node];
            nodes.extend(referenced_in(&mutation.values));
            (nodes, Vec::new())
        }
        GraphOperation::CreateEdge(draft) => {
            let mut nodes = vec![draft.source, draft.target];
            nodes.extend(referenced_in(draft.properties.values().flatten()));
            (nodes, Vec::new())
        }
        GraphOperation::DeleteEdge(edge) => (Vec::new(), vec![*edge]),
        GraphOperation::AddAssertion(assertion) => {
            let mut nodes = Vec::new();
            let mut edges = Vec::new();
            match assertion.subject {
                Subject::Node(node) => nodes.push(node),
                Subject::Edge(edge) => edges.push(edge),
                Subject::Type(_) => {}
            }
            match &assertion.object {
                Object::Node(node) => nodes.push(*node),
                Object::Value(value) => referenced(value, &mut nodes),
                Object::Type(_) => {}
            }
            (nodes, edges)
        }
        GraphOperation::MergeEntity(merge) if merge.is_bare() => {
            (vec![merge.absorbed, merge.into], Vec::new())
        }
        GraphOperation::MergeEntity(_) => (Vec::new(), Vec::new()),
        GraphOperation::Invoke {
            node, arguments, ..
        } => {
            let mut nodes = vec![*node];
            nodes.extend(referenced_in(arguments.values()));
            (nodes, Vec::new())
        }
        GraphOperation::AddAlias(addition) => (vec![addition.node], Vec::new()),
        GraphOperation::RetractAssertion(_)
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

/// The issues of `tx`'s supersessions against the candidate view of `tx`, in operation order.
///
/// What a supersession retires in its transaction is what `candidate` holds as retiring: under
/// profiles v1 to v3 every `SupersedeNode`'s node and the edges at it, as released; under v4 only
/// those of the supersessions `Reference` admits (`story:supersession-verdict-fixes-replay-safe`,
/// D7), so a supersession `Reference` refuses — its node not held or already superseded, its `by`
/// unknown or superseded — retires nothing, and an operation naming its node is refused for what
/// it is itself and not also here.
pub(super) fn check(tx: &GraphTransaction, candidate: &Candidate<'_>) -> Vec<ValidationIssue> {
    let mut issues = Vec::new();
    let mut superseded: BTreeSet<NodeId> = BTreeSet::new();
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
        if !superseded.insert(node) {
            issues.push(named_in_transaction(
                tx,
                format!(
                    "node {node} is superseded by more than one SupersedeNode of the \
                     transaction; supersede it once"
                ),
            ));
        }
    }
    let retiring: BTreeSet<NodeId> = candidate.retiring().collect();
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
    // The edges a supersession of this transaction retires are the candidate's: one pass over
    // the head's edges, made as the view was built.
    for operation in &tx.operations {
        let (nodes, edges) = named(operation);
        let node = nodes
            .into_iter()
            .find(|node| retiring.contains(node))
            .or_else(|| {
                edges
                    .iter()
                    .find_map(|edge| candidate.retired_by_transaction(edge))
            });
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
pub(super) const fn kind(operation: &GraphOperation) -> &'static str {
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

pub(super) fn named_in_transaction(tx: &GraphTransaction, message: String) -> ValidationIssue {
    issue(
        tx,
        ValidatorName::Structural,
        SUPERSEDED_NODE_NAMED_IN_TRANSACTION,
        message,
    )
}
