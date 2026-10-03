//! Source identities and exact payload bytes at the public generated boundary.
use ekr_sdk::knowledge::observation_import;

#[test]
fn observation_builder_reuses_the_existing_source_key() {
    let input = observation_import(
        "manual".into(),
        Some("entry-1".into()),
        "2026-10-03T00:00:00Z".into(),
        serde_json::from_str("\"FeedItem\"").unwrap(),
        b"a\r\nb\n",
    );
    let key = ekr_core::ObservationIdempotencyKey {
        source: "manual".into(),
        source_native_id: Some("entry-1".into()),
        content_hash: ekr_core::ContentHash::of_bytes(b"a\r\nb\n"),
    };
    assert_eq!(
        input.observation.observation_id.0,
        key.observation_id().to_string()
    );
    assert_eq!(
        ekr_core::bytes::decode(&input.payload).unwrap(),
        b"a\r\nb\n"
    );
}
