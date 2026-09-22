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

use ekr_core::PropertyId;
use ekr_graph::GraphSnapshot;
use ekr_ontology::{NodeTypes, PropertyDefinition};

use super::{candidate::Candidate, finish, issue, Validator};
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
        let candidate = Candidate::of(graph, tx);
        let nodes = &candidate.nodes;
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
                        let count = candidate
                            .property_counts
                            .get(&draft.id)
                            .and_then(|properties| properties.get(&property))
                            .copied()
                            .unwrap_or(0);
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
                    let out = candidate.outgoing(draft.source, draft.type_id);
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
                | GraphOperation::SupersedeAssertion(_)
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
