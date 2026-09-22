//! Synchronous store calls refuse an entered runtime before invoking persistence.
mod fixture;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use ekr_core::{ContentHash, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{CanonicalGraph, RevisionEvent};
use ekr_ontology::Ontology;
use ekr_store::{
    CommitAuthority, EventlogStore, FileStore, GraphDocument, Initialize, ObjectStore,
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

fn refusal<T>(result: Result<T, StoreError>) {
    match result {
        Err(error) => {
            assert_eq!(error, StoreError::RuntimeContext);
            assert_eq!(
                error.to_string(),
                "synchronous store access requires a thread outside a Tokio runtime"
            );
        }
        Ok(_) => panic!("store accepted an ambient-runtime call"),
    }
}

#[test]
fn entered_runtime_reads_refuse_and_the_store_remains_usable() {
    let directory = TempDir::new().unwrap();
    let sqlite = SqliteStore::sqlite(
        &directory.path().join("store.db"),
        "runtime-fixture",
        fixture::ontology(),
    )
    .unwrap();
    let file = FileStore::file(
        &directory.path().join("files"),
        "runtime-fixture",
        fixture::ontology(),
    )
    .unwrap();
    let ambient = runtime();
    {
        let _entered = ambient.enter();
        refusal(sqlite.head());
        refusal(file.head());
    }
    assert_eq!(sqlite.head().unwrap(), None);
    assert_eq!(file.head().unwrap(), None);
}

#[test]
fn running_runtime_reads_return_a_refusal_instead_of_panicking() {
    let directory = TempDir::new().unwrap();
    let store = SqliteStore::sqlite(
        &directory.path().join("store.db"),
        "runtime-fixture",
        fixture::ontology(),
    )
    .unwrap();
    runtime().block_on(async { refusal(store.head()) });
    assert_eq!(store.head().unwrap(), None);
}

#[test]
fn constructors_refuse_before_creating_paths() {
    let directory = TempDir::new().unwrap();
    let check = || {
        let sqlite_path = directory.path().join("store.db");
        let file_path = directory.path().join("files");
        refusal(SqliteStore::sqlite(
            &sqlite_path,
            "runtime-fixture",
            fixture::ontology(),
        ));
        refusal(FileStore::file(
            &file_path,
            "runtime-fixture",
            fixture::ontology(),
        ));
        assert!(!sqlite_path.exists());
        assert!(!file_path.exists());
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    };
    let ambient = runtime();
    {
        let _entered = ambient.enter();
        check();
    }
    ambient.block_on(async { check() });
}

struct CountingAuthority(Arc<AtomicUsize>);

impl CommitAuthority for CountingAuthority {
    fn admit_seed(&self, _: &[u8], _: &Ontology) -> Result<CanonicalGraph, StoreError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Err(StoreError::InvalidSeed("test authority refuses".to_owned()))
    }

    fn attests(&self, _: &RecordedValidation) -> bool {
        self.0.fetch_add(1, Ordering::SeqCst);
        false
    }
}

fn all_io_refuses<S: AtomicEventStore>(store: EventlogStore<S>) {
    let calls = Arc::new(AtomicUsize::new(0));
    let store = store.under(CountingAuthority(calls.clone()));
    let retained = b"completed before entering runtime";
    let pending = b"must not be persisted";
    let object = store
        .put(StorageClass::Ephemeral, retained, Timestamp::EPOCH)
        .unwrap();
    let graph = fixture::seed_graph(&fixture::ontology());
    let graph_bytes = GraphDocument::of(&graph).to_bytes().unwrap();
    let event = RevisionEvent::Seeded {
        revision_id: RevisionId::mint(),
        seed_hash: ContentHash::of_bytes(pending),
    };
    runtime().block_on(async {
        refusal(store.seed_bytes());
        refusal(store.head());
        refusal(store.fold());
        refusal(store.replay(RevisionNumber::SEED));
        refusal(store.append(&event));
        refusal(store.initialize(pending, Timestamp::EPOCH));
        refusal(store.store_graph(&graph, Timestamp::EPOCH));
        refusal(store.get(&object.content_hash));
        refusal(store.put(StorageClass::Canonical, retained, Timestamp::EPOCH));
        refusal(store.put(StorageClass::Canonical, pending, Timestamp::EPOCH));
    });
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(store.head().unwrap(), None);
    assert_eq!(store.seed_bytes().unwrap(), None);
    assert_eq!(
        store.get(&object.content_hash).unwrap(),
        Some(retained.to_vec())
    );
    assert_eq!(store.get(&ContentHash::of_bytes(pending)).unwrap(), None);
    assert_eq!(
        store.get(&ContentHash::of_bytes(&graph_bytes)).unwrap(),
        None
    );
    // A refused retention promotion must leave the previous class intact.
    assert_eq!(
        store
            .put(StorageClass::Ephemeral, retained, Timestamp::EPOCH)
            .unwrap(),
        object
    );
    // Returning to a synchronous context restores normal persistence.
    let written = store
        .put(StorageClass::Canonical, pending, Timestamp::EPOCH)
        .unwrap();
    assert_eq!(
        store.get(&written.content_hash).unwrap(),
        Some(pending.to_vec())
    );
}

#[test]
fn every_file_operation_refuses_before_authority_or_persistence() {
    let directory = TempDir::new().unwrap();
    all_io_refuses(
        FileStore::file(directory.path(), "runtime-fixture", fixture::ontology()).unwrap(),
    );
}

#[test]
fn every_sqlite_operation_refuses_before_authority_or_persistence() {
    let directory = TempDir::new().unwrap();
    all_io_refuses(
        SqliteStore::sqlite(
            &directory.path().join("store.db"),
            "runtime-fixture",
            fixture::ontology(),
        )
        .unwrap(),
    );
}

#[test]
fn dropping_inside_a_running_runtime_preserves_completed_writes() {
    let directory = TempDir::new().unwrap();
    let sqlite_path = directory.path().join("store.db");
    let file_path = directory.path().join("files");
    let sqlite = SqliteStore::sqlite(&sqlite_path, "runtime-fixture", fixture::ontology()).unwrap();
    let file = FileStore::file(&file_path, "runtime-fixture", fixture::ontology()).unwrap();
    let bytes = b"completed write survives drop";
    let hash = ContentHash::of_bytes(bytes);
    sqlite
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    file.put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .unwrap();
    runtime().block_on(async move {
        drop(sqlite);
        drop(file);
    });
    let sqlite = SqliteStore::sqlite(&sqlite_path, "runtime-fixture", fixture::ontology()).unwrap();
    let file = FileStore::file(&file_path, "runtime-fixture", fixture::ontology()).unwrap();
    assert_eq!(sqlite.get(&hash).unwrap(), Some(bytes.to_vec()));
    assert_eq!(file.get(&hash).unwrap(), Some(bytes.to_vec()));
}
