//! Source-key identity remains shared with the adapter and transport client.
use ekr_core::{ContentHash, ObservationIdempotencyKey};

#[test]
fn source_native_identity_and_exact_bytes_each_bind_observation_identity() {
    let key = ObservationIdempotencyKey {
        source: "manual".into(),
        source_native_id: None,
        content_hash: ContentHash::of_bytes(b"health: amber\n"),
    };
    let id = key.observation_id();
    assert_eq!(id, key.clone().observation_id());
    let mut changed = key.clone();
    changed.source_native_id = Some(String::new());
    assert_ne!(id, changed.observation_id());
    changed = key.clone();
    changed.source = "other".into();
    assert_ne!(id, changed.observation_id());
    changed = key;
    changed.content_hash = ContentHash::of_bytes(b"health: amber\r\n");
    assert_ne!(id, changed.observation_id());
    assert_eq!(id.to_string().as_bytes()[14], b'8');
}
