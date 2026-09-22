//! The frozen `legacy::Assertion::status` answers design § 36's removal question from the
//! validation state alone, as its documentation says: `Retracted` and `Superseded` map to
//! themselves with their payload intact, and every other validation state is `Active`.

use std::collections::BTreeSet;

use ekr_core::{
    AgentId, AssertionId, EvidenceId, GraphRootId, IssueId, NodeId, PropertyId, RevisionNumber,
    Timestamp,
};
use ekr_graph::legacy::{
    Assertion, AssertionStatus, Object, Predicate, RetractionReason, Subject, TemporalRange,
    TransactionTime, ValidationState, Value,
};

fn assertion(validation: ValidationState) -> Assertion {
    Assertion {
        id: AssertionId::mint(),
        root_id: GraphRootId::mint(),
        subject: Subject::Node(NodeId::mint()),
        predicate: Predicate::Property(PropertyId::mint()),
        object: Object::Value(Value::String("frozen".into())),
        evidence: BTreeSet::from([EvidenceId::mint()]),
        proposed_by: AgentId::mint(),
        validation,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}

/// One position per frozen variant. The match has no wildcard, so a variant this table does not
/// hold cannot compile past it.
fn variant(state: &ValidationState) -> usize {
    match state {
        ValidationState::Proposed => 0,
        ValidationState::Validating { .. } => 1,
        ValidationState::Accepted { .. } => 2,
        ValidationState::Rejected { .. } => 3,
        ValidationState::Disputed { .. } => 4,
        ValidationState::Superseded { .. } => 5,
        ValidationState::Retracted { .. } => 6,
    }
}

#[test]
fn every_frozen_validation_state_maps_to_the_documented_status() {
    let replacement = AssertionId::mint();
    let at_revision = RevisionNumber::new(3);
    let reason = RetractionReason::new("withdrawn by its source");
    let table: Vec<(ValidationState, AssertionStatus)> = vec![
        (ValidationState::Proposed, AssertionStatus::Active),
        (
            ValidationState::Validating {
                completed: 1,
                required: 2,
            },
            AssertionStatus::Active,
        ),
        (
            ValidationState::Accepted {
                validators: BTreeSet::from([AgentId::mint()]),
            },
            AssertionStatus::Active,
        ),
        (
            ValidationState::Rejected {
                issues: vec![IssueId::mint()],
            },
            AssertionStatus::Active,
        ),
        (
            ValidationState::Disputed {
                competing_assertions: vec![AssertionId::mint()],
            },
            AssertionStatus::Active,
        ),
        (
            ValidationState::Superseded { by: replacement },
            AssertionStatus::Superseded { by: replacement },
        ),
        (
            ValidationState::Retracted {
                at_revision,
                reason: reason.clone(),
            },
            AssertionStatus::Retracted {
                at_revision,
                reason: reason.clone(),
            },
        ),
    ];

    let covered: BTreeSet<usize> = table.iter().map(|(state, _)| variant(state)).collect();
    assert_eq!(
        covered,
        (0..7).collect::<BTreeSet<_>>(),
        "a frozen variant has no row"
    );

    let mut wrong = Vec::new();
    for (state, expected) in &table {
        let actual = assertion(state.clone()).status();
        if actual != *expected {
            wrong.push(format!(
                "{}: {actual:?}, expected {expected:?}",
                state.name()
            ));
        }
    }
    assert!(wrong.is_empty(), "status mapped wrongly: {wrong:?}");
}

#[test]
fn removal_statuses_carry_exactly_the_payload_of_their_validation_state() {
    let first = AssertionId::mint();
    let second = AssertionId::mint();
    assert_ne!(
        assertion(ValidationState::Superseded { by: first }).status(),
        assertion(ValidationState::Superseded { by: second }).status()
    );
    let AssertionStatus::Superseded { by } =
        assertion(ValidationState::Superseded { by: second }).status()
    else {
        panic!("a superseded assertion is not reported as superseded")
    };
    assert_eq!(by, second);

    let retracted = assertion(ValidationState::Retracted {
        at_revision: RevisionNumber::new(9),
        reason: RetractionReason::new("source corrected"),
    });
    let AssertionStatus::Retracted {
        at_revision,
        reason,
    } = retracted.status()
    else {
        panic!("a retracted assertion is not reported as retracted")
    };
    assert_eq!(at_revision, RevisionNumber::new(9));
    assert_eq!(reason.as_str(), "source corrected");
    assert_ne!(
        retracted.status(),
        assertion(ValidationState::Retracted {
            at_revision: RevisionNumber::new(10),
            reason: RetractionReason::new("source corrected"),
        })
        .status()
    );
}

#[test]
fn active_says_nothing_about_acceptance_or_belief_time() {
    // `Active` is "not withdrawn and not replaced", read from the validation state only: a record
    // the runtime stopped believing, or one never accepted, is still not removed.
    let mut withdrawn_belief = assertion(ValidationState::Accepted {
        validators: BTreeSet::from([AgentId::mint()]),
    });
    withdrawn_belief.transaction_time =
        TransactionTime::new(Timestamp::EPOCH, Some(Timestamp::from_millis(1))).unwrap();
    assert!(!withdrawn_belief.is_current());
    assert_eq!(withdrawn_belief.status(), AssertionStatus::Active);

    let never_accepted = assertion(ValidationState::Proposed);
    assert!(!never_accepted.is_current());
    assert_eq!(never_accepted.status(), AssertionStatus::Active);

    let current = assertion(ValidationState::Accepted {
        validators: BTreeSet::from([AgentId::mint()]),
    });
    assert!(current.is_current());
    assert_eq!(current.status(), AssertionStatus::Active);
}
