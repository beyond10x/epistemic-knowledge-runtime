//! Adversary pass 1 on `task:read-verbs-open-a-read-only-store`, at the store: a read-only open
//! taken while another handle keeps writing. The File provider copies the store under a shared
//! lock on `writer.lock`; the SQLite provider serializes the database through a `mode=ro`
//! connection inside one read transaction. Either way every open must succeed — no torn copy —
//! and must hold every object whose write returned before the open began.
//!
//! Under load the SQLite case once failed with `SQLITE_CANTOPEN` (extended code 14): the writer
//! had a `-wal` beside the database without its `-shm`, which a `readonly_shm` connection cannot
//! create. The open reads such a store again a bounded number of times (`read_only.rs`,
//! `Attempt::ShmAbsent`); a failure here prints the full `StoreError`, SQLite's extended code
//! included, and a writer that fails ends the race with its own error rather than leaving the
//! reader opening forever.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use ekr_core::{ContentHash, SchemaVersionId, Timestamp};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{FileStore, ObjectStore, SqliteStore, StorageClass, StoreError};

const WRITES: usize = 60;

fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .unwrap()
}

fn object(n: usize) -> Vec<u8> {
    format!("object {n} written while read-only opens copy the store").into_bytes()
}

/// Runs `write(n)` for every `n` below [`WRITES`] on one thread while another thread keeps
/// opening the store read-only with `open` and checking, through `get`, that every object whose
/// write returned before that open began is there. Returns how many read-only opens ran.
fn race<R>(
    write: impl FnOnce() -> Box<dyn FnMut(usize)> + Send,
    open: impl Fn() -> Result<R, StoreError> + Sync,
    get: fn(&R, &ContentHash) -> Option<Vec<u8>>,
) -> usize {
    let acknowledged = AtomicUsize::new(0);
    let done = AtomicBool::new(false);
    let mut opens = 0;
    /// Sets `done` when the writer ends, by returning or by panicking.
    struct Finished<'a>(&'a AtomicBool);
    impl Drop for Finished<'_> {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            let _finished = Finished(&done);
            let mut write = write();
            for n in 0..WRITES {
                write(n);
                acknowledged.store(n + 1, Ordering::SeqCst);
            }
        });
        while !done.load(Ordering::SeqCst) || opens == 0 {
            if writer.is_finished() && acknowledged.load(Ordering::SeqCst) < WRITES {
                break;
            }
            let before = acknowledged.load(Ordering::SeqCst);
            let reader = open().unwrap_or_else(|error| {
                panic!("read-only open {opens} with {before} writes acknowledged: {error:?}")
            });
            for n in 0..before {
                let bytes = object(n);
                assert_eq!(
                    get(&reader, &ContentHash::of_bytes(&bytes)).as_deref(),
                    Some(bytes.as_slice()),
                    "read-only open {opens}: object {n} of {before} acknowledged is missing"
                );
            }
            opens += 1;
        }
    });
    opens
}

#[test]
fn a_file_store_opened_read_only_while_a_writer_writes_holds_every_acknowledged_object() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("store");
    drop(FileStore::file(&path, "ekr", ontology()).unwrap());
    let opens = race(
        || {
            let writer = FileStore::file_existing(&path, "ekr", ontology()).unwrap();
            Box::new(move |n| {
                writer
                    .put(StorageClass::Provenance, &object(n), Timestamp::EPOCH)
                    .unwrap();
            })
        },
        || FileStore::file_read_only(&path, "ekr", ontology()),
        |reader, hash| reader.get(hash).unwrap(),
    );
    assert!(opens > 1, "{opens} read-only opens raced the writer");
}

#[test]
fn a_sqlite_store_opened_read_only_while_a_writer_writes_holds_every_acknowledged_object() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("state.db");
    drop(SqliteStore::sqlite(&path, "ekr", ontology()).unwrap());
    let opens = race(
        || {
            let writer = SqliteStore::sqlite_existing(&path, "ekr", ontology()).unwrap();
            Box::new(move |n| {
                writer
                    .put(StorageClass::Provenance, &object(n), Timestamp::EPOCH)
                    .unwrap();
            })
        },
        || SqliteStore::sqlite_read_only(&path, "ekr", ontology()),
        |reader, hash| reader.get(hash).unwrap(),
    );
    assert!(opens > 1, "{opens} read-only opens raced the writer");
}
