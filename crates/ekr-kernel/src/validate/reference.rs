//! Validator 2 of design § 20: every identity the proposal names resolves.
//!
//! Design § 6.2, no dangling references, and § 73, which puts "reference existence" in the
//! deterministic half. A reference resolves against canonical state **or** against the same
//! transaction: a proposal that creates a node and then attaches an edge to it is the ordinary
//! shape, and a validator reading only the snapshot would refuse every multi-operation
//! transaction.
//!
//! The transaction is read as a set, not as a sequence. A transaction commits or does not; there
//! is no instant between its operations at which canonical state is half-changed, so "referred to
//! before it was created" is not a state anything can observe.
//!
//! # What resolves where
//!
//! Nodes, edges, assertions, evidence and the **graph root** are graph identities and are resolved
//! here. A `TypeId` is not: the ontology is not the graph, and an operation naming a type the
//! schema does not declare is the type validator's refusal, where the rest of that operation's
//! typing is decided.
//!
//! The enumeration is the contract, and it was short by one: `root_id` is the field that says which
//! graph a node, an edge or an assertion is even in, and nothing asked about it, so a record
//! validated into a root nothing holds. A snapshot has exactly one root in P1, so resolving it is
//! an equality rather than a lookup.
//!
//! # Evidence resolves against retained evidence, and against nothing else
//!
//! It used to resolve against the ids the transaction listed as well, and that was the story's
//! fifth acceptance defect passing: a `GraphTransaction` carries evidence **ids** and never an
//! `Evidence`, and not one of the eleven operations creates one, so "brought by the transaction"
//! could only mean "written down by the proposer" — and a proposer naming a phantom id twice, once
//! on the assertion and once in the list, satisfied the provenance validator with a non-empty set
//! and this one with the list. That is AGENTS.md invariant 4 defeated by the agent saying so
//! twice.
//!
//! P1 has no operation that introduces evidence, so evidence an assertion cites is already in
//! canonical state or it does not exist. `tests/adversary_membrane.rs` holds the case.

use std::collections::BTreeSet;

use ekr_core::{AssertionId, EdgeId, EvidenceId, GraphRootId, NodeId};
use ekr_graph::{GraphSnapshot, Object, Subject};
use ekr_ontology::Value;

use super::candidate::Candidate;
use super::{finish, issue, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphOperation, GraphTransaction};

/// A node the graph does not hold and the transaction does not create.
const UNRESOLVED_NODE: &str = "unresolved-node";

/// An edge the graph does not hold and the transaction does not create.
const UNRESOLVED_EDGE: &str = "unresolved-edge";

/// An assertion the graph does not hold and the transaction does not add.
const UNRESOLVED_ASSERTION: &str = "unresolved-assertion";

/// A piece of evidence canonical state does not retain.
const UNRESOLVED_EVIDENCE: &str = "unresolved-evidence";

/// A graph root that is not the one the snapshot reads.
const UNRESOLVED_GRAPH_ROOT: &str = "unresolved-graph-root";

/// Validator 2: reference existence.
pub struct Reference;

impl Validator for Reference {
    fn name(&self) -> ValidatorName {
        ValidatorName::Reference
    }

    fn validate(
        &self,
        graph: &GraphSnapshot<'_>,
        tx: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>> {
        let candidate = Candidate::of(graph, tx);
        let known = Known::of(graph, tx, &candidate);
        let mut issues = Vec::new();

        for operation in &tx.operations {
            match operation {
                GraphOperation::CreateNode(draft) => {
                    known.root(tx, draft.root_id, "node", &mut issues);
                    for value in draft.properties.values().flatten() {
                        known.value(tx, value, &mut issues);
                    }
                }
                GraphOperation::UpdateProperty(mutation) => {
                    known.node(tx, mutation.node, &mut issues);
                    for value in &mutation.values {
                        known.value(tx, value, &mut issues);
                    }
                }
                GraphOperation::CreateEdge(draft) => {
                    known.root(tx, draft.root_id, "edge", &mut issues);
                    known.node(tx, draft.source, &mut issues);
                    known.node(tx, draft.target, &mut issues);
                    for value in draft.properties.values().flatten() {
                        known.value(tx, value, &mut issues);
                    }
                }
                GraphOperation::DeleteEdge(edge) => {
                    if !candidate.available_edges.contains(edge) {
                        known.edge(tx, *edge, &mut issues);
                    }
                }
                GraphOperation::AddAssertion(assertion) => {
                    known.root(tx, assertion.root_id, "assertion", &mut issues);
                    match assertion.subject {
                        Subject::Node(node) => known.node(tx, node, &mut issues),
                        Subject::Edge(edge) => known.edge(tx, edge, &mut issues),
                        Subject::Type(_) => {}
                    }
                    match &assertion.object {
                        Object::Node(node) => known.node(tx, *node, &mut issues),
                        Object::Value(value) => known.value(tx, value, &mut issues),
                        Object::Type(_) => {}
                    }
                    for evidence in &assertion.evidence {
                        if !known.evidence.contains(evidence) {
                            issues.push(issue(
                                tx,
                                ValidatorName::Reference,
                                UNRESOLVED_EVIDENCE,
                                format!(
                                    "assertion {} cites evidence {evidence}, which canonical \
                                     state does not retain",
                                    assertion.id
                                ),
                            ));
                        }
                    }
                }
                GraphOperation::RetractAssertion(retraction) => {
                    let assertion = retraction.assertion;
                    if !known.assertions.contains(&assertion) {
                        issues.push(issue(
                            tx,
                            ValidatorName::Reference,
                            UNRESOLVED_ASSERTION,
                            format!("assertion {assertion} is not in the graph"),
                        ));
                    }
                }
                GraphOperation::SupersedeAssertion(supersession) => {
                    for assertion in [supersession.assertion, supersession.by] {
                        if !known.assertions.contains(&assertion) {
                            issues.push(issue(
                                tx,
                                ValidatorName::Reference,
                                UNRESOLVED_ASSERTION,
                                format!("assertion {assertion} is not in the graph"),
                            ));
                        }
                    }
                }
                GraphOperation::MergeEntity(merge) => {
                    known.node(tx, merge.absorbed, &mut issues);
                    known.node(tx, merge.into, &mut issues);
                }
                GraphOperation::Invoke {
                    node, arguments, ..
                } => {
                    known.node(tx, *node, &mut issues);
                    for value in arguments.values() {
                        known.value(tx, value, &mut issues);
                    }
                }
                GraphOperation::DefineNodeType(_)
                | GraphOperation::DefineEdgeType(_)
                | GraphOperation::ModifyProperty(_) => {}
            }
        }

        // Retraction retains an assertion and its provenance; it does not erase its references.
        for assertion in graph.graph().assertions.values() {
            if let Subject::Edge(edge) = assertion.subject {
                known.edge(tx, edge, &mut issues);
            }
        }

        finish(issues)
    }
}

/// Every graph identity that exists once this transaction has been applied.
struct Known {
    root: GraphRootId,
    nodes: BTreeSet<NodeId>,
    edges: BTreeSet<EdgeId>,
    assertions: BTreeSet<AssertionId>,
    evidence: BTreeSet<EvidenceId>,
}

impl Known {
    /// Identities retained in the shared candidate, plus new assertion identities.
    ///
    /// Deleted edges are absent. Assertions remain addressable after retraction; evidence and
    /// the root must already exist because no operation introduces them.
    fn of(graph: &GraphSnapshot<'_>, tx: &GraphTransaction, candidate: &Candidate) -> Self {
        let state = graph.graph();
        let mut known = Self {
            root: state.root.id,
            nodes: candidate.nodes.keys().copied().collect(),
            edges: candidate.edges.keys().copied().collect(),
            assertions: state.assertions.keys().copied().collect(),
            evidence: state.evidence.keys().copied().collect(),
        };
        for operation in &tx.operations {
            if let GraphOperation::AddAssertion(assertion) = operation {
                known.assertions.insert(assertion.id);
            }
        }
        known
    }

    /// Refuses a graph root that is not the one the snapshot reads.
    ///
    /// Equality rather than a lookup: design § 22–23 gives a `CanonicalGraph` one `GraphRoot`, and
    /// a snapshot is a read of one graph, so "the roots this transaction may write into" is a set
    /// of one. A record whose `root_id` is any other value is a record in a graph the validating
    /// snapshot cannot see, whether or not that graph exists somewhere else.
    fn root(
        &self,
        tx: &GraphTransaction,
        root_id: GraphRootId,
        what: &str,
        issues: &mut Vec<ValidationIssue>,
    ) {
        if root_id != self.root {
            issues.push(issue(
                tx,
                ValidatorName::Reference,
                UNRESOLVED_GRAPH_ROOT,
                format!(
                    "the {what} names graph root {root_id}, and this snapshot reads {}",
                    self.root
                ),
            ));
        }
    }

    /// Refuses `node` if nothing will hold it.
    fn node(&self, tx: &GraphTransaction, node: NodeId, issues: &mut Vec<ValidationIssue>) {
        if !self.nodes.contains(&node) {
            issues.push(issue(
                tx,
                ValidatorName::Reference,
                UNRESOLVED_NODE,
                format!("node {node} is not in the graph"),
            ));
        }
    }

    /// Refuses `edge` if nothing will hold it.
    fn edge(&self, tx: &GraphTransaction, edge: EdgeId, issues: &mut Vec<ValidationIssue>) {
        if !self.edges.contains(&edge) {
            issues.push(issue(
                tx,
                ValidatorName::Reference,
                UNRESOLVED_EDGE,
                format!("edge {edge} is not in the graph"),
            ));
        }
    }

    /// Refuses every `NodeRef` inside `value` that nothing will hold.
    ///
    /// A reference inside a value is a reference: design § 11.3 gives `NodeRef` its own kind
    /// precisely so that `employer = NodeRef(…)` is checkable where `employer = "OpenAI"` is not,
    /// and a checkable reference that nothing checks is the weaker of the two.
    fn value(&self, tx: &GraphTransaction, value: &Value, issues: &mut Vec<ValidationIssue>) {
        match value {
            Value::NodeRef(node) => self.node(tx, *node, issues),
            Value::List(items) => {
                for item in items {
                    self.value(tx, item, issues);
                }
            }
            Value::Record(fields) => {
                for field in fields.values() {
                    self.value(tx, field, issues);
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
}
