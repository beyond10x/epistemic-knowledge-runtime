//! The checks of a `MergeEntity` in the shape that applies (`story:merge-entity-applies`,
//! `architecture-decision-record:0018-merge-and-split-identity`, decision 6): a merge gives a
//! reason and rests on evidence; nothing else in its transaction names the node it absorbs or an
//! edge at it; the two nodes' types are one or related through `parents`; and the graph the merge
//! makes is one a transaction could have written — every property the absorbed node holds declared
//! by the node kept's type and free of conflict with it, every duplicate edge equal to the one it
//! collapses onto, every rewritten edge end and active relation assertion of an allowed type, no
//! active relation assertion joining the two nodes (it would relate the node kept to itself;
//! design § 111.3 names that pair only), every rewritten
//! `NodeRef` value admitted by its declaration, and every count within its cardinality once the
//! duplicates have collapsed.
//!
//! Each check is raised by the validator whose question it is, and reads one [`MergeCheck`]
//! built once per validation by [`checks`], from the snapshot and the transaction's other
//! operations, through the rules [`crate::merge`] applies at commit. A merge whose nodes do not
//! resolve, or that absorbs a node into itself, has no [`MergeCheck`]: `Reference` and
//! `Structural` refuse it, and the checks here say nothing more. A merge across unrelated types
//! is refused for that alone.
//!
//! A transaction without a merge in that shape reads nothing here, and the bare
//! `{absorbed, into}` shape never does, so every retained rejection replays as it was refused.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AssertionId, EdgeId, NodeId, PropertyId, TypeId};
use ekr_graph::{
    AssertionLifecycle, CanonicalGraph, CanonicalValue, GraphSnapshot, MergedEnds, Object,
    Predicate, Subject,
};
use ekr_ontology::{CheckReason, NodeTypes, Ontology, PropertyDefinition, Value};

use super::candidate::Candidate;
use super::issue;
use crate::merge::{self, EdgeAt, Fate};
use crate::transaction::{EntityMerge, GraphOperation, GraphTransaction};
use crate::{ValidationIssue, ValidatorName};

/// A merge without a reason, or one that is only whitespace.
const MERGE_WITHOUT_REASON: &str = "merge-without-reason";

/// A merge that names no evidence.
const MERGE_WITHOUT_EVIDENCE: &str = "merge-without-evidence";

/// A merge of two nodes neither of whose types conforms to the other's.
const MERGE_TYPES_UNRELATED: &str = "merge-types-unrelated";

/// A property the absorbed node holds, or an active assertion gives it, that the node kept's type
/// does not declare.
const MERGE_UNDECLARED_PROPERTY: &str = "merge-undeclared-property";

/// A property both nodes hold with different values.
const MERGE_PROPERTY_CONFLICT: &str = "merge-property-conflict";

/// An edge of the absorbed node duplicating one the node kept holds, with different properties.
const MERGE_DUPLICATE_EDGE_CONFLICT: &str = "merge-duplicate-edge-conflict";

/// An active relation assertion joining the absorbed node and the node kept, either way, which the
/// merge would make a relation of the node kept to itself.
const MERGE_SELF_RELATION: &str = "merge-self-relation";

/// A rewritten edge end or relation assertion end of a type the edge type does not allow: the
/// code `Type` raises for a `CreateEdge`.
const EDGE_ENDPOINT_TYPE: &str = "edge-endpoint-type";

/// A rewritten or moved value its declaration does not admit: the code `Type` raises for a
/// written value.
const WRONG_TYPE: &str = "wrong-type";

/// More edges of one type out of the node kept than the edge type permits: the code
/// `Cardinality` raises for a `CreateEdge`.
const EDGE_CARDINALITY: &str = "edge-cardinality";

/// More values of a moved property than its declaration on the node kept's type permits.
const PROPERTY_CARDINALITY: &str = "property-cardinality";

/// One active assertion a merge rewrites: what it says once rewritten.
struct Rewritten {
    id: AssertionId,
    predicate: Predicate,
    /// The subject afterwards: a node or an edge, by id; `None` for a type.
    subject: Option<Subject<NodeId, EdgeId>>,
    /// The object afterwards.
    object: Object<CanonicalValue>,
    /// Whether the subject was the absorbed node.
    subject_moved: bool,
    /// Whether the object named the absorbed node.
    object_moved: bool,
}

/// What one merge would do, read off the snapshot and the transaction's other operations.
pub(super) struct MergeCheck {
    absorbed: NodeId,
    into: NodeId,
    absorbed_type: TypeId,
    into_type: TypeId,
    /// Whether one type conforms to the other. When not, nothing below is filled in.
    related: bool,
    /// Each edge of the absorbed node: its fate, its type, and its ends once rewritten.
    edges: BTreeMap<EdgeId, (Fate, TypeId, NodeId, NodeId)>,
    /// The node kept's outgoing edges once the merge applies, by type, for each type the merge
    /// gives it one more of.
    outgoing: BTreeMap<TypeId, BTreeSet<EdgeId>>,
    /// The properties the absorbed node holds that the node kept's type does not declare.
    undeclared: BTreeSet<PropertyId>,
    /// The properties both hold with different values.
    conflicts: BTreeSet<PropertyId>,
    /// The properties that move, with their values once rewritten.
    moved: BTreeMap<PropertyId, Vec<CanonicalValue>>,
    /// Each property of a node of the head that the merge rewrites: the node, its type, the
    /// property and its values afterwards.
    node_values: Vec<(NodeId, TypeId, PropertyId, Vec<CanonicalValue>)>,
    /// The same for the head's edges, with the edge's type.
    edge_values: Vec<(EdgeId, TypeId, PropertyId, Vec<CanonicalValue>)>,
    /// The active relation assertions joining the absorbed node and the node kept, either way.
    self_relations: Vec<AssertionId>,
    /// The other active assertions the merge rewrites.
    assertions: Vec<Rewritten>,
}

/// A transaction's values as canonical state would hold them, or `None` where one is not
/// admissible: `Types` refuses that value, and a merge check of it would only repeat the refusal.
fn admitted(values: &[Value]) -> Option<Vec<CanonicalValue>> {
    values
        .iter()
        .map(|value| CanonicalValue::try_from(value.clone()).ok())
        .collect()
}

/// One [`MergeCheck`] for each merge of `tx` in the shape that applies whose absorbed node the
/// head holds and whose node kept the head holds or `tx` creates; empty, reading nothing, for a
/// transaction without one.
pub(super) fn checks(
    snapshot: &GraphSnapshot<'_>,
    tx: &GraphTransaction,
    candidate: &Candidate<'_>,
) -> Vec<MergeCheck> {
    let merges: Vec<&EntityMerge> = merge::shaped(tx).collect();
    // More than one merge is refused for that alone (`same_transaction`). Checking each would read
    // the head once per merge and keep, then report, what each would rewrite: a document of k
    // merges would cost k passes and up to k issues per edge for one verdict.
    let [merge] = merges.as_slice() else {
        return Vec::new();
    };
    check_one(snapshot.graph(), tx, candidate, merge)
        .into_iter()
        .collect()
}

#[allow(clippy::too_many_lines)]
fn check_one(
    state: &CanonicalGraph,
    tx: &GraphTransaction,
    candidate: &Candidate<'_>,
    merge: &EntityMerge,
) -> Option<MergeCheck> {
    let (absorbed, into) = (merge.absorbed, merge.into);
    if absorbed == into {
        return None;
    }
    let node = state.nodes.get(&absorbed)?;
    let into_type = *candidate.nodes.get(&into)?;
    let ontology = &state.ontology;
    let absorbed_type = node.type_id;
    let mut check = MergeCheck {
        absorbed,
        into,
        absorbed_type,
        into_type,
        related: ontology.conforms_to(absorbed_type, into_type)
            || ontology.conforms_to(into_type, absorbed_type),
        edges: BTreeMap::new(),
        outgoing: BTreeMap::new(),
        undeclared: BTreeSet::new(),
        conflicts: BTreeSet::new(),
        moved: BTreeMap::new(),
        node_values: Vec::new(),
        edge_values: Vec::new(),
        self_relations: Vec::new(),
        assertions: Vec::new(),
    };
    if !check.related {
        return Some(check);
    }

    // What the transaction's other operations leave of the head, as commit applies them before
    // the merge: deletions, supersessions, updates and creations. The supersessions are those
    // that retire anything in the transaction, as the edges below read them (`gone`): every
    // `SupersedeNode` under profiles v1 to v3, and under v4 only those `Reference` admits, so a
    // merge beside a refused supersession is judged as the merge alone
    // (`story:supersession-verdict-fixes-replay-safe`, D7).
    let mut deleted = BTreeSet::new();
    let superseding: BTreeSet<NodeId> = candidate.retiring().collect();
    let mut updated = BTreeSet::new();
    let mut created: Option<&crate::NodeDraft> = None;
    let mut near: Vec<EdgeAt<'_>> = Vec::new();
    for operation in &tx.operations {
        match operation {
            GraphOperation::DeleteEdge(edge) => {
                deleted.insert(*edge);
            }
            GraphOperation::UpdateProperty(mutation) => {
                updated.insert((mutation.node, mutation.property));
            }
            GraphOperation::CreateNode(draft) if draft.id == into => created = Some(draft),
            GraphOperation::CreateEdge(draft)
                if (draft.source == into || draft.target == into)
                    && draft.source != absorbed
                    && draft.target != absorbed =>
            {
                let properties = draft
                    .properties
                    .iter()
                    .filter(|(_, values)| !values.is_empty())
                    .map(|(property, values)| admitted(values).map(|held| (*property, held)))
                    .collect::<Option<BTreeMap<_, _>>>();
                if let Some(properties) = properties {
                    near.push(EdgeAt {
                        id: draft.id,
                        type_id: draft.type_id,
                        source: draft.source,
                        target: draft.target,
                        properties: Cow::Owned(properties),
                    });
                }
            }
            _ => {}
        }
    }
    let gone = |edge: EdgeId| deleted.contains(&edge) || candidate.retires(&edge);
    // An edge the transaction creates and deletes again is gone before the merge applies, as a
    // held edge it deletes is: nothing collapses onto it, and it counts toward no cardinality.
    near.retain(|edge| !gone(edge.id));
    near.extend(
        state
            .edges
            .values()
            .filter(|edge| !gone(edge.id))
            .map(EdgeAt::of)
            .filter(|edge| edge.touches(absorbed) || edge.touches(into)),
    );

    // The edges.
    let fates = merge::fates(absorbed, into, &near);
    let end = |at: NodeId| if at == absorbed { into } else { at };
    let mut added = BTreeSet::new();
    for edge in &near {
        if edge.source == into && !edge.touches(absorbed) {
            check
                .outgoing
                .entry(edge.type_id)
                .or_default()
                .insert(edge.id);
        }
        if let Some(fate) = fates.get(&edge.id) {
            check.edges.insert(
                edge.id,
                (*fate, edge.type_id, end(edge.source), end(edge.target)),
            );
            if matches!(fate, Fate::Rewritten(MergedEnds::Source | MergedEnds::Both)) {
                check
                    .outgoing
                    .entry(edge.type_id)
                    .or_default()
                    .insert(edge.id);
                added.insert(edge.type_id);
            }
        }
    }
    check.outgoing.retain(|type_id, _| added.contains(type_id));

    // The properties: the node kept's as the transaction leaves them, its own `NodeRef` values to
    // the absorbed node rewritten, against the absorbed node's.
    let mut kept: BTreeMap<PropertyId, Option<Vec<CanonicalValue>>> = match created {
        Some(draft) => draft
            .properties
            .iter()
            .filter(|(_, values)| !values.is_empty())
            .map(|(property, values)| (*property, admitted(values)))
            .collect(),
        None => state.nodes.get(&into).map_or_else(BTreeMap::new, |held| {
            held.properties
                .iter()
                .map(|(property, values)| (*property, Some(values.clone())))
                .collect()
        }),
    };
    for operation in &tx.operations {
        if let GraphOperation::UpdateProperty(mutation) = operation {
            if mutation.node == into {
                if mutation.values.is_empty() {
                    kept.remove(&mutation.property);
                } else {
                    kept.insert(mutation.property, admitted(&mutation.values));
                }
            }
        }
    }
    let declared = ontology.properties_of(into_type);
    for (property, values) in &node.properties {
        if !declared.contains_key(property) {
            check.undeclared.insert(*property);
            continue;
        }
        let mine = merge::rewritten(values, absorbed, into).unwrap_or_else(|| values.clone());
        match kept.get(property) {
            None => {
                check.moved.insert(*property, mine);
            }
            Some(None) => {}
            Some(Some(theirs)) => {
                let theirs = merge::rewritten(theirs, absorbed, into)
                    .map_or(Cow::Borrowed(theirs), Cow::Owned);
                if *theirs != mine {
                    check.conflicts.insert(*property);
                }
            }
        }
    }

    // `NodeRef` values in the head's nodes and edges, as the transaction leaves them: a value it
    // writes is its own, and one naming the absorbed node is refused by name elsewhere.
    for (id, held) in &state.nodes {
        if *id == absorbed || superseding.contains(id) {
            continue;
        }
        for (property, values) in &held.properties {
            if updated.contains(&(*id, *property)) {
                continue;
            }
            if let Some(new) = merge::rewritten(values, absorbed, into) {
                check.node_values.push((*id, held.type_id, *property, new));
            }
        }
    }
    for edge in state.edges.values() {
        if gone(edge.id)
            || matches!(
                fates.get(&edge.id),
                Some(Fate::Retired | Fate::Collapsed(_) | Fate::Conflicting(_))
            )
        {
            continue;
        }
        for (property, values) in &edge.properties {
            if let Some(new) = merge::rewritten(values, absorbed, into) {
                check
                    .edge_values
                    .push((edge.id, edge.type_id, *property, new));
            }
        }
    }

    // Active assertions naming the absorbed node: what each says once rewritten.
    for assertion in state.assertions.values() {
        if !matches!(assertion.lifecycle, AssertionLifecycle::Active) {
            continue;
        }
        let subject_moved =
            matches!(assertion.subject, Subject::Node(held) if held.node() == absorbed);
        let object_moved = match &assertion.object {
            Object::Node(held) => held.node() == absorbed,
            Object::Value(value) => merge::names(value, absorbed),
            Object::Type(_) => false,
        };
        if !(subject_moved || object_moved) {
            continue;
        }
        // A relation joining the two, either way, would relate the node kept to itself: refused
        // by name, and held to nothing else here, so the one defect draws one issue.
        if let (Predicate::Relation(_), Subject::Node(from), Object::Node(to)) =
            (assertion.predicate, assertion.subject, &assertion.object)
        {
            let (from, to) = (from.node(), to.node());
            if (from == absorbed && to == into) || (from == into && to == absorbed) {
                check.self_relations.push(assertion.id);
                continue;
            }
        }
        check.assertions.push(Rewritten {
            id: assertion.id,
            predicate: assertion.predicate,
            subject: match assertion.subject {
                Subject::Node(held) => Some(Subject::Node(end(held.node()))),
                Subject::Edge(held) => Some(Subject::Edge(held.id())),
                Subject::Type(_) => None,
            },
            object: match &assertion.object {
                Object::Node(held) if held.node() == absorbed => {
                    Object::Node(ekr_graph::CanonicalRef::new(into))
                }
                Object::Value(value) => Object::Value(merge::rewrite(value, absorbed, into)),
                other => other.clone(),
            },
            subject_moved,
            object_moved,
        });
    }
    Some(check)
}

/// `Structural`'s checks of a merge in the shape that applies: its reason and evidence; nothing
/// else in its transaction naming the absorbed node or an edge at it; one merge per transaction;
/// the conflicts the merge would have to choose between; and the active relation assertions it
/// would make relate the node kept to itself.
pub(super) fn structural(
    snapshot: &GraphSnapshot<'_>,
    tx: &GraphTransaction,
    candidate: &Candidate<'_>,
) -> Vec<ValidationIssue> {
    let merges: Vec<&EntityMerge> = merge::shaped(tx).collect();
    if merges.is_empty() {
        return Vec::new();
    }
    let mut issues = Vec::new();
    for merge in &merges {
        let (absorbed, into) = (merge.absorbed, merge.into);
        if merge
            .reason
            .as_deref()
            .is_none_or(|reason| reason.trim().is_empty())
        {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                MERGE_WITHOUT_REASON,
                format!(
                    "the merge of node {absorbed} into node {into} gives no reason; say why the \
                     two are one"
                ),
            ));
        }
        if merge.evidence.as_ref().is_none_or(BTreeSet::is_empty) {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                MERGE_WITHOUT_EVIDENCE,
                format!(
                    "the merge of node {absorbed} into node {into} names no evidence; a merge \
                     rests on at least one evidence entry"
                ),
            ));
        }
    }
    same_transaction(snapshot, tx, &merges, &mut issues);
    for check in candidate.merge_checks(snapshot, tx) {
        if !check.related {
            continue;
        }
        let (absorbed, into) = (check.absorbed, check.into);
        if !check.conflicts.is_empty() {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                MERGE_PROPERTY_CONFLICT,
                format!(
                    "node {absorbed} and node {into} hold different values of {}; a merge keeps \
                     one value of a property: make them equal, or clear it on one node, in an \
                     earlier revision",
                    listed(check.conflicts.iter())
                ),
            ));
        }
        for (edge, (fate, _, source, target)) in &check.edges {
            if let Fate::Conflicting(other) = fate {
                issues.push(issue(
                    tx,
                    ValidatorName::Structural,
                    MERGE_DUPLICATE_EDGE_CONFLICT,
                    format!(
                        "edge {edge} of node {absorbed} would run from node {source} to node \
                         {target}, as edge {other} of node {into} does, with different \
                         properties; a merge collapses the two into one: make their properties \
                         equal, or delete one, in an earlier revision"
                    ),
                ));
            }
        }
        for assertion in &check.self_relations {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                MERGE_SELF_RELATION,
                format!(
                    "assertion {assertion} relates node {absorbed} and node {into}, which the \
                     merge makes one node, so it would relate node {into} to itself; retract the \
                     assertion in an earlier revision, then merge"
                ),
            ));
        }
    }
    issues
}

/// The same-transaction refusals of ADR 0018 decision 6, raised as `SupersedeNode`'s are
/// (design § 110.3): another operation naming the absorbed node or an edge at it; the absorbed
/// node or the node kept retired by another operation; a supersession by the absorbed node; an
/// edge at the absorbed node joining a node the transaction supersedes; and more than one merge.
fn same_transaction(
    snapshot: &GraphSnapshot<'_>,
    tx: &GraphTransaction,
    merges: &[&EntityMerge],
    issues: &mut Vec<ValidationIssue>,
) {
    let named = |message: String| super::supersession::named_in_transaction(tx, message);
    if merges.len() > 1 {
        issues.push(named(format!(
            "the transaction carries {} MergeEntity operations; a merge rewrites every reference \
             to the node it absorbs, so each merge is proposed in a transaction of its own",
            merges.len()
        )));
    }
    let superseding: BTreeMap<NodeId, NodeId> = super::supersession::supersessions(tx)
        .map(|supersession| (supersession.node, supersession.by))
        .collect();
    let absorbing: BTreeSet<NodeId> = merges.iter().map(|merge| merge.absorbed).collect();
    for merge in merges {
        let (absorbed, into) = (merge.absorbed, merge.into);
        if superseding.contains_key(&absorbed) {
            issues.push(named(format!(
                "node {absorbed} is merged into node {into} and superseded by the same \
                 transaction; retire it once"
            )));
        }
        if into != absorbed && superseding.contains_key(&into) {
            issues.push(named(format!(
                "node {absorbed} is merged into node {into}, which the same transaction \
                 supersedes; merge it into the node that is kept"
            )));
        }
    }
    for (node, by) in &superseding {
        if absorbing.contains(by) {
            issues.push(named(format!(
                "node {node} is superseded by node {by}, which the same transaction merges into \
                 another; supersede it by the node that is kept"
            )));
        }
    }
    // The edges at an absorbed node, each with the node at its other end: one pass over the
    // head's edges.
    let state = snapshot.graph();
    let at_absorbed: BTreeMap<EdgeId, (NodeId, NodeId)> = state
        .edges
        .values()
        .filter_map(|edge| {
            let (source, target) = (edge.source.node(), edge.target.node());
            if absorbing.contains(&source) {
                Some((edge.id, (source, target)))
            } else if absorbing.contains(&target) {
                Some((edge.id, (target, source)))
            } else {
                None
            }
        })
        .collect();
    let into_of: BTreeMap<NodeId, NodeId> = merges
        .iter()
        .map(|merge| (merge.absorbed, merge.into))
        .collect();
    for (edge, (absorbed, other)) in &at_absorbed {
        if *other != *absorbed
            && into_of.get(absorbed) != Some(other)
            && superseding.contains_key(other)
        {
            issues.push(named(format!(
                "edge {edge} of node {absorbed}, which a MergeEntity of the transaction rewrites, \
                 has node {other} at its other end, which the same transaction supersedes; \
                 supersede or merge in a transaction of its own"
            )));
        }
    }
    for operation in &tx.operations {
        if matches!(operation, GraphOperation::MergeEntity(merge) if !merge.is_bare()) {
            continue;
        }
        let (nodes, edges) = super::supersession::named(operation);
        let node = nodes
            .into_iter()
            .find(|node| absorbing.contains(node))
            .or_else(|| {
                edges
                    .iter()
                    .find_map(|edge| at_absorbed.get(edge).map(|(absorbed, _)| *absorbed))
            });
        if let Some(node) = node {
            issues.push(named(format!(
                "a {} of the transaction names node {node}, or an edge at it, which a MergeEntity \
                 of the same transaction absorbs; a merged node is named by no other operation \
                 of its transaction",
                super::supersession::kind(operation)
            )));
        }
    }
}

/// `Type`'s checks of a merge in the shape that applies.
pub(super) fn types(
    snapshot: &GraphSnapshot<'_>,
    tx: &GraphTransaction,
    candidate: &Candidate<'_>,
) -> Vec<ValidationIssue> {
    let checks = candidate.merge_checks(snapshot, tx);
    if checks.is_empty() {
        return Vec::new();
    }
    let ontology = &snapshot.graph().ontology;
    let nodes = &candidate.nodes;
    let mut issues = Vec::new();
    for check in checks {
        let (absorbed, into, into_type) = (check.absorbed, check.into, check.into_type);
        if !check.related {
            issues.push(issue(
                tx,
                ValidatorName::Type,
                MERGE_TYPES_UNRELATED,
                format!(
                    "node {absorbed} is a {} and node {into} a {into_type}, and neither type \
                     conforms to the other through parents; a merge joins nodes of one type or \
                     of related types. Retire node {absorbed} in favour of node {into} with \
                     SupersedeNode instead",
                    check.absorbed_type
                ),
            ));
            continue;
        }
        if !check.undeclared.is_empty() {
            issues.push(issue(
                tx,
                ValidatorName::Type,
                MERGE_UNDECLARED_PROPERTY,
                format!(
                    "node {absorbed} holds {}, which type {into_type} of node {into} does not \
                     declare; clear it with an UpdateProperty of no values in an earlier \
                     revision, then merge",
                    listed(check.undeclared.iter())
                ),
            ));
        }
        let merged = format!("once node {absorbed} is merged into node {into}");
        // Each rewritten edge end, held to its edge type as a CreateEdge's is.
        for (edge, (fate, type_id, source, target)) in &check.edges {
            let Fate::Rewritten(ends) = fate else {
                continue;
            };
            let Some(declared) = ontology.edge_type(*type_id) else {
                continue;
            };
            let rewritten = [
                (
                    "source",
                    *source,
                    &declared.source_types,
                    matches!(ends, MergedEnds::Source | MergedEnds::Both),
                ),
                (
                    "target",
                    *target,
                    &declared.target_types,
                    matches!(ends, MergedEnds::Target | MergedEnds::Both),
                ),
            ];
            for (end, node, allowed, moved) in rewritten {
                if moved && !allows(ontology, allowed, into_type) {
                    issues.push(issue(
                        tx,
                        ValidatorName::Type,
                        EDGE_ENDPOINT_TYPE,
                        format!(
                            "edge {edge} would have {end} {node}, a {into_type}, {merged}, which \
                             is not an allowed {end} type of {}; delete the edge in an earlier \
                             revision, then merge",
                            declared.name
                        ),
                    ));
                }
            }
        }
        for assertion in &check.assertions {
            assertion_types(
                ontology,
                nodes,
                candidate,
                tx,
                check,
                assertion,
                &merged,
                &mut issues,
            );
        }
        // Each rewritten and each moved value, against the declaration that holds it.
        let properties = ontology.properties_of(into_type);
        for (node, type_id, property, values) in &check.node_values {
            let holder = if *node == into { into_type } else { *type_id };
            if let Some(definition) = ontology.properties_of(holder).get(property) {
                values_admitted(
                    ontology,
                    nodes,
                    definition,
                    values,
                    &format!("property {property} of node {node}, {merged}"),
                    tx,
                    &mut issues,
                );
            }
        }
        for (edge, type_id, property, values) in &check.edge_values {
            if let Some(definition) = ontology
                .edge_type(*type_id)
                .and_then(|declared| declared.properties.get(property))
            {
                values_admitted(
                    ontology,
                    nodes,
                    definition,
                    values,
                    &format!("property {property} of edge {edge}, {merged}"),
                    tx,
                    &mut issues,
                );
            }
        }
        for (property, values) in &check.moved {
            if let Some(definition) = properties.get(property) {
                values_admitted(
                    ontology,
                    nodes,
                    definition,
                    values,
                    &format!("property {property}, moved from node {absorbed} to node {into}"),
                    tx,
                    &mut issues,
                );
            }
        }
    }
    issues
}

/// The checks of one rewritten active assertion: a relation's rewritten ends against its edge
/// type, and a property's object against the declaration its subject's type makes afterwards.
#[allow(clippy::too_many_arguments)]
fn assertion_types(
    ontology: &Ontology,
    nodes: &impl NodeTypes,
    candidate: &Candidate<'_>,
    tx: &GraphTransaction,
    check: &MergeCheck,
    assertion: &Rewritten,
    merged: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let (absorbed, into, into_type) = (check.absorbed, check.into, check.into_type);
    let id = assertion.id;
    match assertion.predicate {
        Predicate::Relation(type_id) => {
            let Some(declared) = ontology.edge_type(type_id) else {
                return;
            };
            let ends = [
                ("source", &declared.source_types, assertion.subject_moved),
                (
                    "target",
                    &declared.target_types,
                    assertion.object_moved && matches!(assertion.object, Object::Node(_)),
                ),
            ];
            for (end, allowed, moved) in ends {
                if moved && !allows(ontology, allowed, into_type) {
                    issues.push(issue(
                        tx,
                        ValidatorName::Type,
                        EDGE_ENDPOINT_TYPE,
                        format!(
                            "the {end} of assertion {id} would be node {into}, a {into_type}, \
                             {merged}, which is not an allowed {end} type of {}; retract the \
                             assertion in an earlier revision, then merge",
                            declared.name
                        ),
                    ));
                }
            }
        }
        Predicate::Property(property) => {
            let definitions = match assertion.subject {
                Some(Subject::Node(node)) => {
                    let Some(type_id) = nodes.type_of(node) else {
                        return;
                    };
                    let type_id = if node == into { into_type } else { type_id };
                    ontology.properties_of(type_id)
                }
                Some(Subject::Edge(edge)) => {
                    let Some(declared) = candidate
                        .edges
                        .get(&edge)
                        .and_then(|edge| ontology.edge_type(edge.type_id))
                    else {
                        return;
                    };
                    declared
                        .properties
                        .iter()
                        .map(|(id, held)| (*id, held))
                        .collect()
                }
                Some(Subject::Type(_)) | None => return,
            };
            let Some(definition) = definitions.get(&property) else {
                if assertion.subject_moved {
                    issues.push(issue(
                        tx,
                        ValidatorName::Type,
                        MERGE_UNDECLARED_PROPERTY,
                        format!(
                            "assertion {id} gives node {absorbed} property {property}, which \
                             type {into_type} of node {into} does not declare; retract the \
                             assertion in an earlier revision, then merge"
                        ),
                    ));
                }
                return;
            };
            let value = match &assertion.object {
                Object::Value(value) => value.clone(),
                Object::Node(node) => CanonicalValue::NodeRef(*node),
                Object::Type(_) => return,
            };
            values_admitted(
                ontology,
                nodes,
                definition,
                std::slice::from_ref(&value),
                &format!("property {property} of assertion {id}, {merged}"),
                tx,
                issues,
            );
        }
    }
}

/// Whether a node of `type_id` may be at an end that allows `allowed`.
fn allows(ontology: &Ontology, allowed: &BTreeSet<TypeId>, type_id: TypeId) -> bool {
    allowed
        .iter()
        .any(|permitted| ontology.conforms_to(type_id, *permitted))
}

/// Refuses each of `values` its definition does not admit, as `Types` refuses a written value. A
/// `NodeRef` to a node the head no longer holds is the reference validator's question.
fn values_admitted(
    ontology: &Ontology,
    nodes: &impl NodeTypes,
    definition: &PropertyDefinition,
    values: &[CanonicalValue],
    at: &str,
    tx: &GraphTransaction,
    issues: &mut Vec<ValidationIssue>,
) {
    for value in values {
        let written = Value::from(value.clone());
        if let Err(reason) = ontology.check_value(&written, &definition.value_type, nodes) {
            if matches!(reason, CheckReason::UnresolvedNodeRef { .. }) {
                continue;
            }
            issues.push(issue(
                tx,
                ValidatorName::Type,
                WRONG_TYPE,
                format!("{at}: {reason}"),
            ));
        }
    }
}

/// `Cardinality`'s checks of a merge in the shape that applies: the node kept's outgoing edges
/// counted after the duplicates collapse, and each moved property's values.
pub(super) fn cardinality(
    snapshot: &GraphSnapshot<'_>,
    tx: &GraphTransaction,
    candidate: &Candidate<'_>,
) -> Vec<ValidationIssue> {
    let checks = candidate.merge_checks(snapshot, tx);
    if checks.is_empty() {
        return Vec::new();
    }
    let ontology = &snapshot.graph().ontology;
    let mut issues = Vec::new();
    for check in checks.iter().filter(|check| check.related) {
        let (absorbed, into) = (check.absorbed, check.into);
        for (type_id, edges) in &check.outgoing {
            let Some(declared) = ontology.edge_type(*type_id) else {
                continue;
            };
            if !declared.cardinality.permits(edges.len()) {
                issues.push(issue(
                    tx,
                    ValidatorName::Cardinality,
                    EDGE_CARDINALITY,
                    format!(
                        "{} is {} and node {into} would have {} edges of it once node {absorbed} \
                         is merged into it: {:?}",
                        declared.name,
                        declared.cardinality,
                        edges.len(),
                        edges.iter().map(ToString::to_string).collect::<Vec<_>>()
                    ),
                ));
            }
        }
        let properties = ontology.properties_of(check.into_type);
        for (property, values) in &check.moved {
            let Some(definition) = properties.get(property) else {
                continue;
            };
            if !definition.cardinality.permits(values.len()) {
                issues.push(issue(
                    tx,
                    ValidatorName::Cardinality,
                    PROPERTY_CARDINALITY,
                    format!(
                        "property {property} ({}) is {} and would carry {} values on node {into} \
                         once node {absorbed} is merged into it",
                        definition.name,
                        definition.cardinality,
                        values.len()
                    ),
                ));
            }
        }
    }
    issues
}

/// Ids, comma-separated, for a message.
fn listed<T: std::fmt::Display>(ids: impl Iterator<Item = T>) -> String {
    ids.map(|id| id.to_string()).collect::<Vec<_>>().join(", ")
}
