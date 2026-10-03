//! Response loss and abrupt child-process exit immediately around real native atomic writes.
//! These do not inject faults inside the provider's own transaction implementation.
use super::*;
use ekr_core::contract_data::{EkrIntegrateRetainedInterpretation, EkrObserveObservationImport};
use ekr_core::generated_identity::{Identity, InterpretationId};
use std::cell::Cell;

thread_local! { static LOSS: Cell<u8> = const { Cell::new(0) }; }
pub(super) fn before_write() -> bool {
    if LOSS.with(|loss| loss.get() == 3) {
        std::process::exit(86);
    }
    LOSS.with(|loss| loss.get() == 1)
}
pub(super) fn after_write(
    result: Result<eventlog_core::AppendGroupResult, EventLogError>,
) -> Result<eventlog_core::AppendGroupResult, EventLogError> {
    if result.is_ok() && LOSS.with(|loss| loss.get() == 4) {
        std::process::exit(86);
    }
    if result.is_ok() && LOSS.with(|loss| loss.get() == 2) {
        Err(EventLogError::UnknownCommit)
    } else {
        result
    }
}

/// Invoked only by the parent below, with exact fixture bytes and a private temporary store.
#[test]
#[ignore = "child-process entry point; executed by abrupt_exit_retention_reopens_without_duplicates"]
fn retention_crash_child() {
    let directory =
        std::path::PathBuf::from(std::env::var_os("EKR_RETENTION_CRASH_DIRECTORY").unwrap());
    let (observation, interpretation): (
        EkrObserveObservationImport,
        EkrIntegrateRetainedInterpretation,
    ) = serde_json::from_slice(&std::fs::read(directory.join("input.json")).unwrap()).unwrap();
    let mode: u8 = std::env::var("EKR_RETENTION_CRASH_MODE")
        .unwrap()
        .parse()
        .unwrap();
    assert!([3, 4].contains(&mode));
    let operation = std::env::var("EKR_RETENTION_CRASH_OPERATION").unwrap();
    fn write<S: AtomicBlobEventStore>(
        store: EventlogStore<S>,
        observation: &EkrObserveObservationImport,
        interpretation: &EkrIntegrateRetainedInterpretation,
        mode: u8,
        operation: &str,
    ) {
        LOSS.with(|loss| loss.set(mode));
        match operation {
            "observe" => {
                store
                    .retain_observation(observation, Timestamp::EPOCH)
                    .unwrap();
            }
            "incubate" => {
                store
                    .retain_interpretation(interpretation, Timestamp::EPOCH)
                    .unwrap();
            }
            _ => panic!("unknown crash operation"),
        }
        panic!("native boundary did not exit the process");
    }
    if std::env::var("EKR_RETENTION_CRASH_PROVIDER").unwrap() == "sqlite" {
        write(
            SqliteStore::sqlite(&directory.join("store"), "retention", None).unwrap(),
            &observation,
            &interpretation,
            mode,
            &operation,
        );
    } else {
        write(
            FileStore::file(&directory.join("store"), "retention", None).unwrap(),
            &observation,
            &interpretation,
            mode,
            &operation,
        );
    }
}

#[test]
fn abrupt_exit_retention_reopens_without_duplicates() {
    fn check<S: AtomicBlobEventStore>(
        store: EventlogStore<S>,
        observation: &EkrObserveObservationImport,
        interpretation: &EkrIntegrateRetainedInterpretation,
        mode: u8,
        operation: &str,
    ) {
        let applied = mode == 4;
        if operation == "observe" {
            assert_eq!(
                store.retained_observations().unwrap().len(),
                usize::from(applied)
            );
            assert_eq!(
                store
                    .get(&observation.key.content_hash.0.parse().unwrap())
                    .unwrap()
                    .is_some(),
                applied
            );
            assert_eq!(
                store
                    .retain_observation(observation, Timestamp::from_millis(20))
                    .unwrap(),
                !applied
            );
            assert!(!store
                .retain_observation(observation, Timestamp::from_millis(30))
                .unwrap());
            assert_eq!(
                store.retained_observations().unwrap(),
                std::slice::from_ref(observation)
            );
        } else {
            assert_eq!(
                store.retained_interpretations().unwrap().len(),
                usize::from(applied)
            );
            assert_eq!(
                store
                    .get(&interpretation.version.document_digest.0.parse().unwrap())
                    .unwrap()
                    .is_some(),
                applied
            );
            assert_eq!(
                store
                    .retain_interpretation(interpretation, Timestamp::from_millis(20))
                    .unwrap()
                    .1,
                !applied
            );
            assert!(
                !store
                    .retain_interpretation(interpretation, Timestamp::from_millis(30))
                    .unwrap()
                    .1
            );
            assert_eq!(
                store.retained_interpretations().unwrap(),
                std::slice::from_ref(interpretation)
            );
        }
        assert!(store.head().unwrap().is_none());
    }
    for provider in ["file", "sqlite"] {
        for operation in ["observe", "incubate"] {
            for mode in [3, 4] {
                let directory = tempfile::tempdir().unwrap();
                let fixtures = fixtures();
                std::fs::write(
                    directory.path().join("input.json"),
                    serde_json::to_vec(&fixtures).unwrap(),
                )
                .unwrap();
                let output = std::process::Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--exact",
                        "eventlog::retention_faults::retention_crash_child",
                        "--ignored",
                        "--nocapture",
                    ])
                    .env("EKR_RETENTION_CRASH_DIRECTORY", directory.path())
                    .env("EKR_RETENTION_CRASH_PROVIDER", provider)
                    .env("EKR_RETENTION_CRASH_OPERATION", operation)
                    .env("EKR_RETENTION_CRASH_MODE", mode.to_string())
                    .output()
                    .unwrap();
                assert_eq!(
                    output.status.code(),
                    Some(86),
                    "{provider}/{operation}/{mode}: {} {}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                if provider == "sqlite" {
                    check(
                        SqliteStore::sqlite(&directory.path().join("store"), "retention", None)
                            .unwrap(),
                        &fixtures.0,
                        &fixtures.1,
                        mode,
                        operation,
                    );
                } else {
                    check(
                        FileStore::file(&directory.path().join("store"), "retention", None)
                            .unwrap(),
                        &fixtures.0,
                        &fixtures.1,
                        mode,
                        operation,
                    );
                }
            }
        }
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
