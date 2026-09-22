//! Two claims the store's own comments make, driven against the store.
//!
//! One is about the object store's retention answer when the same bytes arrive twice under two
//! classes; the other is about the idempotency key `RevisionLog::append` puts on an envelope.

mod fixture;

use ekr_core::{RevisionId, Timestamp};
use ekr_graph::RevisionEvent;
use ekr_store::{Appended, GraphDocument, ObjectStore, RevisionLog, SqliteStore, StorageClass};
use tempfile::TempDir;

/// A SQLite store over a fresh database inside `directory`, typed by `ontology`.
fn store(directory: &TempDir, ontology: &ekr_ontology::Ontology) -> SqliteStore {
    SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        ontology.clone(),
    )
    .expect("the SQLite provider opens")
    .under(fixture::SeedOnly)
}

/// Content addressing merges two retention answers into the weaker one, silently.
///
/// `StorageClass` is what a reclamation sweep reads: `Cache` is "reproducible, and freely
/// deletable" and `Canonical` is "durable, revisioned, strongly governed". `ObjectStore::put`
/// keys on the bytes alone and answers a later write with the earlier record, so bytes first
/// stored as a cache entry stay freely deletable after `store_graph` has named them a lineage's
/// seed. `providers.rs`'s `a_stored_object_carries_the_class_it_was_written_under` writes two
/// different payloads under two classes and never the same payload twice, so it does not see this.
#[test]
fn storing_canonical_bytes_that_were_cached_earlier_records_them_as_canonical() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = store(&directory, &ontology);
    let bytes = b"a payload that was cached before it was canonical";

    let cached = store
        .put(StorageClass::Cache, bytes, Timestamp::EPOCH)
        .expect("the cache write lands");
    assert_eq!(cached.storage_class, StorageClass::Cache);

    let canonical = store
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .expect("the canonical write is answered");

    assert_eq!(
        canonical.storage_class,
        StorageClass::Canonical,
        "bytes canonical state depends on are recorded as freely deletable, because a cache write \
         reached them first"
    );
}

/// `append`'s idempotency key moves with the stream head, so it never dedupes anything.
///
/// `eventlog.rs` says of the key it builds: "The key is the position this append claims, so
/// retrying an interrupted append writes the same event once rather than twice." The position it
/// claims is read from `stream_version` *before* the append, and a landed append moves that
/// version — so a retry after a lost acknowledgement computes a different key and the provider,
/// which dedupes on `(tenant, stream_type, stream_id, idempotency_key)`, sees a new command. The
/// byte-identical event is written a second time.
///
/// Observed through the seed, because a doubled seed is the one duplicate the fold names:
/// `fold` answers `SeedIsNotFirst` for a lineage the caller appended once and retried once.
#[test]
fn retrying_an_append_writes_the_same_event_once_rather_than_twice() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let store = store(&directory, &ontology);

    let bytes = GraphDocument::of(&graph)
        .to_bytes()
        .expect("the document serialises");
    let seed = store
        .put(StorageClass::Canonical, &bytes, Timestamp::EPOCH)
        .expect("the seed lands");
    let event = RevisionEvent::Seeded {
        revision_id: RevisionId::mint(),
        seed_hash: seed.content_hash,
    };

    assert_eq!(
        store.append(&event).expect("the first append lands"),
        Appended::Written,
        "a new fact, not a retry: the first append lands"
    );
    // The retry, and the one append in this suite that must *not* write. Correction round 2 made
    // `append` say which happened, so the retry is not merely harmless — it is legible.
    assert_eq!(
        store
            .append(&event)
            .expect("the retry of that same append is answered"),
        Appended::AlreadyRecorded,
        "a byte-identical append is recognised as the retry it is, and says so"
    );

    let folded = store.fold();
    assert!(
        folded.is_ok(),
        "one append retried once produced two events: {folded:?}"
    );
}
