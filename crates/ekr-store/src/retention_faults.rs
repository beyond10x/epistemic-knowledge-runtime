//! Response-loss injection around real native atomic writes. These are restart tests, not
//! evidence of process death inside a provider transaction; native crash gates remain separate.
use super::*;
use ekr_core::contract_data::{EkrIntegrateRetainedInterpretation, EkrObserveObservationImport};
use ekr_core::generated_identity::{Identity, InterpretationId};
use std::cell::Cell;

thread_local! { static LOSS: Cell<u8> = const { Cell::new(0) }; }
pub(super) fn before_write() -> bool {
    LOSS.with(|loss| loss.get() == 1)
}
pub(super) fn after_write(
    result: Result<eventlog_core::AppendGroupResult, EventLogError>,
) -> Result<eventlog_core::AppendGroupResult, EventLogError> {
    if result.is_ok() && LOSS.with(|loss| loss.get() == 2) {
        Err(EventLogError::UnknownCommit)
    } else {
        result
    }
}
struct Reset;
impl Drop for Reset {
    fn drop(&mut self) {
        LOSS.with(|loss| loss.set(0));
    }
}

fn fixtures() -> (
    EkrObserveObservationImport,
    EkrIntegrateRetainedInterpretation,
) {
    let bytes = b"raw source";
    let hash = ContentHash::of_bytes(bytes);
    let key = ekr_core::ObservationIdempotencyKey {
        source: "manual".into(),
        source_native_id: None,
        content_hash: hash,
    };
    let observation = serde_json::from_value(serde_json::json!({
        "observation":{"observation_id":key.observation_id(),"source":"manual","content_hash":hash.to_hex(),"captured_at":"1970-01-01T00:00:00Z","kind":"FeedItem"},
        "key":{"source":"manual","content_hash":hash.to_hex()},"payload":ekr_core::bytes::encode(bytes)
    })).unwrap();
    let id = InterpretationId::mint();
    let root = ekr_core::GraphRootId::mint();
    let document = serde_json::json!({"version":{"interpretation_id":id,"version":1},"root_id":root,"observations":[key.observation_id()],
        "local_schema":{"node_types":[],"edge_types":[]},"entities":[],"facts":[],"evidence":[]});
    let payload = serde_json::to_vec_pretty(&document).unwrap();
    let interpretation = serde_json::from_value(serde_json::json!({
        "version":{"interpretation_id":id,"version":1,"document_digest":ContentHash::of_bytes(&payload).to_hex()},
        "interpretation":{"document":document,"blockers":[],"receipts":[]},"payload":ekr_core::bytes::encode(&payload),
        "root":{"id":root,"space":"Transient","schema_version_id":ekr_core::SchemaVersionId::mint(),"created_at":"1970-01-01T00:00:00Z"}
    })).unwrap();
    (observation, interpretation)
}

fn exercise<S: AtomicBlobEventStore>(open: impl Fn() -> EventlogStore<S>, mode: u8) {
    let _reset = Reset;
    let (observation, interpretation) = fixtures();
    let store = open();
    LOSS.with(|loss| loss.set(mode));
    assert!(matches!(
        store.retain_observation(&observation, Timestamp::EPOCH),
        Err(StoreError::UnknownCommit)
    ));
    LOSS.with(|loss| loss.set(0));
    assert_eq!(
        store.retained_observations().unwrap().len(),
        usize::from(mode == 2)
    );
    assert_eq!(
        store
            .get(&observation.key.content_hash.0.parse().unwrap())
            .unwrap()
            .is_some(),
        mode == 2
    );
    drop(store);
    let store = open();
    assert_eq!(
        store
            .retain_observation(&observation, Timestamp::from_millis(11))
            .unwrap(),
        mode == 1
    );
    LOSS.with(|loss| loss.set(mode));
    assert!(matches!(
        store.retain_interpretation(&interpretation, Timestamp::EPOCH),
        Err(StoreError::UnknownCommit)
    ));
    LOSS.with(|loss| loss.set(0));
    assert_eq!(
        store.retained_interpretations().unwrap().len(),
        usize::from(mode == 2)
    );
    assert_eq!(
        store
            .get(&interpretation.version.document_digest.0.parse().unwrap())
            .unwrap()
            .is_some(),
        mode == 2
    );
    drop(store);
    let store = open();
    assert_eq!(
        store
            .retain_interpretation(&interpretation, Timestamp::from_millis(22))
            .unwrap()
            .1,
        mode == 1
    );
    assert_eq!(store.retained_interpretations().unwrap(), [interpretation]);
    assert_eq!(store.retained_observations().unwrap(), [observation]);
    assert!(store.head().unwrap().is_none());
}

#[test]
fn before_and_after_native_response_loss_reopens_without_duplicate_knowledge() {
    for mode in [1, 2] {
        let directory = tempfile::tempdir().unwrap();
        exercise(
            || FileStore::file(&directory.path().join("file"), "retention", None).unwrap(),
            mode,
        );
        exercise(
            || SqliteStore::sqlite(&directory.path().join("sqlite"), "retention", None).unwrap(),
            mode,
        );
    }
}
