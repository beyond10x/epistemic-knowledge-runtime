//! Validator 5 of design § 20: an `Invoke` names an operation the type declares, and makes a move
//! the lifecycle declares.
//!
//! Design amendment 87:
//!
//! > The ontology-constraint validator (§ 20, item 5) refuses an `Invoke` whose precondition fails
//! > or whose transition is not declared.
//!
//! Transitions are checked. Opaque preconditions and applicable property constraints refuse:
//! design § 11.2 names `Constraint` and does not define it,
//! `systems/ekr/domains/ontology.yaml` marks the language `UNMAPPED`, and `ekr_ontology` carries
//! preconditions as opaque text for the same reason. A constraint language decided in passing by a
//! validator would be a guess with an integrity boundary behind it. Unimplemented emission
//! semantics refuse too; a declared effect cannot silently disappear.
//!
//! # Where the state comes from
//!
//! A node canonical state holds carries its own `type_state`. A node this transaction creates has
//! the state amendment 87 gives it — its type's `initial` — because a draft carries no state to
//! disagree with the ontology about.
//!
//! A node whose type declares a lifecycle and whose stored state is absent is asked about as
//! `initial` too: that is what a node created before the lifecycle was declared looks like, and
//! `Lifecycle::transition` refuses a state the lifecycle does not have, so nothing is assumed
//! about a state that is present and wrong.

use ekr_graph::{GraphSnapshot, Predicate, Subject};
use ekr_ontology::NodeTypes;

use super::{candidate::Candidate, finish, issue, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphOperation, GraphTransaction};

/// An operation the node's type does not declare.
const OPERATION_NOT_DECLARED: &str = "operation-not-declared";

/// A move the node's lifecycle does not declare, or does not declare from where the node is.
const TRANSITION_REFUSED: &str = "transition-refused";

/// Applicable opaque constraints or operation effects have no P1 evaluator.
const UNSUPPORTED_CONSTRAINT: &str = "unsupported-constraint";

/// Validator 5: ontology constraints.
pub struct OntologyConstraint;

impl Validator for OntologyConstraint {
    fn name(&self) -> ValidatorName {
        ValidatorName::OntologyConstraint
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
            let (node_type, edge_type) = match operation {
                GraphOperation::CreateNode(draft) => (Some(draft.type_id), None),
                GraphOperation::UpdateProperty(mutation) => (nodes.type_of(mutation.node), None),
                GraphOperation::Invoke { node, .. } => (nodes.type_of(*node), None),
                GraphOperation::CreateEdge(draft) => (None, Some(draft.type_id)),
                GraphOperation::AddAssertion(assertion) => match assertion.predicate {
                    Predicate::Relation(id) => (None, Some(id)),
                    Predicate::Property(_) => match assertion.subject {
                        Subject::Node(node) => (nodes.type_of(node), None),
                        Subject::Edge(edge) => {
                            (None, candidate.edges.get(&edge).map(|edge| edge.type_id))
                        }
                        Subject::Type(_) => (None, None),
                    },
                },
                _ => (None, None),
            };
            let constrained = node_type.is_some_and(|id| {
                ontology
                    .properties_of(id)
                    .values()
                    .any(|property| !property.constraints.is_empty())
            }) || edge_type.and_then(|id| ontology.edge_type(id)).is_some_and(
                |edge| {
                    edge.properties
                        .values()
                        .any(|property| !property.constraints.is_empty())
                },
            );
            if constrained {
                issues.push(issue(
                    tx,
                    ValidatorName::OntologyConstraint,
                    UNSUPPORTED_CONSTRAINT,
                    "the affected type declares property constraints with no P1 evaluator"
                        .to_owned(),
                ));
            }
            let GraphOperation::Invoke {
                node,
                operation: named,
                ..
            } = operation
            else {
                continue;
            };
            // A node whose type cannot be named is the reference validator's or the type
            // validator's refusal, and an operation of a type that is not there has nothing to be
            // declared by.
            let Some(declared) = nodes
                .type_of(*node)
                .and_then(|type_id| ontology.node_type(type_id))
            else {
                continue;
            };
            let Some(definition) = declared.operations.get(named) else {
                issues.push(issue(
                    tx,
                    ValidatorName::OntologyConstraint,
                    OPERATION_NOT_DECLARED,
                    format!(
                        "{} declares no operation {named:?}, so node {node} cannot be asked to \
                         perform one",
                        declared.name
                    ),
                ));
                continue;
            };
            if !definition.preconditions.is_empty() || !definition.emits.is_empty() {
                issues.push(issue(tx, ValidatorName::OntologyConstraint, UNSUPPORTED_CONSTRAINT,
                    format!("{named:?} on node {node} declares unsupported preconditions or emitted events")));
            }
            if definition.transition.is_none() {
                continue;
            }
            let Some(lifecycle) = &declared.lifecycle else {
                issues.push(issue(
                    tx,
                    ValidatorName::OntologyConstraint,
                    TRANSITION_REFUSED,
                    format!(
                        "{named:?} moves a node of {} between states, and {} declares no lifecycle",
                        declared.name, declared.name
                    ),
                ));
                continue;
            };
            let state = graph
                .graph()
                .nodes
                .get(node)
                .and_then(|held| held.type_state.clone())
                .unwrap_or_else(|| lifecycle.initial.clone());
            if let Err(refusal) = lifecycle.transition(&state, definition) {
                issues.push(issue(
                    tx,
                    ValidatorName::OntologyConstraint,
                    TRANSITION_REFUSED,
                    format!("{named:?} on node {node}, which is {state:?}: {refusal}"),
                ));
            }
        }

        finish(issues)
    }
}
