//! `Assertion` round-trips through serde, with every optional field present and absent.
//!
//! Not in the story's test list. `RevisionEvent` had a round-trip case and `Assertion` did not,
//! and `Assertion` is what `ekr-store` persists in the next wave — a record that does not read
//! back is a canonical history that cannot be replayed, which is the one thing design § 34 exists
//! to guarantee. Adversary pass 2 wrote this case, found it green and deleted it under its own
//! rule; the coordinator asked for it to be kept, because a property that holds today and is
//! checked by nothing is a property the next change breaks silently.
//!
//! It lives in `ekr` for the reason `graph_events_serde.rs` does: a round trip needs a *format*,
//! `ekr-graph` declares none, and `ekr` is the one crate with both `ekr-graph` and `serde_json`.
//!
//! Three things it is really about:
//!
//! * **`Option` fields with no `#[serde(default)]`.** An absent `valid_from` must read back absent
//!   rather than as an error or a zero.
//! * **The sum types.** `Subject`, `Predicate`, `Object` and the separate assessment/lifecycle
//!   variants, retaining assessment payloads through every withdrawal state.
//! * **The refusals the constructors make.** `TemporalRange`, `TransactionTime` and
//!   `CanonicalValue` deserialise through `TryFrom`, so a document carrying an inverted range — or
//!   a float, which canonical state does not admit — is refused at the same boundary a Rust caller
//!   is. Otherwise the invariant holds only against callers who use the constructor.

use std::collections::BTreeSet;

use ekr_core::{
    AgentId, AssertionId, EdgeId, EvidenceId, GraphRootId, IssueId, NodeId, PropertyId,
    RevisionNumber, Timestamp, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalRef, CanonicalValue, Object, Predicate,
    RetractionReason, Subject, TemporalRange, TransactionTime,
};
use ekr_ontology::Value;

/// 2024-01-01T00:00:00Z.
const RECORDED: Timestamp = Timestamp::from_millis(1_704_067_200_000);
/// 2026-03-12T00:00:00Z.
const HANDOVER: Timestamp = Timestamp::from_millis(1_773_273_600_000);

fn assertion(
    subject: Subject,
    predicate: Predicate,
    object: Object,
    assessment: Assessment,
    valid_time: TemporalRange,
    transaction_time: TransactionTime,
) -> Assertion {
    Assertion {
        id: AssertionId::mint(),
        root_id: GraphRootId::mint(),
        subject,
        predicate,
        object,
        evidence: BTreeSet::from([
            CanonicalRef::new(EvidenceId::mint()),
            CanonicalRef::new(EvidenceId::mint()),
        ]),
        proposed_by: AgentId::mint(),
        assessment,
        lifecycle: AssertionLifecycle::Active,
        valid_time,
        transaction_time,
    }
}

/// Every assessment and lifecycle, with all subject/predicate/object arms represented and
/// both shapes of each temporal field.
fn every_shape() -> Vec<Assertion> {
    let subjects = [
        Subject::Node(CanonicalRef::new(NodeId::mint())),
        Subject::Edge(CanonicalRef::new(EdgeId::mint())),
        Subject::Type(TypeId::mint()),
    ];
    let predicates = [
        Predicate::Property(PropertyId::mint()),
        Predicate::Relation(TypeId::mint()),
    ];
    let objects = [
        Object::Value(
            CanonicalValue::try_from(Value::String("Acme".to_owned())).expect("admissible"),
        ),
        Object::Value(CanonicalValue::Timestamp(HANDOVER)),
        Object::Node(CanonicalRef::new(NodeId::mint())),
        Object::Type(TypeId::mint()),
    ];
    let assessments = [
        Assessment::Proposed,
        Assessment::Validating {
            completed: 1,
            required: 3,
        },
        Assessment::Accepted {
            validators: [AgentId::mint(), AgentId::mint()].into_iter().collect(),
        },
        Assessment::Accepted {
            // An empty collection is not the same as an absent one, and serde must agree.
            validators: BTreeSet::new(),
        },
        Assessment::Rejected {
            issues: vec![IssueId::mint()],
        },
        Assessment::Rejected { issues: Vec::new() },
        Assessment::Disputed {
            competing_assertions: vec![
                CanonicalRef::new(AssertionId::mint()),
                CanonicalRef::new(AssertionId::mint()),
            ],
        },
    ];
    let lifecycles = [
        AssertionLifecycle::Active,
        AssertionLifecycle::Superseded {
            by: CanonicalRef::new(AssertionId::mint()),
            at_revision: RevisionNumber::new(9),
            effective_from: HANDOVER,
        },
        AssertionLifecycle::Retracted {
            at_revision: RevisionNumber::new(9),
            reason: RetractionReason::new("the evidence was another Acme"),
        },
    ];
    // Both bounds of valid time, present and absent, in every combination that is a value.
    let valid_times = [
        TemporalRange::UNBOUNDED,
        TemporalRange::since(RECORDED),
        TemporalRange::new(None, Some(HANDOVER)).expect("not inverted"),
        TemporalRange::new(Some(RECORDED), Some(HANDOVER)).expect("not inverted"),
        TemporalRange::new(Some(HANDOVER), Some(HANDOVER)).expect("the empty interval"),
    ];
    let transaction_times = [
        TransactionTime::since(RECORDED),
        TransactionTime::new(RECORDED, Some(HANDOVER)).expect("not inverted"),
    ];

    let mut built = Vec::new();
    for (position, assessment) in assessments.iter().enumerate() {
        for lifecycle in &lifecycles {
            for valid_time in valid_times {
                for transaction_time in transaction_times {
                    let mut record = assertion(
                        subjects[position % subjects.len()],
                        predicates[position % predicates.len()],
                        objects[position % objects.len()].clone(),
                        assessment.clone(),
                        valid_time,
                        transaction_time,
                    );
                    record.lifecycle = lifecycle.clone();
                    built.push(record);
                }
            }
        }
    }
    built
}

#[test]
fn every_shape_of_assertion_round_trips_through_serde() {
    let population = every_shape();
    assert_eq!(population.len(), 210, "the cross product lost a shape");

    for record in &population {
        let json = serde_json::to_string(record)
            .unwrap_or_else(|e| panic!("{:?} does not serialise: {e}", record.assessment.name()));
        let back: Assertion = serde_json::from_str(&json).unwrap_or_else(|e| {
            panic!(
                "{} does not read back: {e} from {json}",
                record.assessment.name()
            )
        });
        assert_eq!(&back, record, "an assertion changed across the round trip");
    }
}

/// An absent optional field reads back absent, and is absent on the wire.
///
/// The failure this guards is `#[serde(default)]` arriving on a bound: a `valid_from` that reads
/// back as the Unix epoch instead of `None` turns "nobody knows when this became true" into "this
/// became true in 1970", and every historical read then answers differently.
#[test]
fn an_absent_bound_stays_absent() {
    let record = assertion(
        Subject::Node(CanonicalRef::new(NodeId::mint())),
        Predicate::Relation(TypeId::mint()),
        Object::Node(CanonicalRef::new(NodeId::mint())),
        Assessment::Proposed,
        TemporalRange::UNBOUNDED,
        TransactionTime::since(RECORDED),
    );

    let value: serde_json::Value = serde_json::to_value(&record).expect("an assertion serialises");
    let valid_time = value.get("valid_time").expect("valid_time is a field");
    assert_eq!(valid_time.get("from"), Some(&serde_json::Value::Null));
    assert_eq!(valid_time.get("to"), Some(&serde_json::Value::Null));
    assert_eq!(
        value
            .get("transaction_time")
            .and_then(|t| t.get("recorded_to")),
        Some(&serde_json::Value::Null),
        "an open transaction time has no end, and says so"
    );

    let back: Assertion = serde_json::from_value(value).expect("and reads back");
    assert_eq!(back.valid_time.from, None);
    assert_eq!(back.valid_time.to, None);
    assert_eq!(back.transaction_time.recorded_to, None);
    assert_eq!(back.transaction_time.recorded_from, RECORDED);
    assert_eq!(back, record);
}

/// A document carrying an inverted range is refused, not read.
///
/// The constructors refuse `to < from`, and serde is the other construction path. Without
/// `#[serde(try_from = …)]` the invariant would hold against Rust callers and not against the
/// persisted records read back through the same boundary.
#[test]
fn serde_refuses_an_inverted_range_the_way_the_constructor_does() {
    let inverted_valid_time = serde_json::json!({
        "from": HANDOVER.millis(),
        "to": RECORDED.millis(),
    });
    let refused = serde_json::from_value::<TemporalRange>(inverted_valid_time)
        .expect_err("an inverted valid time must be refused");
    assert!(
        refused.to_string().contains("precedes"),
        "the refusal must say what is wrong: {refused}"
    );

    let inverted_belief = serde_json::json!({
        "recorded_from": HANDOVER.millis(),
        "recorded_to": RECORDED.millis(),
    });
    let refused = serde_json::from_value::<TransactionTime>(inverted_belief)
        .expect_err("a belief withdrawn before it was formed must be refused");
    assert!(refused.to_string().contains("precedes"), "{refused}");

    // And the boundary the refusal must not cross: the empty interval reads back.
    let empty = serde_json::json!({ "from": HANDOVER.millis(), "to": HANDOVER.millis() });
    assert_eq!(
        serde_json::from_value::<TemporalRange>(empty).expect("[t, t) is a value"),
        TemporalRange::new(Some(HANDOVER), Some(HANDOVER)).expect("the empty interval")
    );
}

/// A document carrying a float where canonical state admits none is refused, not read.
///
/// The same reasoning as the case above, for the refusal
/// `architecture-decision-record:0005-float-is-not-canonical` makes: `Object::Value` carries a
/// `CanonicalValue`, so a Rust caller cannot build a float-valued assertion — and serde is the
/// other construction path. Without it the type would hold against callers and not against the
/// store reading records back, which in P1 is the caller that will exist first.
///
/// It is also where the wire shape is pinned: a canonical value serialises as the
/// `ekr_ontology::Value` it came from, so this type existing moved no bytes.
#[test]
fn serde_refuses_a_float_where_canonical_state_admits_none() {
    let record = assertion(
        Subject::Node(CanonicalRef::new(NodeId::mint())),
        Predicate::Property(PropertyId::mint()),
        Object::Value(CanonicalValue::Decimal("0.1".to_owned())),
        Assessment::Proposed,
        TemporalRange::UNBOUNDED,
        TransactionTime::since(RECORDED),
    );

    let mut document: serde_json::Value =
        serde_json::to_value(&record).expect("an assertion serialises");
    assert_eq!(
        document.get("object"),
        Some(&serde_json::json!({ "Value": { "value_kind": "Decimal", "value": "0.1" } })),
        "a canonical value is on the wire as the ekr_ontology::Value it came from"
    );

    document["object"] = serde_json::json!({ "Value": { "value_kind": "Float", "value": 0.1 } });
    let refused = serde_json::from_value::<Assertion>(document)
        .expect_err("a float does not enter canonical state through a document either");
    assert!(
        refused.to_string().contains("Float"),
        "the refusal must say what is wrong: {refused}"
    );

    // The kind that carries the same quantity exactly does read back.
    let mut document: serde_json::Value =
        serde_json::to_value(&record).expect("an assertion serialises");
    document["object"] = serde_json::json!({ "Value": { "value_kind": "Integer", "value": 3 } });
    let back: Assertion = serde_json::from_value(document).expect("an integer is admissible");
    assert_eq!(back.object, Object::Value(CanonicalValue::Integer(3)));
}

#[test]
fn current_assertions_require_separate_assessment_and_lifecycle() {
    let record = every_shape().remove(0);
    let value = serde_json::to_value(&record).unwrap();
    assert!(value.get("validation").is_none());
    for field in ["assessment", "lifecycle"] {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<Assertion>(missing).is_err(),
            "the current assertion omitted {field}"
        );
    }
    let mut legacy = value;
    let object = legacy.as_object_mut().unwrap();
    object.remove("assessment");
    object.remove("lifecycle");
    object.insert("validation".into(), serde_json::json!("Proposed"));
    assert!(serde_json::from_value::<Assertion>(legacy).is_err());
}
