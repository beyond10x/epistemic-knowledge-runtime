//! Validator 1 of design § 20: the proposal is a well-formed transaction, and every identity it
//! creates is new.
//!
//! `ekr.kernel.Propose` refuses a malformed proposal at the door with `StructurallyInvalid` and
//! states one rule — `operation_count >= 1`. The other three here are the rest of what a
//! transaction can be wrong about before any *value* is looked at.
//!
//! # One identity is created once, and the second half is not the store's question
//!
//! Two `CreateNode` operations over one id inside one transaction are not a conflict with
//! canonical state, so no other validator's question catches them. Neither is a `CreateNode` over
//! an id canonical state **already holds** — and that one was left to whatever a storage engine
//! does with a key it has seen before. It is not left there any more:
//!
//! * AGENTS.md invariant 1 makes this crate the integrity membrane — "only a `ValidatedTransaction`
//!   commits" is worth nothing if what it commits can silently replace a committed record;
//! * AGENTS.md invariant 5 says a committed revision is immutable and a later change is a new
//!   revision, a retraction, a supersession or a migration — never a create;
//! * and there are two store providers behind `ekr-store`. A rule that holds only where the key
//!   index happens to reject a duplicate is a rule that differs between them.
//!
//! So this validator reads two things from the graph: the identities it already holds, and the
//! types its ontology already declares.
//!
//! # The class is five, and it was written here as three
//!
//! The rule is "an operation that brings an identity into existence", and this file said so while
//! covering node, edge and assertion — the three an adversary had named, widened from the two an
//! adversary had named before that. `DefineNodeType` and `DefineEdgeType` each mint a [`TypeId`],
//! which AGENTS.md invariant 3 makes an identity exactly as much as a `NodeId` is, and both sat in
//! a catch-all arm. They are the fourth and fifth, and the arms below are now exhaustive rather
//! than open, so the next operation added to `ekr.kernel.OperationKind` cannot join the class
//! silently: it has to be given an arm, and the arm has to say which side of the rule it is on.
//!
//! The two ids live in **one** space. A `TypeId` the ontology holds as an edge type is not free
//! for a node type: `ekr.ontology.NodeType` and `ekr.ontology.EdgeType` share `TypeId`, and
//! `Ontology::node_type` and `Ontology::edge_type` are two indexes over one kind of identifier.
//!
//! **Only the identity half is here.** Whether a declaration is internally coherent — a `NodeRef`
//! that allows some type, a parent chain without a cycle — is `Ontology::load`'s question over a
//! whole document, and the kernel has no half-applied ontology to ask; `types.rs` says so where it
//! declines the same question.
//!
//! # The evidence set is a manifest, and it must match
//!
//! `GraphTransaction::evidence` is what the transaction declares it rests on, and
//! `ekr.kernel.GraphTransaction.evidence_hash` is the address of exactly that. A declaration
//! nothing compares to the operations is a hash over a number the proposer chose, so the set is
//! held equal to the evidence the transaction's assertions cite — the same shape as the domain's
//! `operation_count`, which is equally derivable and equally declared.

use std::collections::BTreeSet;

use ekr_core::{EvidenceId, TypeId};
use ekr_graph::GraphSnapshot;

use super::{finish, issue, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphOperation, GraphTransaction};

/// A transaction with no operations: `ekr.kernel.GraphTransaction`'s `operation_count >= 1`.
const EMPTY_TRANSACTION: &str = "empty-transaction";

/// One identity, created twice in one transaction.
const DUPLICATE_IDENTITY: &str = "duplicate-identity";

/// An identity created over one canonical state already holds.
const IDENTITY_ALREADY_EXISTS: &str = "identity-already-exists";

/// A transaction whose declared evidence is not the evidence its assertions cite.
const EVIDENCE_SET_MISMATCH: &str = "evidence-set-mismatch";

/// A merge naming one node as both the record absorbed and the record that survives.
const MERGE_INTO_ITSELF: &str = "merge-into-itself";

/// Validator 1: structural validity.
pub struct Structural;

impl Validator for Structural {
    fn name(&self) -> ValidatorName {
        ValidatorName::Structural
    }

    fn validate(
        &self,
        graph: &GraphSnapshot<'_>,
        tx: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        if tx.operations.is_empty() {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                EMPTY_TRANSACTION,
                "a transaction proposes at least one operation".to_owned(),
            ));
        }

        let state = graph.graph();
        let mut nodes = BTreeSet::new();
        let mut edges = BTreeSet::new();
        let mut assertions = BTreeSet::new();
        let mut types = BTreeSet::new();
        let mut cited: BTreeSet<EvidenceId> = BTreeSet::new();
        let declared = |type_id: TypeId| {
            state.ontology.node_type(type_id).is_some()
                || state.ontology.edge_type(type_id).is_some()
        };
        for operation in &tx.operations {
            // Each creation path answers the same two questions about the id it mints: was it
            // minted earlier in this transaction, and does canonical state already hold it. The
            // arms are exhaustive so that an operation added to `ekr.kernel.OperationKind` has to
            // be placed on one side of that rule rather than falling into a catch-all.
            let (created, held) = match operation {
                GraphOperation::CreateNode(draft) => (
                    (!nodes.insert(draft.id)).then(|| format!("node {}", draft.id)),
                    state
                        .nodes
                        .contains_key(&draft.id)
                        .then(|| format!("node {}", draft.id)),
                ),
                GraphOperation::CreateEdge(draft) => (
                    (!edges.insert(draft.id)).then(|| format!("edge {}", draft.id)),
                    state
                        .edges
                        .contains_key(&draft.id)
                        .then(|| format!("edge {}", draft.id)),
                ),
                GraphOperation::AddAssertion(assertion) => {
                    cited.extend(assertion.evidence.iter().copied());
                    (
                        (!assertions.insert(assertion.id))
                            .then(|| format!("assertion {}", assertion.id)),
                        state
                            .assertions
                            .contains_key(&assertion.id)
                            .then(|| format!("assertion {}", assertion.id)),
                    )
                }
                GraphOperation::DefineNodeType(declaration) => (
                    (!types.insert(declaration.id)).then(|| format!("type {}", declaration.id)),
                    declared(declaration.id).then(|| format!("type {}", declaration.id)),
                ),
                GraphOperation::DefineEdgeType(declaration) => (
                    (!types.insert(declaration.id)).then(|| format!("type {}", declaration.id)),
                    declared(declaration.id).then(|| format!("type {}", declaration.id)),
                ),
                GraphOperation::MergeEntity(merge) => {
                    if merge.absorbed == merge.into {
                        issues.push(issue(
                            tx,
                            ValidatorName::Structural,
                            MERGE_INTO_ITSELF,
                            format!(
                                "node {} is named as both the record absorbed and the record that \
                                 keeps its id; a merge of a node into itself decides nothing and \
                                 leaves no alias behind",
                                merge.absorbed
                            ),
                        ));
                    }
                    (None, None)
                }
                // Brings no identity into existence: `ModifyProperty` redeclares a property of a
                // type that already exists, which is what the declare-over-an-existing-id rule
                // above is the *alternative* to. Whether the redeclaration is a compatible one is
                // schema evolution — design § 26, and not P1's.
                GraphOperation::ModifyProperty(_) => (None, None),
                // Name an identity and create none: each is refused by `Reference` if what it
                // names is not there.
                GraphOperation::UpdateProperty(_)
                | GraphOperation::DeleteEdge(_)
                | GraphOperation::RetractAssertion(_)
                | GraphOperation::Invoke { .. } => (None, None),
            };
            if let Some(named) = created {
                issues.push(issue(
                    tx,
                    ValidatorName::Structural,
                    DUPLICATE_IDENTITY,
                    format!("{named} is created twice by one transaction"),
                ));
            }
            if let Some(named) = held {
                issues.push(issue(
                    tx,
                    ValidatorName::Structural,
                    IDENTITY_ALREADY_EXISTS,
                    format!(
                        "{named} is created by this transaction and canonical state already holds \
                         it; a committed record changes through an update, a retraction or a \
                         supersession, never through a create (AGENTS.md invariant 5)"
                    ),
                ));
            }
        }

        if cited != tx.evidence {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                EVIDENCE_SET_MISMATCH,
                format!(
                    "the transaction declares it rests on {:?} and its assertions cite {:?}; the \
                     declared set is what `evidence_hash` addresses and is held to the operations",
                    tx.evidence
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>(),
                    cited.iter().map(ToString::to_string).collect::<Vec<_>>(),
                ),
            ));
        }

        finish(issues)
    }
}
