//! Validator 5 of design § 20: an `Invoke` names an operation the type declares, and makes a move
//! the lifecycle declares.
//!
//! Design amendment 87:
//!
//! > The ontology-constraint validator (§ 20, item 5) refuses an `Invoke` whose precondition fails
//! > or whose transition is not declared.
//!
//! The transition half is here. **Preconditions are not refused**, and that is deliberate rather
//! than missing: design § 11.2 names `Constraint` and does not define it,
//! `systems/ekr/domains/ontology.yaml` marks the language `UNMAPPED`, and `ekr_ontology` carries
//! preconditions as opaque text for the same reason. A constraint language decided in passing by a
//! validator would be a guess with an integrity boundary behind it.
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

use ekr_graph::GraphSnapshot;
use ekr_ontology::NodeTypes;

use super::{finish, issue, node_types, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphOperation, GraphTransaction};

/// An operation the node's type does not declare.
const OPERATION_NOT_DECLARED: &str = "operation-not-declared";

/// A move the node's lifecycle does not declare, or does not declare from where the node is.
const TRANSITION_REFUSED: &str = "transition-refused";

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
        let nodes = node_types(graph, tx);
        let mut issues = Vec::new();

        for operation in &tx.operations {
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
