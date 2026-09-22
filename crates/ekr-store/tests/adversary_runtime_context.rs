//! Attacks the synchronous boundary with populated stores and actual entered handles.
//! Substitute authority here measures callback reachability, not kernel seed validation.
mod fixture;
mod lineage;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use ekr_core::{ContentHash, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::CanonicalGraph;
use ekr_ontology::Ontology;
use ekr_store::{
    Appended, CommitAuthority, EventlogStore, FileStore, GraphDocument, Initialize, ObjectStore,
    RecordedValidation, RevisionLog, SqliteStore, StorageClass, StoreError,
};
use eventlog_core::AtomicEventStore;
use tempfile::TempDir;

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn refused<T>(result: Result<T, StoreError>) {
    assert!(
        matches!(result, Err(StoreError::RuntimeContext)),
        "every synchronous I/O path must refuse before provider or authority work"
    );
}

fn bytes_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

#[test]
fn entered_handle_constructor_refusal_preserves_existing_provider_bytes_and_missing_parents() {
    let directory = TempDir::new().unwrap();
    let sqlite_path = directory.path().join("existing.db");
    let file_path = directory.path().join("existing-files");
    let bytes = b"constructor boundary retained object";
    let hash = ContentHash::of_bytes(bytes);
    SqliteStore::sqlite(&sqlite_path, "constructor-probe", fixture::ontology())
        .unwrap()
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    FileStore::file(&file_path, "constructor-probe", fixture::ontology())
        .unwrap()
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    let before = bytes_under(directory.path());
    let ambient = runtime();
    let handle = ambient.handle().clone();
    std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let _entered = handle.enter();
                for tenant in ["constructor-probe", "invalid\ntenant"] {
                    refused(SqliteStore::sqlite(
                        &sqlite_path,
                        tenant,
                        fixture::ontology(),
                    ));
                    refused(FileStore::file(&file_path, tenant, fixture::ontology()));
                    refused(SqliteStore::sqlite(
                        &directory.path().join("missing/child/store.db"),
                        tenant,
                        fixture::ontology(),
                    ));
                    refused(FileStore::file(
                        &directory.path().join("missing/child/files"),
                        tenant,
                        fixture::ontology(),
                    ));
                }
                assert_eq!(bytes_under(directory.path()), before);
            })
            .join()
            .unwrap();
    });
    assert!(!directory.path().join("missing").exists());
    assert_eq!(
        SqliteStore::sqlite(&sqlite_path, "constructor-probe", fixture::ontology())
            .unwrap()
            .get(&hash)
            .unwrap(),
        Some(bytes.to_vec())
    );
    assert_eq!(
        FileStore::file(&file_path, "constructor-probe", fixture::ontology())
            .unwrap()
            .get(&hash)
            .unwrap(),
        Some(bytes.to_vec())
    );
}

struct AuthorityCalls {
    seed: Arc<AtomicUsize>,
    validation: Arc<AtomicUsize>,
}

impl CommitAuthority for AuthorityCalls {
    fn admit_seed(&self, bytes: &[u8], ontology: &Ontology) -> Result<CanonicalGraph, StoreError> {
        self.seed.fetch_add(1, Ordering::SeqCst);
        lineage::Attesting.admit_seed(bytes, ontology)
    }

    fn attests(&self, validation: &RecordedValidation) -> bool {
        self.validation.fetch_add(1, Ordering::SeqCst);
        lineage::Attesting.attests(validation)
    }
}

fn populated_boundary<S: AtomicEventStore>(store: EventlogStore<S>, root: &Path) {
    let seed_calls = Arc::new(AtomicUsize::new(0));
    let validation_calls = Arc::new(AtomicUsize::new(0));
    let store = store.under(AuthorityCalls {
        seed: seed_calls.clone(),
        validation: validation_calls.clone(),
    });
    let graph = fixture::seed_graph(&fixture::ontology());
    let seed_bytes = GraphDocument::of(&graph).to_bytes().unwrap();
    lineage::seed_and_commit(&store, &graph).unwrap();
    let prior = store.head().unwrap().unwrap();
    assert_eq!(prior.revision, RevisionNumber::new(1));
    assert!(seed_calls.load(Ordering::SeqCst) > 0);
    assert!(validation_calls.load(Ordering::SeqCst) > 0);
    let object = store
        .put(StorageClass::Ephemeral, b"retained", Timestamp::EPOCH)
        .unwrap();
    let seed_calls_before = seed_calls.load(Ordering::SeqCst);
    let validation_calls_before = validation_calls.load(Ordering::SeqCst);
    let before = bytes_under(root);
    let event = lineage::proposed(TransactionId::mint());
    let attempt = || {
        // Dispatch through public trait objects as well as the inherent graph archival method.
        let objects: &dyn ObjectStore = &store;
        let revisions: &dyn RevisionLog = &store;
        let initialization: &dyn Initialize = &store;
        refused(objects.get(&object.content_hash));
        refused(objects.get(&ContentHash::of_bytes(b"absent")));
        refused(objects.put(StorageClass::Canonical, b"retained", Timestamp::EPOCH));
        refused(objects.put(StorageClass::Canonical, b"new", Timestamp::EPOCH));
        refused(revisions.seed_bytes());
        refused(revisions.head());
        refused(revisions.fold());
        refused(revisions.replay(RevisionNumber::SEED));
        refused(revisions.replay(RevisionNumber::new(u64::MAX)));
        refused(revisions.append(&event));
        refused(initialization.initialize(&seed_bytes, Timestamp::EPOCH));
        refused(initialization.initialize(b"malformed", Timestamp::EPOCH));
        refused(store.store_graph(&graph, Timestamp::EPOCH));
        assert_eq!(seed_calls.load(Ordering::SeqCst), seed_calls_before);
        assert_eq!(
            validation_calls.load(Ordering::SeqCst),
            validation_calls_before
        );
        assert_eq!(bytes_under(root), before);
    };
    let ambient = runtime();
    {
        let _entered = ambient.enter();
        attempt();
    }
    ambient.block_on(async { attempt() });
    // No rejected call consumes the append key or promotes retention.
    assert_eq!(
        store
            .put(StorageClass::Ephemeral, b"retained", Timestamp::EPOCH)
            .unwrap(),
        object
    );
    assert_eq!(store.append(&event).unwrap(), Appended::Written);
    assert_eq!(store.append(&event).unwrap(), Appended::AlreadyRecorded);
    assert_eq!(store.head().unwrap(), Some(prior));
    assert_eq!(store.get(&ContentHash::of_bytes(b"new")).unwrap(), None);
    assert_eq!(store.seed_bytes().unwrap(), Some(seed_bytes));
}

#[test]
fn populated_file_lineage_refuses_before_seed_and_commit_callbacks_or_disk_changes() {
    let directory = TempDir::new().unwrap();
    populated_boundary(
        FileStore::file(directory.path(), "populated-file", fixture::ontology()).unwrap(),
        directory.path(),
    );
}

#[test]
fn populated_sqlite_lineage_refuses_before_seed_and_commit_callbacks_or_disk_changes() {
    let directory = TempDir::new().unwrap();
    populated_boundary(
        SqliteStore::sqlite(
            &directory.path().join("store.db"),
            "populated-sqlite",
            fixture::ontology(),
        )
        .unwrap(),
        directory.path(),
    );
}

fn unwind_after_refusal<S: AtomicEventStore>(store: EventlogStore<S>) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let owned_store = store;
        refused(owned_store.head());
        panic!("synthetic caller unwind after named refusal");
    }));
    let panic = result.expect_err("the deliberate caller panic must be observed");
    assert_eq!(
        panic.downcast_ref::<&str>(),
        Some(&"synthetic caller unwind after named refusal")
    );
}

#[test]
fn store_drop_during_caller_unwind_in_entered_handle_keeps_completed_data_reopenable() {
    let directory = TempDir::new().unwrap();
    let sqlite_path = directory.path().join("store.db");
    let file_path = directory.path().join("files");
    let sqlite = SqliteStore::sqlite(&sqlite_path, "drop-probe", fixture::ontology()).unwrap();
    let file = FileStore::file(&file_path, "drop-probe", fixture::ontology()).unwrap();
    let bytes = b"completed before caller unwind";
    let hash = ContentHash::of_bytes(bytes);
    sqlite
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    file.put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    let ambient = runtime();
    {
        let _entered = ambient.enter();
        unwind_after_refusal(sqlite);
        unwind_after_refusal(file);
    }
    assert_eq!(
        SqliteStore::sqlite(&sqlite_path, "drop-probe", fixture::ontology())
            .unwrap()
            .get(&hash)
            .unwrap(),
        Some(bytes.to_vec())
    );
    assert_eq!(
        FileStore::file(&file_path, "drop-probe", fixture::ontology())
            .unwrap()
            .get(&hash)
            .unwrap(),
        Some(bytes.to_vec())
    );
}

#[test]
fn plain_worker_thread_can_open_and_write_while_another_thread_runs_tokio() {
    let directory = TempDir::new().unwrap();
    runtime().block_on(async {
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    assert!(tokio::runtime::Handle::try_current().is_err());
                    let sqlite = SqliteStore::sqlite(
                        &directory.path().join("worker.db"),
                        "worker",
                        fixture::ontology(),
                    )
                    .unwrap();
                    let file = FileStore::file(
                        &directory.path().join("worker-files"),
                        "worker",
                        fixture::ontology(),
                    )
                    .unwrap();
                    let bytes = b"ordinary synchronous worker";
                    let sqlite_object = sqlite
                        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
                        .unwrap();
                    let file_object = file
                        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
                        .unwrap();
                    assert_eq!(
                        sqlite.get(&sqlite_object.content_hash).unwrap(),
                        Some(bytes.to_vec())
                    );
                    assert_eq!(
                        file.get(&file_object.content_hash).unwrap(),
                        Some(bytes.to_vec())
                    );
                })
                .join()
                .unwrap();
        });
    });
}
