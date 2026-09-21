//! Validator 4 of design § 20: every count the ontology constrains is within it.
//!
//! Two counts, because the ontology constrains two different things and design § 11.2 and § 12
//! name them separately:
//!
//! * **a property's** `cardinality` — how many values one property of one node or edge carries,
//!   with `required` making zero a refusal; and
//! * **an edge type's** `cardinality` — how many edges of that type leave one source.
//!
//! The second is the one the story's acceptance names, and it is the one that cannot be answered
//! from the transaction alone: an edge type that permits one edge per source is violated by a
//! proposal that adds a *first* edge to a source canonical state already has one for. So the count
//! is taken over canonical state and the transaction together — created edges in, deleted edges
//! out — which is the graph as it would be if this transaction committed.
//!
//! # What it is silent about
//!
//! A node whose type it cannot name and a property the type does not declare: both are the type
//! validator's refusals, and a count against a definition that does not exist is not a count.

use std::collections::BTreeSet;

use ekr_core::{EdgeId, NodeId, PropertyId, TypeId};
use ekr_graph::GraphSnapshot;
use ekr_ontology::{NodeTypes, PropertyDefinition};

use super::{finish, issue, node_types, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphOperation, GraphTransaction};

/// More values than the property's cardinality permits.
const PROPERTY_CARDINALITY: &str = "property-cardinality";

/// A required property carrying no value.
const MISSING_REQUIRED_PROPERTY: &str = "missing-required-property";

/// More edges of one type out of one source than the edge type permits.
const EDGE_CARDINALITY: &str = "edge-cardinality";

/// Validator 4: cardinality.
pub struct Cardinality;

impl Validator for Cardinality {
    fn name(&self) -> ValidatorName {
        ValidatorName::Cardinality
    }

    fn validate(
        &self,
        graph: &GraphSnapshot<'_>,
        tx: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>> {
        let ontology = &graph.graph().ontology;
        let nodes = node_types(graph, tx);
        let mut issues = Vec::new();

        for operation in &tx.operations {
            match operation {
                GraphOperation::CreateNode(draft) => {
                    if ontology.node_type(draft.type_id).is_none() {
                        continue;
                    }
                    // Presence is asked of the *declarations*, not of the bag the draft carries:
                    // an absent property and one carrying no values are the same absence.
                    for (property, definition) in ontology.properties_of(draft.type_id) {
                        let count = draft.properties.get(&property).map_or(0, Vec::len);
                        count_property(tx, definition, property, count, &mut issues);
                    }
                }
                GraphOperation::UpdateProperty(mutation) => {
                    let Some(definition) = nodes.type_of(mutation.node).and_then(|type_id| {
                        ontology
                            .properties_of(type_id)
                            .get(&mutation.property)
                            .copied()
                    }) else {
                        continue;
                    };
                    count_property(
                        tx,
                        definition,
                        mutation.property,
                        mutation.values.len(),
                        &mut issues,
                    );
                }
                GraphOperation::CreateEdge(draft) => {
                    let Some(declared) = ontology.edge_type(draft.type_id) else {
                        continue;
                    };
                    for (property, definition) in &declared.properties {
                        let count = draft.properties.get(property).map_or(0, Vec::len);
                        count_property(tx, definition, *property, count, &mut issues);
                    }
                    let out = outgoing(graph, tx, draft.source, draft.type_id);
                    if !declared.cardinality.permits(out.len()) {
                        issues.push(issue(
                            tx,
                            ValidatorName::Cardinality,
                            EDGE_CARDINALITY,
                            format!(
                                "{} is {} and node {} would have {} edges of it: {:?}",
                                declared.name,
                                declared.cardinality,
                                draft.source,
                                out.len(),
                                out.iter().map(ToString::to_string).collect::<Vec<_>>()
                            ),
                        ));
                    }
                }
                GraphOperation::DeleteEdge(_)
                | GraphOperation::AddAssertion(_)
                | GraphOperation::RetractAssertion(_)
                | GraphOperation::MergeEntity(_)
                | GraphOperation::Invoke { .. }
                | GraphOperation::DefineNodeType(_)
                | GraphOperation::DefineEdgeType(_)
                | GraphOperation::ModifyProperty(_) => {}
            }
        }

        finish(issues)
    }
}

/// One property's value count, against what its definition permits.
fn count_property(
    tx: &GraphTransaction,
    definition: &PropertyDefinition,
    property: PropertyId,
    count: usize,
    issues: &mut Vec<ValidationIssue>,
) {
    if !definition.cardinality.permits(count) {
        issues.push(issue(
            tx,
            ValidatorName::Cardinality,
            PROPERTY_CARDINALITY,
            format!(
                "property {property} ({}) is {} and carries {count} values",
                definition.name, definition.cardinality
            ),
        ));
    }
    if definition.required && count == 0 {
        issues.push(issue(
            tx,
            ValidatorName::Cardinality,
            MISSING_REQUIRED_PROPERTY,
            format!(
                "property {property} ({}) is required and carries no value",
                definition.name
            ),
        ));
    }
}

/// The edges of `type_id` leaving `source` once this transaction has been applied.
///
/// **Order-free, and that is the point.** It used to replay the operations in `Vec` order, so a
/// `DeleteEdge` listed before the `CreateEdge` of the same edge removed nothing and the edge was
/// counted as surviving — the same operation set accepted in one order and refused with
/// `edge-cardinality` in the other. That made this validator and the reference validator hold two
/// different theories of what a transaction is, and the reference validator's is the right one:
/// design § 19–20 makes a transaction atomic, there is no instant at which canonical state is
/// half-changed, and a verdict that turns on the order of a vector the proposer fills is a verdict
/// an agent can shop for by reordering.
///
/// So the answer is the set: what canonical state holds, plus everything the transaction creates,
/// minus everything it deletes. `tests/adversary_membrane.rs` holds the two orderings to one
/// verdict.
fn outgoing(
    graph: &GraphSnapshot<'_>,
    tx: &GraphTransaction,
    source: NodeId,
    type_id: TypeId,
) -> BTreeSet<EdgeId> {
    let mut held: BTreeSet<EdgeId> = graph
        .graph()
        .edges
        .values()
        .filter(|edge| edge.source.node() == source && edge.type_id == type_id)
        .map(|edge| edge.id)
        .collect();
    held.extend(
        tx.operations
            .iter()
            .filter_map(|operation| match operation {
                GraphOperation::CreateEdge(draft)
                    if draft.source == source && draft.type_id == type_id =>
                {
                    Some(draft.id)
                }
                _ => None,
            }),
    );
    let deleted: BTreeSet<EdgeId> = tx
        .operations
        .iter()
        .filter_map(|operation| match operation {
            GraphOperation::DeleteEdge(edge) => Some(*edge),
            _ => None,
        })
        .collect();
    held.retain(|edge| !deleted.contains(edge));
    held
}
