//! The type checker: what validators 3 to 5 of design § 20 call.
//!
//! It answers one question — does this value satisfy the type its property declares — and answers
//! it with a reason, because "invalid" that does not say which property and why is not a finding
//! anyone can act on.
//!
//! A `NodeRef` cannot be checked without knowing what the referenced node is, and this crate holds
//! no graph state (`AGENTS.md` invariant 8), so the caller supplies that through [`NodeTypes`].
//! Whether the node *exists* is validator 4's question; what this one refuses is a reference whose
//! target type is not among the declared `allowed_types`, or one whose type the caller cannot
//! name at all — an unnameable target cannot be shown to be allowed, and a checker that assumes
//! the best about what it cannot see is not a checker.

use std::collections::BTreeMap;

use ekr_core::{NodeId, PropertyId, TypeId};

use crate::schema::Ontology;
use crate::value::{Cardinality, Value, ValueKind, ValueType};

/// What the checker needs to know about the graph: the type of a referenced node.
///
/// Implemented here for `BTreeMap<NodeId, TypeId>`, which is what a case or a small caller has; a
/// store implements it over its own index.
pub trait NodeTypes {
    /// The type of this node, or `None` if the caller cannot name it.
    fn type_of(&self, node: NodeId) -> Option<TypeId>;
}

impl NodeTypes for BTreeMap<NodeId, TypeId> {
    fn type_of(&self, node: NodeId) -> Option<TypeId> {
        self.get(&node).copied()
    }
}

/// A refusal: which property, and why.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
#[error("{}: {reason}", match property { Some(id) => format!("property {id}"), None => "the node type".to_owned() })]
pub struct CheckError {
    property: Option<PropertyId>,
    reason: CheckReason,
}

impl CheckError {
    /// The property the refusal is about, or `None` when it is about the type itself.
    #[must_use]
    pub const fn property(&self) -> Option<PropertyId> {
        self.property
    }

    /// Why the value was refused.
    #[must_use]
    pub const fn reason(&self) -> &CheckReason {
        &self.reason
    }
}

/// Why a value does not satisfy its declared type.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum CheckReason {
    /// The ontology declares no such node type.
    #[error("{type_id} is not a declared node type")]
    UnknownType {
        /// The type that was asked for.
        type_id: TypeId,
    },
    /// The type is abstract, and abstract types have no nodes.
    #[error("{type_id} is abstract and has no nodes")]
    AbstractType {
        /// The abstract type.
        type_id: TypeId,
    },
    /// The node carries a property its type does not declare.
    #[error("the type declares no such property")]
    UndeclaredProperty,
    /// A required property carries no value.
    #[error("the property is required and carries no value")]
    MissingRequiredProperty,
    /// More values than the declared cardinality permits.
    #[error("cardinality {cardinality} does not permit {count} values")]
    CardinalityExceeded {
        /// What the property declares.
        cardinality: Cardinality,
        /// How many values were supplied.
        count: usize,
    },
    /// The value has a different kind from the one declared.
    #[error("expected a {expected} and found a {found}")]
    WrongKind {
        /// The declared kind.
        expected: ValueKind,
        /// The kind the value has.
        found: ValueKind,
    },
    /// An `Enum` value that is not one of the declared variants.
    #[error("{variant:?} is not a declared variant")]
    UndeclaredVariant {
        /// The variant the value carried.
        variant: String,
    },
    /// A reference to a node whose type is outside `allowed_types`.
    #[error("{node} is a {node_type}, which is not an allowed type of this reference")]
    NodeRefNotAllowed {
        /// The node referred to.
        node: NodeId,
        /// Its type.
        node_type: TypeId,
    },
    /// A reference to a node whose type the caller cannot name.
    #[error("the type of {node} is unknown, so the reference cannot be shown to be allowed")]
    UnresolvedNodeRef {
        /// The node referred to.
        node: NodeId,
    },
    /// A record value carries a field its type does not declare.
    #[error("the record type declares no field {field:?}")]
    UndeclaredField {
        /// The field the value carried.
        field: String,
    },
    /// A record value is missing a declared field.
    #[error("the record type declares a field {field:?} the value does not carry")]
    MissingField {
        /// The field that is missing.
        field: String,
    },
}

impl Ontology {
    /// Whether these properties satisfy the node type that declares them.
    ///
    /// # Errors
    ///
    /// The first violation found, as a [`CheckError`] naming the property and the reason.
    pub fn check_node(
        &self,
        type_id: TypeId,
        properties: &BTreeMap<PropertyId, Vec<Value>>,
        nodes: &impl NodeTypes,
    ) -> Result<(), CheckError> {
        let Some(declared) = self.node_type(type_id) else {
            return Err(CheckError {
                property: None,
                reason: CheckReason::UnknownType { type_id },
            });
        };
        if declared.abstract_type {
            return Err(CheckError {
                property: None,
                reason: CheckReason::AbstractType { type_id },
            });
        }

        let definitions = self.properties_of(type_id);
        let refuse = |property: PropertyId, reason: CheckReason| CheckError {
            property: Some(property),
            reason,
        };

        for (property, values) in properties {
            let Some(definition) = definitions.get(property) else {
                return Err(refuse(*property, CheckReason::UndeclaredProperty));
            };
            if !definition.cardinality.permits(values.len()) {
                return Err(refuse(
                    *property,
                    CheckReason::CardinalityExceeded {
                        cardinality: definition.cardinality,
                        count: values.len(),
                    },
                ));
            }
            for value in values {
                self.check_value(value, &definition.value_type, nodes)
                    .map_err(|reason| refuse(*property, reason))?;
            }
        }

        // Presence is the property definition's question, not the supplied bag's: an absent entry
        // and an entry carrying no values are the same absence.
        for (property, definition) in &definitions {
            if definition.required
                && !properties
                    .get(property)
                    .is_some_and(|values| !values.is_empty())
            {
                return Err(refuse(*property, CheckReason::MissingRequiredProperty));
            }
        }

        Ok(())
    }

    /// Whether one value satisfies one declared type.
    ///
    /// # Errors
    ///
    /// The first violation found, as a [`CheckReason`]. There is no property to name here; the
    /// caller that has one wraps it.
    pub fn check_value(
        &self,
        value: &Value,
        value_type: &ValueType,
        nodes: &impl NodeTypes,
    ) -> Result<(), CheckReason> {
        // The kind is checked once, for every shape, so that a kind added to design § 11.3 cannot
        // arrive with a match arm that silently accepts it.
        let (expected, found) = (value_type.kind(), value.kind());
        if expected != found {
            return Err(CheckReason::WrongKind { expected, found });
        }

        match (value_type, value) {
            (ValueType::NodeRef { allowed_types }, Value::NodeRef(node)) => {
                let Some(node_type) = nodes.type_of(*node) else {
                    return Err(CheckReason::UnresolvedNodeRef { node: *node });
                };
                if allowed_types
                    .iter()
                    .any(|allowed| self.conforms_to(node_type, *allowed))
                {
                    Ok(())
                } else {
                    Err(CheckReason::NodeRefNotAllowed {
                        node: *node,
                        node_type,
                    })
                }
            }
            (ValueType::Enum { variants }, Value::Enum(variant)) => {
                if variants.contains(variant) {
                    Ok(())
                } else {
                    Err(CheckReason::UndeclaredVariant {
                        variant: variant.clone(),
                    })
                }
            }
            (ValueType::List(element), Value::List(items)) => items
                .iter()
                .try_for_each(|item| self.check_value(item, element, nodes)),
            (ValueType::Record(fields), Value::Record(carried)) => {
                for (field, value) in carried {
                    let Some(declared) = fields.get(field) else {
                        return Err(CheckReason::UndeclaredField {
                            field: field.clone(),
                        });
                    };
                    self.check_value(value, declared, nodes)?;
                }
                for field in fields.keys() {
                    if !carried.contains_key(field) {
                        return Err(CheckReason::MissingField {
                            field: field.clone(),
                        });
                    }
                }
                Ok(())
            }
            // A scalar carries no parameters, so its kind was the whole of its type.
            _ => Ok(()),
        }
    }
}
