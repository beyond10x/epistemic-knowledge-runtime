//! `Lifecycle::transition(from, operation)` refuses a move the type does not declare and accepts
//! every declared one — the third of the story's shipped tests, and design amendment 87's point:
//! "a `decided` decision may not become `moot` through a generic property update".
//!
//! The quantification is over *every* ordered pair of states, not over a sample: the case asserts
//! `Ok` exactly on the declared pairs and `Err` on all the others, so a `transition` that answers
//! `Ok` to everything and one that answers `Err` to everything both die here.

use std::collections::BTreeSet;

use ekr_ontology::{Lifecycle, LifecycleError, OperationDefinition, Transition};
use proptest::prelude::*;

/// The lifecycle of a decision, as amendment 87 describes it.
fn decision_lifecycle() -> Lifecycle {
    Lifecycle {
        initial: "open".to_owned(),
        states: ["open", "decided", "moot"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        transitions: [("open", "decided"), ("open", "moot"), ("decided", "moot")]
            .into_iter()
            .map(|(from, to)| Transition::new(from, to))
            .collect(),
    }
}

/// An operation that declares exactly this move.
fn operation(from: &str, to: &str) -> OperationDefinition {
    let mut declared = OperationDefinition::new(format!("{from}_to_{to}"));
    declared.transition = Some(Transition::new(from, to));
    declared
}

#[test]
fn every_declared_transition_is_accepted() {
    let lifecycle = decision_lifecycle();
    for declared in &lifecycle.transitions {
        assert_eq!(
            lifecycle.transition(&declared.from, &operation(&declared.from, &declared.to)),
            Ok(declared.to.clone()),
            "{declared:?} is declared"
        );
        assert!(lifecycle.declares(declared));
    }
    assert_eq!(lifecycle.transitions.len(), 3, "three declared moves");
}

proptest! {
    /// Over every ordered pair of the lifecycle's states: accepted exactly when declared.
    #[test]
    fn a_move_is_accepted_exactly_when_the_lifecycle_declares_it(
        from in prop::sample::select(vec!["open", "decided", "moot"]),
        to in prop::sample::select(vec!["open", "decided", "moot"]),
    ) {
        let lifecycle = decision_lifecycle();
        let move_ = Transition::new(from, to);
        let declared = lifecycle.transitions.contains(&move_);

        let outcome = lifecycle.transition(from, &operation(from, to));
        prop_assert_eq!(
            outcome.is_ok(),
            declared,
            "{:?} is declared={}, and transition answered {:?}",
            move_,
            declared,
            lifecycle.transition(from, &operation(from, to))
        );
        if !declared {
            prop_assert_eq!(
                lifecycle.transition(from, &operation(from, to)),
                Err(LifecycleError::TransitionNotDeclared { transition: move_ })
            );
        }
    }
}

#[test]
fn a_move_from_a_state_the_node_is_not_in_is_refused() {
    let lifecycle = decision_lifecycle();
    let refused = lifecycle
        .transition("decided", &operation("open", "decided"))
        .expect_err("the node is decided; the operation moves an open one");
    assert_eq!(
        refused,
        LifecycleError::NotInState {
            expected: "open".to_owned(),
            actual: "decided".to_owned(),
        }
    );
    assert!(
        refused.to_string().contains("decided"),
        "the refusal reaches a reader: {refused}"
    );
}

#[test]
fn a_move_from_a_state_the_lifecycle_does_not_have_is_refused() {
    let lifecycle = decision_lifecycle();
    assert_eq!(
        lifecycle.transition("archived", &operation("archived", "moot")),
        Err(LifecycleError::UnknownState {
            state: "archived".to_owned()
        })
    );
}

#[test]
fn an_operation_that_declares_no_transition_leaves_the_state_alone() {
    let lifecycle = decision_lifecycle();
    let annotate = OperationDefinition::new("annotate");
    assert_eq!(annotate.transition, None);
    assert_eq!(
        lifecycle.transition("decided", &annotate),
        Ok("decided".to_owned()),
        "a property update is not a lifecycle move"
    );
}

/// `declares` answers for a transition the lifecycle does not have, including one between two
/// states it does have — the pair is the unit, not the endpoints.
#[test]
fn declares_answers_for_the_pair_and_not_for_its_endpoints() {
    let lifecycle = decision_lifecycle();
    assert!(lifecycle.declares(&Transition::new("open", "decided")));
    assert!(!lifecycle.declares(&Transition::new("decided", "open")));
    assert!(!lifecycle.declares(&Transition::new("moot", "open")));
    assert!(!lifecycle.declares(&Transition::new("open", "archived")));
}

/// An operation's arguments are typed, and its preconditions are carried as opaque text: design
/// § 11.2 names `Constraint` and does not define it, and `systems/ekr/domains/ontology.yaml`
/// marks the language `UNMAPPED`. Nothing here refuses one.
#[test]
fn preconditions_are_carried_as_opaque_text_and_refuse_nothing() {
    let mut decide = OperationDefinition::new("decide");
    decide.transition = Some(Transition::new("open", "decided"));
    decide.preconditions = vec!["not a constraint language".to_owned()];
    decide
        .arguments
        .insert("rationale".to_owned(), ekr_ontology::ValueType::String);
    decide.emits = vec!["DecisionMade".to_owned()];

    let lifecycle = decision_lifecycle();
    assert_eq!(
        lifecycle.transition("open", &decide),
        Ok("decided".to_owned()),
        "an undefined constraint language refuses nothing"
    );
    assert_eq!(decide.preconditions.len(), 1);
    assert_eq!(decide.arguments.len(), 1);
    assert_eq!(decide.emits, vec!["DecisionMade".to_owned()]);
    assert_eq!(decide.name, "decide");
}

/// A lifecycle's own shape: `initial` is one of `states`, and the state set is what a `state:
/// Enum` property would range over (amendment 87).
#[test]
fn a_lifecycle_names_its_initial_state_among_its_states() {
    let lifecycle = decision_lifecycle();
    assert!(lifecycle.states.contains(&lifecycle.initial));
    assert_eq!(
        lifecycle.states,
        ["open", "decided", "moot"]
            .into_iter()
            .map(str::to_owned)
            .collect::<BTreeSet<String>>()
    );
}
