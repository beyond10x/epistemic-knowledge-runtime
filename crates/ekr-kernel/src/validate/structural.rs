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

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{EvidenceId, TypeId};
use ekr_graph::GraphSnapshot;

use super::{finish, issue, node_types, Validator};
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

/// An unordered operation set proposes competing writes to the same field.
const CONFLICTING_WRITE: &str = "conflicting-write";

/// P1 has no application semantics for schema evolution or entity integration.
const UNSUPPORTED_OPERATION: &str = "unsupported-operation";

/// Validator 1: structural validity.
pub struct Structural;

/// A schema-changing transaction carries no `schema_version`.
const SCHEMA_VERSION_MISSING: &str = "schema-version-missing";

/// A transaction carries a `schema_version` and changes no schema.
const SCHEMA_VERSION_WITHOUT_CHANGE: &str = "schema-version-without-schema-change";

/// A schema change proposed together with an operation of another kind.
const MIXED_SCHEMA_TRANSACTION: &str = "mixed-schema-transaction";

/// A `ModifyProperty` in the ownerless P1 shape, which profile v2 cannot place in the ontology.
const MODIFY_PROPERTY_WITHOUT_OWNER: &str = "modify-property-without-owner";

impl Validator for Structural {
    fn name(&self) -> ValidatorName {
        ValidatorName::Structural
    }

    /// Validation profile v1's structural check: the three schema kinds are refused as P1 refused
    /// them, byte for byte, so every rejection a v1 store retains still replays.
    fn validate(
        &self,
        graph: &GraphSnapshot<'_>,
        tx: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>> {
        check(graph, tx, false)
    }
}

/// Validation profile v2's structural check: the three schema kinds are admitted, as schema-only
/// transactions that name the version they produce. Reports as [`ValidatorName::Structural`].
pub(crate) struct SchemaStructural;

impl Validator for SchemaStructural {
    fn name(&self) -> ValidatorName {
        ValidatorName::Structural
    }

    fn validate(
        &self,
        graph: &GraphSnapshot<'_>,
        tx: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>> {
        check(graph, tx, true)
    }
}

/// The version field and the schema-only rule (wave p5-01, decisions 1 and 3).
///
/// Under v1 only the first rule applies — a version on a transaction that changes no schema —
/// because no transaction v1 ever retained carries a version: a v1 schema change without one is
/// refused exactly as P1 refused it and by nothing else, so its retained rejection replays.
fn schema_shape(tx: &GraphTransaction, admits_schema: bool, issues: &mut Vec<ValidationIssue>) {
    let changes = tx
        .operations
        .iter()
        .filter(|operation| super::schema::is_schema_change(operation))
        .count();
    match (tx.schema_version, changes) {
        (Some(version), 0) => issues.push(issue(
            tx,
            ValidatorName::Structural,
            SCHEMA_VERSION_WITHOUT_CHANGE,
            format!(
                "the transaction names schema version {version} and none of its operations is \
                 DefineNodeType, DefineEdgeType or ModifyProperty; only a schema change produces \
                 a schema version"
            ),
        )),
        (None, 1..) if admits_schema => issues.push(issue(
            tx,
            ValidatorName::Structural,
            SCHEMA_VERSION_MISSING,
            "the transaction changes the schema and names no schema_version; mint one with \
             `ekr mint schema-version`"
                .to_owned(),
        )),
        _ => {}
    }
    if admits_schema {
        for operation in &tx.operations {
            if let GraphOperation::ModifyProperty(modification) = operation {
                if modification.owner.is_none() {
                    issues.push(issue(
                        tx,
                        ValidatorName::Structural,
                        MODIFY_PROPERTY_WITHOUT_OWNER,
                        format!(
                            "ModifyProperty of property {} names no owner; profile v2 files a \
                             property under the type that declares it: write \
                             `{{owner: <TypeId>, property: {{...}}}}`",
                            modification.property.id
                        ),
                    ));
                }
            }
        }
    }
    if admits_schema && changes > 0 && changes < tx.operations.len() {
        issues.push(issue(
            tx,
            ValidatorName::Structural,
            MIXED_SCHEMA_TRANSACTION,
            format!(
                "{changes} of the transaction's {} operations change the schema; a schema change \
                 is proposed in a transaction of its own",
                tx.operations.len()
            ),
        ));
    }
}

/// The structural check, under v2 when `admits_schema` and under v1 otherwise.
fn check(
    graph: &GraphSnapshot<'_>,
    tx: &GraphTransaction,
    admits_schema: bool,
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
        state.ontology.node_type(type_id).is_some() || state.ontology.edge_type(type_id).is_some()
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
            // the ontology-constraint validator's question under profile v2 (design § 26).
            GraphOperation::ModifyProperty(_) => (None, None),
            // Name an identity and create none: each is refused by `Reference` if what it
            // names is not there.
            GraphOperation::UpdateProperty(_)
            | GraphOperation::DeleteEdge(_)
            | GraphOperation::RetractAssertion(_)
            | GraphOperation::SupersedeAssertion(_)
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

    schema_shape(tx, admits_schema, &mut issues);

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

    let node_types = node_types(graph, tx);
    let mut properties = BTreeMap::new();
    let mut lifecycle_writes = BTreeSet::new();
    for operation in &tx.operations {
        let writes = match operation {
            GraphOperation::CreateNode(draft) => draft
                .properties
                .iter()
                .map(|(property, values)| ((draft.id, *property), values))
                .collect::<Vec<_>>(),
            GraphOperation::UpdateProperty(mutation) => {
                vec![((mutation.node, mutation.property), &mutation.values)]
            }
            GraphOperation::Invoke {
                node, operation, ..
            } => {
                let moves = node_types
                    .get(node)
                    .and_then(|id| state.ontology.node_type(*id))
                    .and_then(|declared| declared.operations.get(operation))
                    .is_some_and(|declared| declared.transition.is_some());
                if moves && !lifecycle_writes.insert(*node) {
                    issues.push(issue(tx, ValidatorName::Structural, CONFLICTING_WRITE,
                        format!("multiple operations move node {node}'s lifecycle in an unordered transaction")));
                }
                Vec::new()
            }
            _ => Vec::new(),
        };
        for ((node, property), values) in writes {
            if properties
                .insert((node, property), values)
                .is_some_and(|previous| previous != values)
            {
                issues.push(issue(tx, ValidatorName::Structural, CONFLICTING_WRITE,
                    format!("node {node} property {property} has competing values in an unordered transaction")));
            }
        }
    }

    // Keep malformed-operation diagnostics precise. A structurally sound operation is not
    // thereby implemented: entity merge has no application path, and under v1 neither has schema
    // evolution. The v1 message is P1's, unchanged, because retained v1 rejections replay
    // against it.
    if issues.is_empty() {
        for operation in &tx.operations {
            let name = match operation {
                GraphOperation::DefineNodeType(_) if !admits_schema => "DefineNodeType",
                GraphOperation::DefineEdgeType(_) if !admits_schema => "DefineEdgeType",
                GraphOperation::ModifyProperty(_) if !admits_schema => "ModifyProperty",
                GraphOperation::MergeEntity(merge)
                    if node_types.contains_key(&merge.absorbed)
                        && node_types.contains_key(&merge.into) =>
                {
                    "MergeEntity"
                }
                _ => continue,
            };
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                UNSUPPORTED_OPERATION,
                format!("{name} is not supported in P1"),
            ));
        }
    }

    issues.extend(super::lifecycle::check(graph, tx));
    finish(issues)
}
