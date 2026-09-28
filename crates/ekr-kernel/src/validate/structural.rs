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
//! Canonical state is one revision, and `DeleteEdge` leaves no trace of the id it removes, so
//! "already holds" does not cover an id an earlier revision held. [`HeldIdentities`] carries that
//! history, and the structural validator of [`Pipeline::identity_keeping`](super::Pipeline) —
//! validation profile v3, `ekr.p3-deterministic/1` — holds node and edge ids against it in one
//! space (`identity-previously-held`). Profiles v1 and v2 do not, and keep answering as they did,
//! so a store that already committed such a reuse replays.
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
use std::sync::Arc;

use ekr_core::{EdgeId, EvidenceId, NodeId, RevisionNumber, TypeId};
use ekr_graph::{CanonicalGraph, GraphSnapshot, ValueSpace};

use super::{finish, issue, node_types, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphOperation, GraphTransaction};

/// A transaction with no operations: `ekr.kernel.GraphTransaction`'s `operation_count >= 1`.
const EMPTY_TRANSACTION: &str = "empty-transaction";

/// One identity, created twice in one transaction.
const DUPLICATE_IDENTITY: &str = "duplicate-identity";

/// An identity created over one canonical state already holds.
const IDENTITY_ALREADY_EXISTS: &str = "identity-already-exists";

/// A node or edge created over an id an earlier revision held and canonical state no longer does.
const IDENTITY_PREVIOUSLY_HELD: &str = "identity-previously-held";

/// One `(type, alias)` taken by two `CreateNode` operations of one transaction.
const DUPLICATE_ALIAS: &str = "duplicate-alias";

/// A `CreateNode` alias a node of the same type already holds in canonical state.
const ALIAS_ALREADY_EXISTS: &str = "alias-already-exists";

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

/// A created node's aliases identify it within its exact type: `ekr_integrate::resolve` matches a
/// typed reference to the nodes of the root whose `type_id` equals the reference's and that hold
/// one of its non-empty aliases, byte for byte. A `CreateNode` that took a `(type, alias)` another
/// node already identifies by would make every reference to either `Ambiguous`, and no operation
/// changes an alias to repair that, so it is refused: against canonical state as
/// `alias-already-exists`, and against another `CreateNode` of the same transaction as
/// `duplicate-alias`. The empty alias identifies nothing and is not checked, and a repeat inside one
/// draft is one alias.
///
/// Transactions only: the seed routes its nodes through this validator with no aliases, because a
/// seed may give two nodes one alias.
fn aliases(graph: &GraphSnapshot<'_>, tx: &GraphTransaction, issues: &mut Vec<ValidationIssue>) {
    let state = graph.graph();
    let mut taken = BTreeSet::new();
    for operation in &tx.operations {
        let GraphOperation::CreateNode(draft) = operation else {
            continue;
        };
        let own: BTreeSet<&str> = draft
            .aliases
            .iter()
            .map(String::as_str)
            .filter(|alias| !alias.is_empty())
            .collect();
        for alias in own {
            if let Some(holder) = state.nodes.values().find(|node| {
                node.root_id == state.root.id
                    && node.type_id == draft.type_id
                    && node.aliases.iter().any(|held| held == alias)
            }) {
                issues.push(issue(
                    tx,
                    ValidatorName::Structural,
                    ALIAS_ALREADY_EXISTS,
                    format!(
                        "node {} is created with alias {alias:?}, which node {} of the same type \
                         {} already holds; resolve the reference and use that node",
                        draft.id, holder.id, draft.type_id
                    ),
                ));
            }
            if !taken.insert((draft.type_id, alias)) {
                issues.push(issue(
                    tx,
                    ValidatorName::Structural,
                    DUPLICATE_ALIAS,
                    format!(
                        "alias {alias:?} of type {} is given to more than one node this \
                         transaction creates; one alias identifies one node of a type",
                        draft.type_id
                    ),
                ));
            }
        }
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
        // Two declarations of one property of one owner compete in an unordered transaction as
        // two writes of one field do, and are refused the same way — identical ones too, since the
        // second has no effect. Two definitions of one type id are already `duplicate-identity`.
        // v2 only: a v1 store's retained rejections replay against the P1 rules unchanged.
        let mut declared = BTreeSet::new();
        for operation in &tx.operations {
            if let GraphOperation::ModifyProperty(modification) = operation {
                if let Some(owner) = modification.owner {
                    if !declared.insert((owner, modification.property.id)) {
                        issues.push(issue(
                            tx,
                            ValidatorName::Structural,
                            CONFLICTING_WRITE,
                            format!(
                                "property {} of type {owner} is declared by more than one \
                                 ModifyProperty in an unordered transaction",
                                modification.property.id
                            ),
                        ));
                    }
                }
            }
        }
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

/// Every node and edge identity some revision of a lineage held.
///
/// The history half of the rule this module opens with. `DeleteEdge` removes an edge from
/// canonical state and keeps no record of its id, so "canonical state already holds it" is true
/// of a deleted edge's id at the revision that deletes it and false ever after, and a later
/// `CreateEdge` could take the id for a different relationship: one `EdgeId` naming two records at
/// two revisions, so that a reference to it, an assertion about it or an explain chain through it
/// can mean the wrong one. The snapshot a validator reads is one revision and cannot answer that,
/// so the lineage's identities are handed in, as the schema lineage is to
/// [`SchemaOntology`](super::schema::SchemaOntology).
///
/// `NodeId` and `EdgeId` are two types over one UUID space — a transaction document writes both
/// as the same text — so the set is keyed by the UUID and remembers which kind held it.
///
/// Each identity also remembers the revision that first held it, so that one set carried forward
/// through a lineage answers for every revision of it: the identities held at revision N are the
/// ones first held at N or before. [`HeldIdentities::at`] reads the set at one revision without
/// copying it, and a replay extends it by the identities each committed revision creates
/// ([`HeldIdentities::hold`]) rather than rescanning every revision's graph.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HeldIdentities {
    /// Each identity, the revision that first held it, and the record it named there.
    ids: Arc<BTreeMap<u128, (RevisionNumber, Record)>>,
    /// The revision the set is read at, or every revision when `None`: an identity first held
    /// after it is not held yet.
    at: Option<RevisionNumber>,
}

/// Which kind of record an identity named.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Held {
    Node,
    Edge,
}

/// The record an identity named, with its typed id.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Record {
    Node(NodeId),
    Edge(EdgeId),
}

impl Record {
    const fn uuid(self) -> u128 {
        match self {
            Self::Node(id) => id.as_u128(),
            Self::Edge(id) => id.as_u128(),
        }
    }

    const fn kind(self) -> Held {
        match self {
            Self::Node(_) => Held::Node,
            Self::Edge(_) => Held::Edge,
        }
    }
}

impl Held {
    const fn name(self) -> &'static str {
        match self {
            Self::Node => "a node",
            Self::Edge => "an edge",
        }
    }
}

impl HeldIdentities {
    /// The identities `graphs` hold between them, each from the earliest of their revisions
    /// that holds it.
    #[must_use]
    pub fn of<'a>(graphs: impl IntoIterator<Item = &'a CanonicalGraph>) -> Self {
        let mut held = Self::default();
        for graph in graphs {
            held.record(
                graph.revision,
                graph.nodes.keys().copied(),
                graph.edges.keys().copied(),
            );
        }
        held
    }

    /// Records the nodes and edges `revision` holds, unless an earlier revision already held them.
    /// An id held as a node and as an edge at one revision is recorded as the node's.
    pub(crate) fn record(
        &mut self,
        revision: RevisionNumber,
        nodes: impl IntoIterator<Item = NodeId>,
        edges: impl IntoIterator<Item = EdgeId>,
    ) {
        let ids = Arc::make_mut(&mut self.ids);
        let found = nodes
            .into_iter()
            .map(Record::Node)
            .chain(edges.into_iter().map(Record::Edge));
        for record in found {
            match ids.get(&record.uuid()) {
                Some((from, _)) if *from <= revision => {}
                _ => {
                    ids.insert(record.uuid(), (revision, record));
                }
            }
        }
    }

    /// Records the nodes and edges `tx` creates as first held at `revision`, the revision that
    /// committing it produces. What a committed transaction creates is everything a revision
    /// holds that its predecessor did not, because applying one removes edges and adds nothing
    /// else.
    pub(crate) fn hold<V: ValueSpace>(
        &mut self,
        revision: RevisionNumber,
        tx: &GraphTransaction<V>,
    ) {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        for operation in &tx.operations {
            match operation {
                GraphOperation::CreateNode(draft) => nodes.push(draft.id),
                GraphOperation::CreateEdge(draft) => edges.push(draft.id),
                _ => {}
            }
        }
        if !nodes.is_empty() || !edges.is_empty() {
            self.record(revision, nodes, edges);
        }
    }

    /// The same identities, read at `revision`: only those first held at it or before.
    #[must_use]
    pub(crate) fn at(&self, revision: RevisionNumber) -> Self {
        Self {
            ids: Arc::clone(&self.ids),
            at: Some(revision),
        }
    }

    /// The kind of record that held `uuid` at the revision this set is read at, if any did.
    fn get(&self, uuid: u128) -> Option<Held> {
        self.ids
            .get(&uuid)
            .filter(|(from, _)| self.at.is_none_or(|at| *from <= at))
            .map(|(_, record)| record.kind())
    }

    /// Every identity of every revision, grouped by the revision that first held it, in revision
    /// order: `(revision, nodes, edges)`.
    pub(crate) fn by_revision(
        &self,
    ) -> BTreeMap<RevisionNumber, (BTreeSet<NodeId>, BTreeSet<EdgeId>)> {
        let mut grouped: BTreeMap<RevisionNumber, (BTreeSet<NodeId>, BTreeSet<EdgeId>)> =
            BTreeMap::new();
        for (from, record) in self.ids.values() {
            let (nodes, edges) = grouped.entry(*from).or_default();
            match record {
                Record::Node(id) => {
                    nodes.insert(*id);
                }
                Record::Edge(id) => {
                    edges.insert(*id);
                }
            }
        }
        grouped
    }
}

/// The structural check of [`Pipeline::identity_keeping`](super::Pipeline::identity_keeping):
/// profile v2's, and node and edge ids held in one space against the whole lineage.
///
/// A separate validator rather than a change to v1's and v2's, because replay re-runs a store's
/// own profile over every retained decision and compares the verdict: a v1 or v2 store that
/// already committed a reuse must keep reopening, so those two keep answering as they did.
pub(crate) struct IdentityStructural {
    /// Every node and edge identity the lineage up to the snapshot held.
    pub(crate) held: HeldIdentities,
}

impl Validator for IdentityStructural {
    fn name(&self) -> ValidatorName {
        ValidatorName::Structural
    }

    fn validate(
        &self,
        graph: &GraphSnapshot<'_>,
        tx: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>> {
        let mut issues = check(graph, tx, true).err().unwrap_or_default();
        once_held(graph, &self.held, tx, &mut issues);
        finish(issues)
    }
}

/// The rule of [`IdentityStructural`] that the per-kind checks in [`check`] do not already state:
/// a node and an edge minted under one id in one transaction (`duplicate-identity`), a node or
/// edge minted over an id canonical state holds as the other kind (`identity-already-exists`),
/// and either minted over an id an earlier revision held and none holds now
/// (`identity-previously-held`). The same-kind cases are [`check`]'s and are not repeated here.
fn once_held(
    graph: &GraphSnapshot<'_>,
    history: &HeldIdentities,
    tx: &GraphTransaction,
    issues: &mut Vec<ValidationIssue>,
) {
    let state = graph.graph();
    let mut minted = BTreeMap::new();
    for operation in &tx.operations {
        let (uuid, kind, named) = match operation {
            GraphOperation::CreateNode(draft) => {
                (draft.id.to_uuid(), Held::Node, format!("node {}", draft.id))
            }
            GraphOperation::CreateEdge(draft) => {
                (draft.id.to_uuid(), Held::Edge, format!("edge {}", draft.id))
            }
            _ => continue,
        };
        match minted.insert(uuid.as_u128(), kind) {
            Some(other) if other != kind => {
                issues.push(issue(
                    tx,
                    ValidatorName::Structural,
                    DUPLICATE_IDENTITY,
                    format!(
                        "{named} is created by one transaction that also creates {} with that \
                         id; node and edge ids share one space",
                        other.name()
                    ),
                ));
                continue;
            }
            // The same kind twice is `duplicate-identity` already.
            Some(_) => continue,
            None => {}
        }
        let current = if state.nodes.contains_key(&NodeId::from_uuid(uuid)) {
            Some(Held::Node)
        } else if state.edges.contains_key(&EdgeId::from_uuid(uuid)) {
            Some(Held::Edge)
        } else {
            None
        };
        match current {
            // `identity-already-exists` already.
            Some(held) if held == kind => {}
            Some(held) => issues.push(issue(
                tx,
                ValidatorName::Structural,
                IDENTITY_ALREADY_EXISTS,
                format!(
                    "{named} is created by this transaction and canonical state already holds {} \
                     with that id; node and edge ids share one space, and an id names one record",
                    held.name()
                ),
            )),
            None => {
                if let Some(held) = history.get(uuid.as_u128()) {
                    issues.push(issue(
                        tx,
                        ValidatorName::Structural,
                        IDENTITY_PREVIOUSLY_HELD,
                        format!(
                            "{named} is created by this transaction and an earlier revision held \
                             {} with that id, which no longer exists; an id names one record for \
                             the life of the store, so a new record takes a new id",
                            held.name()
                        ),
                    ));
                }
            }
        }
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

    aliases(graph, tx, &mut issues);
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
