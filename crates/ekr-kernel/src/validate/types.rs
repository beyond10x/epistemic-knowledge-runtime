//! Validator 3 of design § 20: every value satisfies its declared type, and is one canonical
//! state admits.
//!
//! Design § 6.3 and § 73. The checking itself is `ekr_ontology`'s — `Ontology::check_value` — and
//! this module's job is to find every value a transaction carries, discover the type each value is
//! declared to have, and turn a `CheckReason` into an issue that names the operation it came from.
//!
//! # The second question this validator asks
//!
//! `architecture-decision-record:0005-float-is-not-canonical` gives the kernel's type validator
//! the other half of the rule: an operation carrying a value canonical state does not admit is
//! refused here, with an issue naming it. `Value::inadmissible_in_canonical_state` is the
//! ontology's statement of that rule and answers with a `ValuePath` — where inside the value the
//! offending part sits — because a proposer holding a record of a hundred fields cannot act on "a
//! float is in here somewhere".
//!
//! It is asked of every value, including the ones whose declared type could not be found. A float
//! under an undeclared property is still a float, and the transaction is being refused either way.
//!
//! # What this validator is silent about
//!
//! * A value whose node the reference validator could not resolve, and a `NodeRef` to a node that
//!   is not there: `CheckReason::UnresolvedNodeRef` is that same finding seen from here, and it is
//!   dropped rather than repeated. Which validator refused is a statement the suite asserts.
//! * An `Invoke` naming an operation the type does not declare — the ontology-constraint
//!   validator's refusal, and the arguments of an operation that does not exist have no declared
//!   types to be checked against.
//! * Whether a schema operation's *declaration* coheres — that a `NodeRef` allows some type, that
//!   a parent chain has no cycle. `Ontology::load` answers it for a whole document and the kernel
//!   has no half-applied ontology to ask; it arrives with the schema-transaction story, not here.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{NodeId, PropertyId, TypeId};
use ekr_graph::{GraphSnapshot, Object, Predicate, Subject};
use ekr_ontology::{CheckReason, NodeTypes, Ontology, PropertyDefinition, Value};

use super::{candidate::Candidate, finish, issue, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphOperation, GraphTransaction};

/// A type the ontology does not declare.
const UNKNOWN_TYPE: &str = "unknown-type";

/// A type the ontology declares abstract. An abstract type has no instances.
const ABSTRACT_TYPE: &str = "abstract-type";

/// A property the type does not declare.
const UNDECLARED_PROPERTY: &str = "undeclared-property";

/// A value that does not satisfy the type its property declares.
const WRONG_TYPE: &str = "wrong-type";

/// A value canonical state does not admit: `architecture-decision-record:0005`.
const INADMISSIBLE_VALUE: &str = "inadmissible-value";

/// An edge whose endpoint is not of a type the edge type allows.
const EDGE_ENDPOINT_TYPE: &str = "edge-endpoint-type";

/// An argument the invoked operation does not declare.
const UNDECLARED_ARGUMENT: &str = "undeclared-argument";

/// An argument the invoked operation declares and the invocation does not carry.
const MISSING_ARGUMENT: &str = "missing-argument";

/// Validator 3: type checking.
pub struct Types;

impl Validator for Types {
    fn name(&self) -> ValidatorName {
        ValidatorName::Type
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

        // Admissibility first, over every value the transaction carries, and independently of
        // whether the type each value is declared to have could be found. The two questions are
        // separate: a float under an undeclared property, or under a node that is not there, is
        // still a value canonical state cannot hold, and asking only where a declaration was
        // resolved would leave a proposal's refusal depending on which *other* validator also
        // refused it. `a_float_anywhere_is_always_refused` in `tests/validate_properties.rs` is
        // this paragraph as a property, and it failed before this pass existed.
        for operation in &tx.operations {
            for (at, value) in values_of(operation) {
                admissible(tx, value, &at, &mut issues);
            }
        }

        for operation in &tx.operations {
            match operation {
                GraphOperation::CreateNode(draft) => {
                    let Some(declared) = ontology.node_type(draft.type_id) else {
                        issues.push(issue(
                            tx,
                            ValidatorName::Type,
                            UNKNOWN_TYPE,
                            format!(
                                "node {} claims type {}, which the ontology does not declare",
                                draft.id, draft.type_id
                            ),
                        ));
                        continue;
                    };
                    if declared.abstract_type {
                        issues.push(issue(
                            tx,
                            ValidatorName::Type,
                            ABSTRACT_TYPE,
                            format!(
                                "node {} claims type {}, which is abstract and has no instances",
                                draft.id, draft.type_id
                            ),
                        ));
                        continue;
                    }
                    let definitions = ontology.properties_of(draft.type_id);
                    for (property, values) in &draft.properties {
                        check_values(
                            tx,
                            ontology,
                            nodes,
                            definitions.get(property).copied(),
                            values,
                            &format!("property {property} of node {}", draft.id),
                            &mut issues,
                        );
                    }
                }
                GraphOperation::UpdateProperty(mutation) => {
                    let Some(type_id) = nodes.type_of(mutation.node) else {
                        continue;
                    };
                    let definitions = ontology.properties_of(type_id);
                    check_values(
                        tx,
                        ontology,
                        nodes,
                        definitions.get(&mutation.property).copied(),
                        &mutation.values,
                        &format!("property {} of node {}", mutation.property, mutation.node),
                        &mut issues,
                    );
                }
                GraphOperation::CreateEdge(draft) => {
                    let Some(declared) = ontology.edge_type(draft.type_id) else {
                        issues.push(issue(
                            tx,
                            ValidatorName::Type,
                            UNKNOWN_TYPE,
                            format!(
                                "edge {} claims type {}, which the ontology does not declare",
                                draft.id, draft.type_id
                            ),
                        ));
                        continue;
                    };
                    for (end, node, allowed) in [
                        ("source", draft.source, &declared.source_types),
                        ("target", draft.target, &declared.target_types),
                    ] {
                        let Some(node_type) = nodes.type_of(node) else {
                            continue;
                        };
                        if !allowed
                            .iter()
                            .any(|permitted| ontology.conforms_to(node_type, *permitted))
                        {
                            issues.push(issue(
                                tx,
                                ValidatorName::Type,
                                EDGE_ENDPOINT_TYPE,
                                format!(
                                    "edge {} has {end} {node}, a {node_type}, which is not an \
                                     allowed {end} type of {}",
                                    draft.id, declared.name
                                ),
                            ));
                        }
                    }
                    for (property, values) in &draft.properties {
                        check_values(
                            tx,
                            ontology,
                            nodes,
                            declared.properties.get(property),
                            values,
                            &format!("property {property} of edge {}", draft.id),
                            &mut issues,
                        );
                    }
                }
                GraphOperation::AddAssertion(assertion) => {
                    // Three fields, checked one at a time and in every combination, because the
                    // arm used to leave unless the *object* was a value — so one undeclared
                    // property was refused against a value object and accepted against a node
                    // object. One defect had two verdicts, decided by a field that was not the
                    // defect.
                    let what = format!("assertion {}", assertion.id);
                    if let Subject::Type(type_id) = assertion.subject {
                        if !declared_type(
                            tx,
                            ontology,
                            type_id,
                            &format!("the subject of {what}"),
                            &mut issues,
                        ) {
                            continue;
                        }
                    }
                    if let Object::Type(type_id) = assertion.object {
                        if !declared_type(
                            tx,
                            ontology,
                            type_id,
                            &format!("the object of {what}"),
                            &mut issues,
                        ) {
                            continue;
                        }
                    }
                    match assertion.predicate {
                        // A relation is named by its edge type, so the edge type has to exist.
                        // `edge_type` alone rather than `declared_type`: a predicate naming a
                        // *node* type is not a relation, and accepting it because the id resolves
                        // somewhere would be the same kind of hole one index wider.
                        Predicate::Relation(type_id) => {
                            let Some(declared) = ontology.edge_type(type_id) else {
                                issues.push(issue(
                                    tx,
                                    ValidatorName::Type,
                                    UNKNOWN_TYPE,
                                    format!(
                                        "{what} is made through relation {type_id}, which the \
                                         ontology does not declare as an edge type"
                                    ),
                                ));
                                continue;
                            };
                            let source = match assertion.subject {
                                Subject::Node(node) => Some(node),
                                _ => None,
                            };
                            let target = match assertion.object {
                                Object::Node(node) => Some(node),
                                _ => None,
                            };
                            for (end, node, allowed) in [
                                ("source", source, &declared.source_types),
                                ("target", target, &declared.target_types),
                            ] {
                                check_endpoint(
                                    tx,
                                    ontology,
                                    nodes,
                                    node,
                                    allowed,
                                    &format!("{end} of {what}"),
                                    &mut issues,
                                );
                            }
                        }
                        // The property is looked up through the subject's type, and *then* the
                        // object is checked against what it declares — a node object as the
                        // `NodeRef` it is, so that the allowed types of the reference are held to
                        // as they would be in any other property value.
                        Predicate::Property(property) => {
                            let definitions = match assertion.subject {
                                Subject::Node(node) => {
                                    let Some(type_id) = nodes.type_of(node) else {
                                        continue;
                                    };
                                    ontology.properties_of(type_id)
                                }
                                Subject::Edge(edge) => {
                                    let Some(declared) = candidate
                                        .edges
                                        .get(&edge)
                                        .and_then(|edge| ontology.edge_type(edge.type_id))
                                    else {
                                        continue;
                                    };
                                    declared
                                        .properties
                                        .iter()
                                        .map(|(id, property)| (*id, property))
                                        .collect()
                                }
                                // No metatype declares properties of ontology definitions in P1.
                                Subject::Type(_) => BTreeMap::new(),
                            };
                            let declared = definitions.get(&property).copied();
                            let at = format!("property {property} of {what}");
                            match &assertion.object {
                                Object::Value(value) => {
                                    check(tx, ontology, nodes, declared, value, &at, &mut issues);
                                }
                                Object::Node(node) => check(
                                    tx,
                                    ontology,
                                    nodes,
                                    declared,
                                    &Value::NodeRef(*node),
                                    &at,
                                    &mut issues,
                                ),
                                // No property ValueType admits a TypeId. Existence is insufficient.
                                Object::Type(_) => {
                                    if declared.is_none() {
                                        issues.push(issue(
                                            tx,
                                            ValidatorName::Type,
                                            UNDECLARED_PROPERTY,
                                            format!("{at} is not declared by the type"),
                                        ));
                                    } else {
                                        issues.push(issue(
                                            tx,
                                            ValidatorName::Type,
                                            WRONG_TYPE,
                                            format!("{at} cannot carry a type as its value"),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                GraphOperation::Invoke {
                    node,
                    operation,
                    arguments,
                } => {
                    let Some(declared) = nodes
                        .type_of(*node)
                        .and_then(|type_id| ontology.node_type(type_id))
                        .and_then(|declared| declared.operations.get(operation))
                    else {
                        continue;
                    };
                    for (name, value) in arguments {
                        let Some(argument_type) = declared.arguments.get(name) else {
                            issues.push(issue(
                                tx,
                                ValidatorName::Type,
                                UNDECLARED_ARGUMENT,
                                format!(
                                    "{operation:?} on node {node} was given an argument \
                                     {name:?}, which it does not declare"
                                ),
                            ));
                            continue;
                        };
                        if let Err(reason) = ontology.check_value(value, argument_type, nodes) {
                            refuse_reason(
                                tx,
                                &reason,
                                &format!("argument {name:?} of {operation:?} on node {node}"),
                                &mut issues,
                            );
                        }
                    }
                    for name in declared.arguments.keys() {
                        if !arguments.contains_key(name) {
                            issues.push(issue(
                                tx,
                                ValidatorName::Type,
                                MISSING_ARGUMENT,
                                format!(
                                    "{operation:?} on node {node} declares an argument {name:?}, \
                                     which the invocation does not carry"
                                ),
                            ));
                        }
                    }
                }
                GraphOperation::DeleteEdge(_)
                | GraphOperation::RetractAssertion(_)
                | GraphOperation::SupersedeAssertion(_)
                | GraphOperation::MergeEntity(_)
                | GraphOperation::DefineNodeType(_)
                | GraphOperation::DefineEdgeType(_)
                | GraphOperation::ModifyProperty(_) => {}
            }
        }

        finish(issues)
    }
}

/// Every value an operation carries, each with the place inside the operation it sits.
///
/// The place is what makes a refusal actionable — a proposer holding a transaction of forty
/// operations cannot act on "there is a float in it" — and it is also what tells the two refusals
/// of an inadmissible value apart: this validator names the property, and the conversion
/// `Pipeline::validate` makes afterwards names only the path inside the value.
fn values_of(operation: &GraphOperation) -> Vec<(String, &Value)> {
    match operation {
        GraphOperation::CreateNode(draft) => {
            property_values(&draft.properties, &format!("node {}", draft.id))
        }
        GraphOperation::UpdateProperty(mutation) => mutation
            .values
            .iter()
            .map(|value| {
                (
                    format!("property {} of node {}", mutation.property, mutation.node),
                    value,
                )
            })
            .collect(),
        GraphOperation::CreateEdge(draft) => {
            property_values(&draft.properties, &format!("edge {}", draft.id))
        }
        GraphOperation::AddAssertion(assertion) => match &assertion.object {
            Object::Value(value) => {
                vec![(format!("the object of assertion {}", assertion.id), value)]
            }
            Object::Node(_) | Object::Type(_) => Vec::new(),
        },
        GraphOperation::Invoke {
            node,
            operation: named,
            arguments,
        } => arguments
            .iter()
            .map(|(name, value)| {
                (
                    format!("argument {name:?} of {named:?} on node {node}"),
                    value,
                )
            })
            .collect(),
        GraphOperation::DeleteEdge(_)
        | GraphOperation::RetractAssertion(_)
        | GraphOperation::SupersedeAssertion(_)
        | GraphOperation::DefineNodeType(_)
        | GraphOperation::DefineEdgeType(_)
        | GraphOperation::ModifyProperty(_)
        | GraphOperation::MergeEntity(_) => Vec::new(),
    }
}

/// The values of a property bag, each named by the property that carries it.
fn property_values<'a>(
    properties: &'a BTreeMap<PropertyId, Vec<Value>>,
    what: &str,
) -> Vec<(String, &'a Value)> {
    properties
        .iter()
        .flat_map(|(property, values)| {
            values
                .iter()
                .map(move |value| (format!("property {property} of {what}"), value))
        })
        .collect()
}

/// Refuses a `TypeId` the ontology declares in neither of its two indexes.
///
/// One id space: `ekr.ontology.NodeType` and `ekr.ontology.EdgeType` are keyed by the same
/// `TypeId`, so "declared" means declared as either, and a subject or an object naming a type is
/// not claiming which of the two it is.
fn declared_type(
    tx: &GraphTransaction,
    ontology: &Ontology,
    type_id: TypeId,
    at: &str,
    issues: &mut Vec<ValidationIssue>,
) -> bool {
    if ontology.node_type(type_id).is_none() && ontology.edge_type(type_id).is_none() {
        issues.push(issue(
            tx,
            ValidatorName::Type,
            UNKNOWN_TYPE,
            format!("{at} names type {type_id}, which the ontology does not declare"),
        ));
        false
    } else {
        true
    }
}

/// Relation assertions obey the same endpoint declarations as edges.
fn check_endpoint(
    tx: &GraphTransaction,
    ontology: &Ontology,
    nodes: &impl NodeTypes,
    node: Option<NodeId>,
    allowed: &BTreeSet<TypeId>,
    at: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let compatible = match node {
        Some(node) => {
            let Some(type_id) = nodes.type_of(node) else {
                return;
            };
            allowed
                .iter()
                .any(|permitted| ontology.conforms_to(type_id, *permitted))
        }
        None => false,
    };
    if !compatible {
        issues.push(issue(
            tx,
            ValidatorName::Type,
            EDGE_ENDPOINT_TYPE,
            format!("{at} must name a node of an allowed endpoint type"),
        ));
    }
}

/// Even an empty assignment names a property and must have a declaration.
fn check_values(
    tx: &GraphTransaction,
    ontology: &Ontology,
    nodes: &impl NodeTypes,
    declared: Option<&PropertyDefinition>,
    values: &[Value],
    at: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    if declared.is_none() {
        issues.push(issue(
            tx,
            ValidatorName::Type,
            UNDECLARED_PROPERTY,
            format!("{at} is not declared by the type"),
        ));
        return;
    }
    for value in values {
        check(tx, ontology, nodes, declared, value, at, issues);
    }
}

/// One value, against the type its property declares.
///
/// `declared` is `None` when the property is not declared at all, which is itself a refusal.
/// Admissibility is not asked here: it is asked of every value in the transaction, before any of
/// this, by the pass at the top of [`Types::validate`].
fn check(
    tx: &GraphTransaction,
    ontology: &Ontology,
    nodes: &impl NodeTypes,
    declared: Option<&PropertyDefinition>,
    value: &Value,
    at: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    let Some(definition) = declared else {
        issues.push(issue(
            tx,
            ValidatorName::Type,
            UNDECLARED_PROPERTY,
            format!("{at} is not declared by the type"),
        ));
        return;
    };
    if let Err(reason) = ontology.check_value(value, &definition.value_type, nodes) {
        refuse_reason(tx, &reason, at, issues);
    }
}

/// The ontology's refusal, as an issue — unless it is the reference validator's finding.
fn refuse_reason(
    tx: &GraphTransaction,
    reason: &CheckReason,
    at: &str,
    issues: &mut Vec<ValidationIssue>,
) {
    if matches!(reason, CheckReason::UnresolvedNodeRef { .. }) {
        return;
    }
    issues.push(issue(
        tx,
        ValidatorName::Type,
        WRONG_TYPE,
        format!("{at}: {reason}"),
    ));
}

/// Refuses a value canonical state does not admit, naming where inside the value it sits.
fn admissible(tx: &GraphTransaction, value: &Value, at: &str, issues: &mut Vec<ValidationIssue>) {
    if let Some(path) = value.inadmissible_in_canonical_state() {
        issues.push(issue(
            tx,
            ValidatorName::Type,
            INADMISSIBLE_VALUE,
            format!(
                "{at}: {path} is a Float, which canonical state does not admit — carry a quantity \
                 that must be hashed as an Integer or a Decimal"
            ),
        ));
    }
}
