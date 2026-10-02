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
//! # Evidence resolves against retained evidence and `AddEvidence`, and against nothing else
//!
//! It used to resolve against the ids the transaction listed as well, and that was the story's
//! fifth acceptance defect passing: the list is evidence **ids**, never an `Evidence`, so
//! "brought by the transaction" could only mean "written down by the proposer" — and a proposer
//! naming a phantom id twice, once on the assertion and once in the list, satisfied the provenance
//! validator with a non-empty set and this one with the list. That is AGENTS.md invariant 4
//! defeated by the agent saying so twice. `tests/adversary_membrane.rs` holds the case.
//!
//! Evidence an assertion cites is therefore in canonical state — seeded, or added by a committed
//! transaction — or it is brought by an `AddEvidence` of the same transaction, which carries the
//! whole entry and the payload its content hash names, held equal by the provenance validator
//! (`decision-blocker:evidence-entry-after-seed`, option 1). Any other id is
//! `unresolved-evidence`. The transaction's own evidence list is still never a source.

use std::collections::BTreeSet;

use ekr_core::{AssertionId, EdgeId, EvidenceId, GraphRootId, NodeId};
use ekr_graph::{CanonicalGraph, GraphSnapshot, Object, Subject};
use ekr_ontology::Value;

use super::candidate::Candidate;
use super::{finish, issue, Check, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphOperation, GraphTransaction};

/// A node the graph does not hold and the transaction does not create.
const UNRESOLVED_NODE: &str = "unresolved-node";

/// An edge the graph does not hold and the transaction does not create.
const UNRESOLVED_EDGE: &str = "unresolved-edge";

/// An assertion the graph does not hold and the transaction does not add.
const UNRESOLVED_ASSERTION: &str = "unresolved-assertion";

/// A piece of evidence canonical state does not retain and no `AddEvidence` of the transaction
/// brings. The message is P1's, unchanged, because retained rejections replay against it.
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
        self.check(graph, tx, &Candidate::of(graph, tx))
    }
}

impl Check for Reference {
    fn check<'g>(
        &self,
        graph: &GraphSnapshot<'g>,
        tx: &GraphTransaction,
        candidate: &Candidate<'g>,
    ) -> Result<(), Vec<ValidationIssue>> {
        let known = Known::of(graph, tx, candidate);
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
                    if !candidate.available(edge) {
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
                        if !known.holds_evidence(evidence) {
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
                    if !known.holds_assertion(&assertion) {
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
                        if !known.holds_assertion(&assertion) {
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
                GraphOperation::AddAlias(addition) => known.node(tx, addition.node, &mut issues),
                // The assertion is looked up in canonical state alone: evidence attaches to an
                // assertion a committed revision holds, and one the same transaction adds would
                // cite the evidence instead (design § 102.2).
                GraphOperation::AttachEvidence(attachment) => {
                    let (assertion, evidence) = (attachment.assertion, attachment.evidence);
                    if !known.state.assertions.contains_key(&assertion) {
                        issues.push(issue(
                            tx,
                            ValidatorName::Reference,
                            UNRESOLVED_ASSERTION,
                            format!(
                                "assertion {assertion} is not in the graph; evidence attaches to \
                                 an assertion a committed revision holds"
                            ),
                        ));
                    }
                    if !known.holds_evidence(&evidence) {
                        issues.push(issue(
                            tx,
                            ValidatorName::Reference,
                            UNRESOLVED_EVIDENCE,
                            format!(
                                "evidence {evidence} attached to assertion {assertion} is not \
                                 retained by canonical state, and no AddEvidence of the \
                                 transaction adds it"
                            ),
                        ));
                    }
                }
                // Names no graph identity but its own, which is new (`Structural`). Its source
                // is a `HumanStatement` or refused by `Provenance`, so it names no assertion.
                GraphOperation::AddEvidence(_)
                | GraphOperation::DefineNodeType(_)
                | GraphOperation::DefineEdgeType(_)
                | GraphOperation::ModifyProperty(_)
                | GraphOperation::WidenEdgeType(_) => {}
            }
        }

        // Retraction retains an assertion and its provenance; it does not erase its references.
        // Only an assertion about an edge the candidate does not hold is refused, so only those
        // edges' assertions are read, and they are refused in the order canonical state holds them.
        let mut unresolved: Vec<(AssertionId, EdgeId)> = candidate
            .asserted_edges()
            .iter()
            .filter(|(edge, _)| !candidate.edges.contains_key(edge))
            .flat_map(|(edge, assertions)| assertions.iter().map(|assertion| (*assertion, *edge)))
            .collect();
        unresolved.sort_unstable();
        for (_, edge) in unresolved {
            known.edge(tx, edge, &mut issues);
        }

        finish(issues)
    }
}

/// Every graph identity that exists once this transaction has been applied: canonical state and
/// the shared candidate, read in place, and the assertions and evidence the transaction adds.
struct Known<'c, 'g> {
    root: GraphRootId,
    state: &'g CanonicalGraph,
    candidate: &'c Candidate<'g>,
    assertions: BTreeSet<AssertionId>,
    evidence: BTreeSet<EvidenceId>,
}

impl<'c, 'g> Known<'c, 'g> {
    /// Identities retained in the shared candidate, plus new assertion and evidence identities.
    ///
    /// Deleted edges are absent. Assertions remain addressable after retraction. Evidence is
    /// retained evidence and what an `AddEvidence` of this transaction brings; the root must
    /// already exist because no operation introduces one.
    fn of(graph: &GraphSnapshot<'g>, tx: &GraphTransaction, candidate: &'c Candidate<'g>) -> Self {
        let state = graph.graph();
        let mut known = Self {
            root: state.root.id,
            state,
            candidate,
            assertions: BTreeSet::new(),
            evidence: BTreeSet::new(),
        };
        for operation in &tx.operations {
            match operation {
                GraphOperation::AddAssertion(assertion) => {
                    known.assertions.insert(assertion.id);
                }
                GraphOperation::AddEvidence(addition) => {
                    known.evidence.insert(addition.evidence.id);
                }
                _ => {}
            }
        }
        known
    }

    /// Whether canonical state holds `assertion` or the transaction adds it.
    fn holds_assertion(&self, assertion: &AssertionId) -> bool {
        self.state.assertions.contains_key(assertion) || self.assertions.contains(assertion)
    }

    /// Whether canonical state retains `evidence` or an `AddEvidence` of the transaction brings it.
    fn holds_evidence(&self, evidence: &EvidenceId) -> bool {
        self.state.evidence.contains_key(evidence) || self.evidence.contains(evidence)
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
        if !self.candidate.nodes.contains_key(&node) {
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
        if !self.candidate.edges.contains_key(&edge) {
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
