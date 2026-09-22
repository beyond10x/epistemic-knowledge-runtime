//! Per-type lifecycles and named operations: design amendment 87.
//!
//! The original ontology gives a node type properties and constraints. What refuses a wrong move
//! is a type with a lifecycle: a decision that is `open` may become `decided`; a `decided`
//! decision may not become `moot` through a generic property update. A node's lifecycle state is a
//! typed property `state: Enum` over the lifecycle's states, and a named operation is the only
//! thing that moves it.
//!
//! This is the world's lifecycle — whether a decision is decided — and is orthogonal to
//! `KnowledgeState` (design § 29), which is about whether the runtime has integrated the node.
//!
//! An operation's `preconditions` are carried as opaque text. Design § 11.2 names `Constraint` and
//! does not define it, and `systems/ekr/domains/ontology.yaml` marks the language `UNMAPPED`; a
//! constraint language decided in passing here would be a guess with a schema behind it, so
//! nothing in this module evaluates a precondition. The kernel refuses an invocation carrying
//! one, held by `opaque_preconditions_and_emissions_are_not_silently_accepted` in its suite.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::value::ValueType;

/// One declared move of a lifecycle: `ekr.ontology.Transition` of
/// `systems/ekr/domains/ontology.yaml`.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Transition {
    /// The state a node must be in for this move.
    pub from: String,
    /// The state it is in afterwards.
    pub to: String,
}

impl Transition {
    /// A move from one state to another.
    #[must_use]
    pub fn new(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
        }
    }
}

/// The lifecycle of a node type: amendment 87.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lifecycle {
    /// The state a newly created node of this type is in. Must be one of `states`.
    pub initial: String,
    /// Every state a node of this type may be in.
    pub states: BTreeSet<String>,
    /// Every move a node of this type may make. A pair not here is not a move.
    #[serde(default)]
    pub transitions: BTreeSet<Transition>,
}

impl Lifecycle {
    /// Whether this lifecycle declares that move.
    ///
    /// The pair is the unit, not the endpoints: two states this lifecycle has are not thereby
    /// connected in either direction.
    #[must_use]
    pub fn declares(&self, transition: &Transition) -> bool {
        self.transitions.contains(transition)
    }

    /// The state a node in `from` is in after `operation`, or why the operation is refused.
    ///
    /// An operation that declares no transition is a property update and leaves the state alone —
    /// which is exactly why moving state through one is not possible.
    ///
    /// # Errors
    ///
    /// [`LifecycleError::UnknownState`] if `from` is not a state of this lifecycle,
    /// [`LifecycleError::NotInState`] if the operation moves a node in a different state, and
    /// [`LifecycleError::TransitionNotDeclared`] if the move itself is not declared.
    pub fn transition(
        &self,
        from: &str,
        operation: &OperationDefinition,
    ) -> Result<String, LifecycleError> {
        if !self.states.contains(from) {
            return Err(LifecycleError::UnknownState {
                state: from.to_owned(),
            });
        }
        let Some(declared) = &operation.transition else {
            return Ok(from.to_owned());
        };
        if declared.from != from {
            return Err(LifecycleError::NotInState {
                expected: declared.from.clone(),
                actual: from.to_owned(),
            });
        }
        if !self.declares(declared) {
            return Err(LifecycleError::TransitionNotDeclared {
                transition: declared.clone(),
            });
        }
        Ok(declared.to.clone())
    }
}

/// A named operation on a node type: amendment 87.
///
/// `GraphOperation::Invoke` names one, and the ontology-constraint validator (design § 20, item 5)
/// refuses an invocation whose transition is not declared.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationDefinition {
    /// The operation's name, as `GraphOperation::Invoke` names it.
    pub name: String,
    /// The arguments it takes, by name, each typed.
    #[serde(default)]
    pub arguments: BTreeMap<String, ValueType>,
    /// Constraint expressions over the node's properties and the arguments. The language is
    /// `UNMAPPED`; these are carried and nothing here refuses one.
    #[serde(default)]
    pub preconditions: Vec<String>,
    /// The move it makes, if it makes one.
    #[serde(default)]
    pub transition: Option<Transition>,
    /// The event types it emits.
    #[serde(default)]
    pub emits: Vec<String>,
}

impl OperationDefinition {
    /// An operation that takes no arguments, moves nothing and emits nothing.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            arguments: BTreeMap::new(),
            preconditions: Vec::new(),
            transition: None,
            emits: Vec::new(),
        }
    }
}

/// Why a lifecycle refused a move.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum LifecycleError {
    /// The node is in a state this lifecycle does not have.
    #[error("{state:?} is not a state of this lifecycle")]
    UnknownState {
        /// The state that was asked about.
        state: String,
    },
    /// The operation moves a node in another state.
    #[error("the operation moves a node in {expected:?}, and this node is in {actual:?}")]
    NotInState {
        /// The state the operation's transition starts from.
        expected: String,
        /// The state the node is actually in.
        actual: String,
    },
    /// The lifecycle does not declare that move.
    #[error("{:?} -> {:?} is not a declared transition", transition.from, transition.to)]
    TransitionNotDeclared {
        /// The move that was refused.
        transition: Transition,
    },
}
