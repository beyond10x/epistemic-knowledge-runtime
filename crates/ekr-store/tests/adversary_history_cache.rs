//! Adversary pass 1 on `story:history-loaded-once-per-process` (wave ingest-01, unit H).
//!
//! Each case states the behaviour it expects and the configuration that reaches it.

use std::collections::BTreeSet;
use std::path::Path;

use ekr_core::TransactionId;
use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    read_work, AdmittedRevision, Appended, CommitAuthority, FileStore, Initialize, ObjectStore,
    Publication, PublicationObject, ReadWork, RetainedHistory, RevisionLog, SqliteStore,
    StorageClass, StoreError,
};
use tempfile::TempDir;

const TENANT: &str = "ekr";

fn restart_count() {
    let _ = read_work();
}

/// Reads every record and seed payload the way a replay does, and admits nothing.
struct Touch;

impl CommitAuthority for Touch {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }

    fn replay(
        &self,
        history: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        for occurrence in &history.occurrences {
            history.content(occurrence.event.record_hash, StorageClass::Canonical)?;
            if let RevisionPayload::Seeded { seed_hash, .. } = occurrence.event.payload {
                history.content(seed_hash, StorageClass::Canonical)?;
            }
        }
        Ok(None)
    }
}

/// Requires one extra object and reads it at `Canonical`, as the kernel reads a record.
struct Needs(ContentHash);

impl CommitAuthority for Needs {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::from([self.0]))
    }

    fn replay(
        &self,
        history: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        history.content(self.0, StorageClass::Canonical)?;
        Ok(None)
    }
}

fn payload(label: &str) -> Vec<u8> {
    format!("adversary history {label} {} ", EventId::mint())
        .repeat(4)
        .into_bytes()
}

fn occurrence(
    payload: RevisionPayload,
    expected_version: u64,
    record: Vec<u8>,
    extra: Option<Vec<u8>>,
) -> Publication {
    let record_hash = ContentHash::of_bytes(&record);
    let canonical = |bytes| PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
        bytes,
    };
    let mut objects = std::collections::BTreeMap::from([(record_hash, canonical(record))]);
    if let Some(bytes) = extra {
        objects.insert(ContentHash::of_bytes(&bytes), canonical(bytes));
    }
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: EventId::mint(),
            record_hash,
            payload,
        },
        objects,
        expected_version,
    }
}

fn seed(record: Vec<u8>, document: Vec<u8>) -> Publication {
    occurrence(
        RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: ContentHash::of_bytes(&document),
        },
        0,
        record,
        Some(document),
    )
}

fn proposal(expected_version: u64, record: Vec<u8>) -> Publication {
    occurrence(
        RevisionPayload::TransactionProposed {
            transaction_id: TransactionId::mint(),
            proposer: AgentId::mint(),
            operations_hash: None,
        },
        expected_version,
        record,
        None,
    )
}

fn written<S: RevisionLog + Initialize>(store: &S, label: &str, later: u64) {
    let (record, document) = (payload(&format!("{label} seed record")), payload(label));
    assert_eq!(
        store.initialize(&seed(record, document)).unwrap(),
        Appended::Written
    );
    for at in 1..=later {
        assert_eq!(
            store
                .publish(&proposal(at, payload(&format!("{label} record {at}"))))
                .unwrap(),
            Appended::Written
        );
    }
}

fn sqlite_under(root: &Path, authority: impl CommitAuthority + 'static) -> SqliteStore {
    SqliteStore::sqlite(&root.join("state.db"), TENANT, None)
        .expect("the SQLite provider opens")
        .under(authority)
}

fn file_under(root: &Path, authority: impl CommitAuthority + 'static) -> FileStore {
    FileStore::file(&root.join("state"), TENANT, None)
        .expect("the file provider opens")
        .under(authority)
}

// ---------------------------------------------------------------------------------------------
// A. A handle whose verified copy is not the one the process registry points at hashes again on
//    every later read once the handle that registered it is dropped.
// ---------------------------------------------------------------------------------------------

/// Two handles on one store in one process. The first registers its verified copies; the second
/// loads the same blobs, finds the registry live, and keeps a copy of its own that is never
/// registered. Once the first handle is dropped, every `content` on the second handle's histories
/// hashes again, for as long as it lives: its second read repeats the work its first read did.
fn a_second_handle_keeps_its_saving<S: RevisionLog + ObjectStore + Initialize>(
    open: impl Fn() -> S,
) {
    {
        let writer = open();
        written(&writer, "two handles", 2);
    }
    let first = open();
    first.history().unwrap();

    let second = open();
    restart_count();
    let loaded = second.history().unwrap();
    let work = read_work();
    assert_eq!(loaded.objects.len(), 4);
    assert_eq!(
        (work.blobs_read, work.blobs_hashed),
        (4, 0),
        "the second handle loads every blob and compares it with the first handle's copies"
    );

    drop(first);
    restart_count();
    let again = second.history().unwrap();
    assert_eq!(again, loaded);
    assert_eq!(
        read_work(),
        ReadWork::default(),
        "a second read on one handle hashed again blobs its first read verified, because the \
         registry entry pointed at another handle's copy, since dropped"
    );
}

#[test]
fn adversary_history_second_handle_second_read_hashes_nothing_sqlite() {
    let directory = TempDir::new().unwrap();
    a_second_handle_keeps_its_saving(|| sqlite_under(directory.path(), Touch));
}

#[test]
fn adversary_history_second_handle_second_read_hashes_nothing_file() {
    let directory = TempDir::new().unwrap();
    a_second_handle_keeps_its_saving(|| file_under(directory.path(), Touch));
}

// ---------------------------------------------------------------------------------------------
// B. `get` now fills the per-handle memo, and the memo keeps the retention class it read. A raise
//    by another handle afterwards is not seen, so a history read that requires the raised class
//    is refused where it was accepted before `get` held anything.
// ---------------------------------------------------------------------------------------------

fn a_raise_after_get_is_seen<S: RevisionLog + ObjectStore + Initialize>(
    open: impl Fn() -> S,
    open_needing: impl Fn(ContentHash) -> S,
) {
    let evidence = payload("raised later");
    let address = ContentHash::of_bytes(&evidence);
    {
        let writer = open();
        written(&writer, "raise", 1);
        let stored = writer
            .put(StorageClass::Provenance, &evidence, Timestamp::EPOCH)
            .unwrap();
        assert_eq!(stored.storage_class, StorageClass::Provenance);
    }

    let reader = open_needing(address);
    assert_eq!(
        reader.get(&address).unwrap().as_deref(),
        Some(evidence.as_slice())
    );

    {
        let other = open();
        let raised = other
            .put(StorageClass::Canonical, &evidence, Timestamp::EPOCH)
            .unwrap();
        assert_eq!(raised.storage_class, StorageClass::Canonical);
    }

    let fresh = open_needing(address).history();
    assert!(
        fresh.is_ok(),
        "a handle holding nothing reads the raised object: {fresh:?}"
    );
    let held = reader.history();
    assert!(
        held.is_ok(),
        "a handle that only called get() before another handle raised the object's retention \
         refused a history a fresh handle accepts: {held:?}"
    );
}

#[test]
fn adversary_history_get_then_foreign_raise_still_reads_sqlite() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    a_raise_after_get_is_seen(
        || sqlite_under(root, Touch),
        |address| sqlite_under(root, Needs(address)),
    );
}

#[test]
fn adversary_history_get_then_foreign_raise_still_reads_file() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    a_raise_after_get_is_seen(
        || file_under(root, Touch),
        |address| file_under(root, Needs(address)),
    );
}

// ---------------------------------------------------------------------------------------------
// C. A file store replaced by a different history. The provider refuses the replacement for a
//    handle that observed the original; a read at an earlier revision answered wholly from what
//    the handle holds must not answer where the same handle's head read refuses.
// ---------------------------------------------------------------------------------------------

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

#[test]
fn adversary_history_replaced_file_store_selected_read_refuses_like_head_read() {
    let held_dir = TempDir::new().unwrap();
    let other_dir = TempDir::new().unwrap();
    let reader = file_under(held_dir.path(), Touch);
    written(&reader, "original", 1);
    let before = reader.history().unwrap();
    assert_eq!(before.occurrences.len(), 2);
    assert_eq!(
        reader
            .history_at(RevisionNumber::SEED)
            .unwrap()
            .occurrences
            .len(),
        1
    );
    {
        let other = file_under(other_dir.path(), Touch);
        written(&other, "replacement", 1);
    }
    std::fs::remove_dir_all(held_dir.path().join("state")).unwrap();
    copy_tree(
        &other_dir.path().join("state"),
        &held_dir.path().join("state"),
    );

    let head = reader.history();
    let selected = reader.history_at(RevisionNumber::SEED);
    eprintln!("head read after replacement: {head:?}");
    eprintln!(
        "selected read after replacement: {:?}",
        selected.as_ref().map(|history| history.occurrences.len())
    );
    assert!(
        head.is_err(),
        "the provider accepted a replaced history for this handle"
    );
    assert!(
        selected.is_err(),
        "the head read refused the replaced store, but a read at the seed on the same handle \
         answered from the history it held before the replacement"
    );
}
