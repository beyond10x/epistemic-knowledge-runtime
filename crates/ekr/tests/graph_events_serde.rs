//! `RevisionEvent` round-trips through serde, for every variant.
//!
//! One of `story:graph-model-and-assertions`'s "Tests the story ships". The event is the thing
//! `ekr-store` persists and replays, so a variant that does not survive a round trip is a revision
//! history that cannot be read back — and a `#[serde(tag = …)]` that names a field one of the
//! variants also has is exactly the kind of mistake that shows up only here.
//!
//! # Why the case lives in `ekr`
//!
//! A round trip needs a *format*, and `ekr-graph` declares none: `story:workspace-crate-skeleton`
//! fixed its dependencies at `ekr-core`, `ekr-ontology`, `serde` and `thiserror`, with `trybuild`
//! as its only dev-dependency, and `crates/ekr/tests/story_contract.rs` holds the manifest to that
//! list. `ekr` is the one crate that has both `ekr-graph` and `serde_json`, so the case is here
//! rather than behind a manifest change this unit does not own. The alternative — adding
//! `serde_json` to `ekr-graph`'s dev-dependencies — is a story amendment and a coordinator's
//! change, and is named in this unit's report.

use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, TransactionId};
use ekr_graph::{RevisionEvent, RevisionPayload};

/// One value of each variant, with distinguishable payloads.
fn every_variant() -> Vec<RevisionEvent> {
    let transaction_id = TransactionId::mint();
    let revision_id = RevisionId::mint();
    // Equal envelope coordinates ensure payload distinctions do the work in the wire test.
    let event_id = EventId::mint();
    let record_hash = ContentHash::of_bytes(b"retained decision");

    vec![
        RevisionPayload::Seeded {
            revision_id,
            seed_hash: ContentHash::of_bytes(b"seed"),
        },
        RevisionPayload::TransactionProposed {
            transaction_id,
            proposer: AgentId::mint(),
            operations_hash: Some(ContentHash::of_bytes(b"operations")),
        },
        RevisionPayload::TransactionValidated {
            transaction_id,
            against: RevisionNumber::new(11),
            validation_hash: ContentHash::of_bytes(b"validation"),
        },
        RevisionPayload::TransactionRejected {
            transaction_id,
            issues: 2,
        },
        RevisionPayload::TransactionStale {
            transaction_id,
            validated_against: RevisionNumber::new(11),
            current: RevisionNumber::new(12),
        },
        RevisionPayload::RevisionCommitted {
            transaction_id,
            revision_id,
            number: RevisionNumber::new(12),
            knowledge_root: ContentHash::of_bytes(b"knowledge"),
        },
    ]
    .into_iter()
    .map(|payload| RevisionEvent {
        format: RevisionEvent::FORMAT.to_owned(),
        event_id,
        record_hash,
        payload,
    })
    .collect()
}

#[test]
fn every_revision_event_variant_round_trips_through_serde() {
    let events = every_variant();
    assert_eq!(events.len(), 6, "a variant has no fixture");

    for event in &events {
        let json = serde_json::to_string(event)
            .unwrap_or_else(|e| panic!("{} does not serialise: {e}", event.name()));
        let back: RevisionEvent = serde_json::from_str(&json)
            .unwrap_or_else(|e| panic!("{} does not read back: {e} from {json}", event.name()));
        assert_eq!(
            &back,
            event,
            "{} changed across the round trip",
            event.name()
        );
    }
}

/// The wire form names the variant, so a stored event says what it is without a reader having to
/// guess from which fields are present.
#[test]
fn the_wire_form_names_the_variant() {
    for event in every_variant() {
        let value: serde_json::Value =
            serde_json::to_value(&event).expect("a revision event serialises");
        let tag = value
            .get("payload")
            .and_then(|payload| payload.get("event"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_else(|| panic!("no variant tag in {value}"));
        assert!(
            event.name().ends_with(tag),
            "the wire tag {tag:?} does not match the domain name {}",
            event.name()
        );
    }
}

/// Two variants are not confusable on the wire either.
#[test]
fn no_two_variants_share_a_wire_form() {
    let mut wire: Vec<String> = every_variant()
        .iter()
        .map(|event| serde_json::to_string(event).expect("a revision event serialises"))
        .collect();
    let count = wire.len();
    wire.sort();
    wire.dedup();
    assert_eq!(wire.len(), count, "two revision events share a wire form");
}

#[test]
fn the_current_envelope_requires_and_preserves_its_coordinates() {
    let event = every_variant().remove(0);
    let value = serde_json::to_value(&event).unwrap();
    assert_eq!(value["format"], "ekr.revision-event/2");
    assert_eq!(value["event_id"], event.event_id.to_string());
    assert_eq!(value["record_hash"], event.record_hash.to_string());
    for field in ["format", "event_id", "record_hash", "payload"] {
        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<RevisionEvent>(missing).is_err(),
            "a current occurrence omitted {field}"
        );
        let repeated = format!(
            "{{\"{field}\":{},{}",
            value[field],
            serde_json::to_string(&event)
                .unwrap()
                .strip_prefix('{')
                .unwrap()
        );
        assert!(
            serde_json::from_str::<RevisionEvent>(&repeated).is_err(),
            "a current occurrence repeated {field}"
        );
    }
    let mut unknown = value;
    unknown["unknown"] = serde_json::Value::Bool(true);
    assert!(serde_json::from_value::<RevisionEvent>(unknown).is_err());
}

#[test]
fn a_noncanonical_proposal_keeps_its_absent_operations_hash() {
    let mut event = every_variant().remove(1);
    let RevisionPayload::TransactionProposed {
        operations_hash, ..
    } = &mut event.payload
    else {
        panic!("the proposal fixture changed position")
    };
    *operations_hash = None;
    let value = serde_json::to_value(&event).unwrap();
    assert_eq!(value["payload"]["operations_hash"], serde_json::Value::Null);
    assert_eq!(
        serde_json::from_value::<RevisionEvent>(value).unwrap(),
        event
    );
}
