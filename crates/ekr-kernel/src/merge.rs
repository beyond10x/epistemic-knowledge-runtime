//! What one `MergeEntity` does to a graph (`story:merge-entity-applies`,
//! `architecture-decision-record:0018-merge-and-split-identity`): the rules commit applies and
//! validation holds a proposal to, in one place, so the two cannot answer one merge differently.
//!
//! A merge is a supersession whose references are rewritten in place. The absorbed node leaves the
//! head into a supersession record naming the node kept, as a `SupersedeNode` retires it; every
//! edge end, assertion subject and object and `NodeRef` value naming it names the node kept
//! instead, keeping its id; an edge joining the two is retired into the record; an edge the node
//! kept already holds — same type, direction and other end, equal properties — absorbs its
//! duplicate, which is retired into the record and mapped to it; a property only the absorbed node
//! holds moves to the node kept. The record's [`MergeLineage`] says what moved.
//!
//! Only the shape that applies is read here: a merge that writes `reason` or `evidence`
//! ([`EntityMerge::is_bare`]). The bare `{absorbed, into}` shape is P1's, refused as it was, and
//! reaches nothing in this module.

use std::borrow::Cow;
use std::collections::BTreeMap;

use ekr_core::{EdgeId, NodeId, PropertyId, RevisionNumber, TypeId};
use ekr_graph::{
    CanonicalGraph, CanonicalRef, CanonicalValue, Edge, MergeLineage, MergedEnds, Object,
    RewrittenAssertion, Subject, SupersededNode, ValueSpace,
};
use ekr_store::StoreError;

use crate::transaction::{EntityMerge, GraphOperation, GraphTransaction};

/// The merges of `tx` in the shape that applies, in operation order.
pub(crate) fn shaped<V: ValueSpace>(
    tx: &GraphTransaction<V>,
) -> impl Iterator<Item = &EntityMerge> {
    tx.operations
        .iter()
        .filter_map(|operation| match operation {
            GraphOperation::MergeEntity(merge) if !merge.is_bare() => Some(merge),
            _ => None,
        })
}

/// Whether a `NodeRef` inside `value`, at any depth, names `node`.
pub(crate) fn names(value: &CanonicalValue, node: NodeId) -> bool {
    match value {
        CanonicalValue::NodeRef(held) => held.node() == node,
        CanonicalValue::List(items) => items.iter().any(|item| names(item, node)),
        CanonicalValue::Record(fields) => fields.values().any(|field| names(field, node)),
        CanonicalValue::String(_)
        | CanonicalValue::Boolean(_)
        | CanonicalValue::Integer(_)
        | CanonicalValue::Decimal(_)
        | CanonicalValue::Timestamp(_)
        | CanonicalValue::Duration(_)
        | CanonicalValue::Enum(_) => false,
    }
}

/// `value` with every `NodeRef` to `absorbed`, at any depth, naming `into`.
pub(crate) fn rewrite(value: &CanonicalValue, absorbed: NodeId, into: NodeId) -> CanonicalValue {
    match value {
        CanonicalValue::NodeRef(held) if held.node() == absorbed => {
            CanonicalValue::NodeRef(CanonicalRef::new(into))
        }
        CanonicalValue::List(items) => CanonicalValue::List(
            items
                .iter()
                .map(|item| rewrite(item, absorbed, into))
                .collect(),
        ),
        CanonicalValue::Record(fields) => CanonicalValue::Record(
            fields
                .iter()
                .map(|(name, field)| (name.clone(), rewrite(field, absorbed, into)))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// `values` rewritten, when one of them names `absorbed`; `None` when none does.
pub(crate) fn rewritten(
    values: &[CanonicalValue],
    absorbed: NodeId,
    into: NodeId,
) -> Option<Vec<CanonicalValue>> {
    values.iter().any(|value| names(value, absorbed)).then(|| {
        values
            .iter()
            .map(|value| rewrite(value, absorbed, into))
            .collect()
    })
}

/// The values of a node's or an edge's properties, by property.
pub(crate) type Bag = BTreeMap<PropertyId, Vec<CanonicalValue>>;

/// A property bag with every `NodeRef` to `absorbed` naming `into`, borrowed when none does.
fn rewritten_bag(properties: &Bag, absorbed: NodeId, into: NodeId) -> Cow<'_, Bag> {
    if properties
        .values()
        .flatten()
        .any(|value| names(value, absorbed))
    {
        Cow::Owned(
            properties
                .iter()
                .map(|(property, values)| {
                    (
                        *property,
                        values
                            .iter()
                            .map(|value| rewrite(value, absorbed, into))
                            .collect(),
                    )
                })
                .collect(),
        )
    } else {
        Cow::Borrowed(properties)
    }
}

/// One edge as a merge reads it: of the graph the merge applies to, after every other operation of
/// its transaction.
pub(crate) struct EdgeAt<'a> {
    pub(crate) id: EdgeId,
    pub(crate) type_id: TypeId,
    pub(crate) source: NodeId,
    pub(crate) target: NodeId,
    pub(crate) properties: Cow<'a, Bag>,
}

impl<'a> EdgeAt<'a> {
    /// A canonical edge, borrowed.
    pub(crate) fn of(edge: &'a Edge) -> Self {
        Self {
            id: edge.id,
            type_id: edge.type_id,
            source: edge.source.node(),
            target: edge.target.node(),
            properties: Cow::Borrowed(&edge.properties),
        }
    }

    /// Whether `node` is one of its ends.
    pub(crate) fn touches(&self, node: NodeId) -> bool {
        self.source == node || self.target == node
    }
}

/// What becomes of one edge of the absorbed node (ADR 0018, decision 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Fate {
    /// (a) It joins the absorbed node and the node kept: retired into the record, whole.
    Retired,
    /// (b) It has the type, direction and other end of this edge of the node kept, and equal
    /// properties: it collapses onto it, retired into the record and mapped to it.
    Collapsed(EdgeId),
    /// (b) It has the type, direction and other end of this edge of the node kept, and different
    /// properties: the merge is refused.
    Conflicting(EdgeId),
    /// (c) It keeps its id, these ends naming the node kept.
    Rewritten(MergedEnds),
}

/// What makes two edges duplicates: their type, their source and their target.
type EdgeKey = (TypeId, NodeId, NodeId);

/// The fate of each edge of `absorbed`, by id, among `edges`: the edges at `absorbed` or `into`
/// of the graph the merge applies to, after every other operation of its transaction. An edge
/// collapses onto the lowest id of the node kept's edges it duplicates with equal properties —
/// compared with every `NodeRef` to `absorbed` naming `into` on both sides, since that is what
/// each holds once the merge applies — and conflicts with the lowest id when none is equal.
pub(crate) fn fates(
    absorbed: NodeId,
    into: NodeId,
    edges: &[EdgeAt<'_>],
) -> BTreeMap<EdgeId, Fate> {
    let mut held: BTreeMap<EdgeKey, Vec<(EdgeId, Cow<'_, Bag>)>> = BTreeMap::new();
    for edge in edges
        .iter()
        .filter(|edge| edge.touches(into) && !edge.touches(absorbed))
    {
        held.entry((edge.type_id, edge.source, edge.target))
            .or_default()
            .push((
                edge.id,
                rewritten_bag(edge.properties.as_ref(), absorbed, into),
            ));
    }
    for list in held.values_mut() {
        list.sort_by_key(|(id, _)| *id);
    }
    let end = |node: NodeId| if node == absorbed { into } else { node };
    edges
        .iter()
        .filter(|edge| edge.touches(absorbed))
        .map(|edge| {
            let fate = if (edge.source == absorbed && edge.target == into)
                || (edge.source == into && edge.target == absorbed)
            {
                Fate::Retired
            } else {
                let ends = match (edge.source == absorbed, edge.target == absorbed) {
                    (true, true) => MergedEnds::Both,
                    (true, false) => MergedEnds::Source,
                    _ => MergedEnds::Target,
                };
                match held.get(&(edge.type_id, end(edge.source), end(edge.target))) {
                    None => Fate::Rewritten(ends),
                    Some(duplicates) => {
                        let mine = rewritten_bag(edge.properties.as_ref(), absorbed, into);
                        duplicates
                            .iter()
                            .find(|(_, theirs)| **theirs == *mine)
                            .map_or(Fate::Conflicting(duplicates[0].0), |(kept, _)| {
                                Fate::Collapsed(*kept)
                            })
                    }
                }
            };
            (edge.id, fate)
        })
        .collect()
}

/// Applies the merge of `tx`, if it carries one, to `graph` at its revision, after every other
/// operation of the transaction, supersessions included. Validation held the transaction to one
/// merge, its absorbed node held and named by no other operation, its node kept held and retired
/// by no other operation, and every fate and property free of conflict, so a refusal here is
/// unreachable.
pub(crate) fn apply(
    graph: &mut CanonicalGraph,
    tx: &GraphTransaction<CanonicalValue>,
) -> Result<(), StoreError> {
    let Some(merge) = shaped(tx).next() else {
        return Ok(());
    };
    let (absorbed, into) = (merge.absorbed, merge.into);
    let admitted = |what: &str| StoreError::Document(format!("admitted-merge-{what}"));
    let node = graph
        .nodes
        .remove(&absorbed)
        .ok_or_else(|| admitted("node-missing"))?;
    if !graph.nodes.contains_key(&into) {
        return Err(admitted("node-missing"));
    }
    let mut lineage = MergeLineage::default();

    // The edges, by fate: one pass over the head's edges finds the ones at either node.
    let fated = {
        let near: Vec<EdgeAt<'_>> = graph
            .edges
            .values()
            .filter(|edge| {
                let (source, target) = (edge.source.node(), edge.target.node());
                source == absorbed || target == absorbed || source == into || target == into
            })
            .map(EdgeAt::of)
            .collect();
        fates(absorbed, into, &near)
    };
    let mut edges = BTreeMap::new();
    for (id, fate) in &fated {
        match *fate {
            Fate::Retired | Fate::Collapsed(_) => {
                let edge = graph
                    .edges
                    .remove(id)
                    .ok_or_else(|| admitted("edge-missing"))?;
                edges.insert(*id, edge);
                match *fate {
                    Fate::Collapsed(kept) => {
                        lineage.collapsed.insert(*id, kept);
                    }
                    _ => {
                        lineage.retired.insert(*id);
                    }
                }
            }
            Fate::Rewritten(ends) => {
                let edge = graph
                    .edges
                    .get_mut(id)
                    .ok_or_else(|| admitted("edge-missing"))?;
                if edge.source.node() == absorbed {
                    edge.source = CanonicalRef::new(into);
                }
                if edge.target.node() == absorbed {
                    edge.target = CanonicalRef::new(into);
                }
                lineage.edges.insert(*id, ends);
            }
            Fate::Conflicting(_) => return Err(admitted("edge-conflict")),
        }
    }

    // `NodeRef` values, in the head's edges and nodes: the node kept's own before its properties
    // are compared with the absorbed node's.
    for edge in graph.edges.values_mut() {
        for (property, values) in &mut edge.properties {
            if let Some(new) = rewritten(values, absorbed, into) {
                let prior = std::mem::replace(values, new);
                lineage
                    .edge_values
                    .entry(edge.id)
                    .or_default()
                    .insert(*property, prior);
            }
        }
    }
    for held in graph.nodes.values_mut() {
        for (property, values) in &mut held.properties {
            if let Some(new) = rewritten(values, absorbed, into) {
                let prior = std::mem::replace(values, new);
                lineage
                    .node_values
                    .entry(held.id)
                    .or_default()
                    .insert(*property, prior);
            }
        }
    }

    // A property only the absorbed node holds moves; one both hold equally stays.
    let kept = graph
        .nodes
        .get_mut(&into)
        .ok_or_else(|| admitted("node-missing"))?;
    for (property, values) in &node.properties {
        let mine = rewritten(values, absorbed, into).unwrap_or_else(|| values.clone());
        match kept.properties.get(property) {
            None => {
                kept.properties.insert(*property, mine);
                lineage.moved_properties.insert(*property);
            }
            Some(theirs) if *theirs == mine => {}
            Some(_) => return Err(admitted("property-conflict")),
        }
    }

    // Assertions, in place: every byte but the reference rewritten. An active relation joining
    // the two nodes was refused by validation (`merge-self-relation`); a retracted or superseded
    // one is rewritten as any other is, and relates the node kept to itself.
    for assertion in graph.assertions.values_mut() {
        let mut change = RewrittenAssertion {
            subject: None,
            object: None,
        };
        let subject = match assertion.subject {
            Subject::Node(held) if held.node() == absorbed => {
                Some(Subject::Node(CanonicalRef::new(into)))
            }
            Subject::Edge(held) => match fated.get(&held.id()) {
                Some(Fate::Collapsed(kept)) => Some(Subject::Edge(CanonicalRef::new(*kept))),
                _ => None,
            },
            _ => None,
        };
        if let Some(subject) = subject {
            change.subject = Some(std::mem::replace(&mut assertion.subject, subject));
        }
        let object = match &assertion.object {
            Object::Node(held) if held.node() == absorbed => {
                Some(Object::Node(CanonicalRef::new(into)))
            }
            Object::Value(value) if names(value, absorbed) => {
                Some(Object::Value(rewrite(value, absorbed, into)))
            }
            _ => None,
        };
        if let Some(object) = object {
            change.object = Some(std::mem::replace(&mut assertion.object, object));
        }
        if change.subject.is_some() || change.object.is_some() {
            lineage.assertions.insert(assertion.id, change);
        }
    }

    // No chain: a record whose node kept was absorbed now names the node kept by the merge.
    for record in graph.superseded.values_mut() {
        if record.by.node() == absorbed {
            record.by = CanonicalRef::new(into);
        }
    }
    let revision: RevisionNumber = graph.revision;
    graph.superseded.insert(
        absorbed,
        SupersededNode {
            node,
            edges,
            by: CanonicalRef::new(into),
            revision,
            reason: merge.reason.clone().unwrap_or_default(),
            evidence: merge
                .evidence
                .iter()
                .flatten()
                .copied()
                .map(CanonicalRef::new)
                .collect(),
            merge: Some(lineage),
        },
    );
    Ok(())
}
