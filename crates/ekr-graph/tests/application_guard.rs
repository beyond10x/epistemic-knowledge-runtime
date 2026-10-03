//! Guarded application envelopes preserve old bytes and reject malformed physical links.
use ekr_core::{
    canonical::Canonical, contract_data as w, AgentId, ContentHash, EventId, TransactionId,
};
use ekr_graph::{events::ApplicationGuard, RevisionEvent, RevisionPayload};
use serde_json::json;

fn guard(transaction: TransactionId, cursor: i64) -> w::EkrIntegrateApplicationPublicationGuard {
    let id = AgentId::mint();
    let hash = ContentHash::of_bytes(b"synthetic reviewed application");
    serde_json::from_value(json!({"application_id":id,"step_election_id":id,"proposal_id":id,"proposal_digest":hash,"review_id":id,"human_proof_digest":hash,"review_stream_version":cursor,"step":{"kind":"Schema"},"attempt_transaction":transaction})).unwrap()
}

#[test]
fn guarded_ordinary_format_is_closed_and_old_hash_input_is_unchanged() {
    let transaction = TransactionId::mint();
    let payload = RevisionPayload::TransactionProposed {
        transaction_id: transaction,
        proposer: AgentId::mint(),
        operations_hash: None,
    };
    let mut event = RevisionEvent {
        format: RevisionEvent::FORMAT.into(),
        event_id: EventId::mint(),
        record_hash: ContentHash::of_bytes(b"record"),
        payload,
        application: None,
    };
    let old = event.canonical_bytes();
    let old_json = serde_json::to_value(&event).unwrap();
    assert!(old_json.get("application").is_none());
    let mut expected = event.format.canonical_bytes();
    expected.extend(event.event_id.canonical_bytes());
    expected.extend(event.record_hash.canonical_bytes());
    expected.extend(event.payload.canonical_bytes());
    assert_eq!(old, expected);
    event.application = Some(ApplicationGuard::try_from(guard(transaction, 1)).unwrap());
    assert!(
        !event.supported(),
        "legacy format may not smuggle an application guard"
    );
    event.format = RevisionEvent::APPLICATION_FORMAT.into();
    assert!(event.supported());
    assert_eq!(event.schema_version(), 6);
    assert_ne!(old, event.canonical_bytes());
    let restored: RevisionEvent =
        serde_json::from_value(serde_json::to_value(&event).unwrap()).unwrap();
    assert_eq!(restored, event);
    assert_eq!(
        restored.application.as_ref().unwrap().as_data(),
        event.application.as_ref().unwrap().as_data()
    );
    event.application = None;
    assert!(!event.supported(), "new ordinary format requires a guard");
}

#[test]
fn guard_codec_refuses_invalid_cursor_identity_and_step_shape() {
    let transaction = TransactionId::mint();
    for cursor in [-1, 0] {
        assert!(ApplicationGuard::try_from(guard(transaction, cursor)).is_err());
    }
    let mut invalid = guard(transaction, 1);
    invalid.application_id.0 = "not-an-identity".into();
    assert!(ApplicationGuard::try_from(invalid).is_err());
    let mut invalid = serde_json::to_value(guard(transaction, 1)).unwrap();
    invalid["step"] = json!({"kind":"Mapping"});
    assert!(ApplicationGuard::try_from(
        serde_json::from_value::<w::EkrIntegrateApplicationPublicationGuard>(invalid).unwrap()
    )
    .is_err());
    let mut invalid = guard(transaction, 1);
    invalid.proposal_digest.0 = "bad hash".into();
    assert!(ApplicationGuard::try_from(invalid).is_err());
}
