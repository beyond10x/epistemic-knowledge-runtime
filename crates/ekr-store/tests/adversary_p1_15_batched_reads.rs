//! Adversary pass 1 on `story:eventlog-0-4-batched-reads`.
//!
//! `EventlogStore::required` (`crates/ekr-store/src/eventlog.rs`) documents: "An absent object is
//! refused in its place in that order", and "a batched load refuses exactly what a single read
//! refuses". These cases drive the batched history load against that sentence and against the
//! single-object read (`ObjectStore::get`), through the SQLite and file providers, on stores written straight
//! through the provider as a damaged store would hold them.

use std::path::Path;
use std::time::Duration;

use ekr_core::{ContentHash, EventId, RevisionId, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_store::{FileStore, ObjectStore, RevisionLog, SqliteStore, StorageClass, StoreError};
use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};
use eventlog_file::FileEventStore;
use eventlog_sqlite::SqliteEventStore;
use tempfile::TempDir;

const TENANT: &str = "ekr";

fn metadata(key: &str) -> CommandMeta {
    CommandMeta {
        idempotency_key: key.to_owned(),
        request_hash: key.to_owned(),
        subject: "ekr.test".to_owned(),
        actor: "ekr.test".to_owned(),
        request_id: key.to_owned(),
        trace_id: key.to_owned(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: time::OffsetDateTime::UNIX_EPOCH,
        claim: None,
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

fn object_stream(hash: ContentHash) -> StreamId {
    StreamId::new(
        TenantId::new(TENANT).unwrap(),
        "ekr.store.object",
        hash.to_hex(),
    )
    .unwrap()
}

/// An intact current object: its `ObjectStored` schema 2 stream and its blob.
async fn intact_object<S: EventStore>(provider: &S, bytes: &[u8], key: &str) -> ContentHash {
    let hash = ContentHash::of_bytes(bytes);
    let tenant = TenantId::new(TENANT).unwrap();
    provider
        .put_blob(&tenant, &hash.to_hex(), bytes)
        .await
        .unwrap();
    let body = serde_json::json!({
        "content_hash": hash, "storage_class": "Canonical", "byte_len": bytes.len(),
        "stored_at": Timestamp::EPOCH,
    });
    let event = NewEvent::new("ekr.store.ObjectStored", 2, body).unwrap();
    provider
        .append(
            &object_stream(hash),
            Expected::NoStream,
            &[event],
            &metadata(key),
        )
        .await
        .unwrap();
    hash
}

async fn seeded<S: EventStore>(provider: &S, record: ContentHash, seed: ContentHash) {
    let seeded = RevisionEvent {
        format: RevisionEvent::FORMAT.to_owned(),
        event_id: EventId::mint(),
        record_hash: record,
        payload: RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: seed,
        },
    };
    let stream =
        StreamId::new(TenantId::new(TENANT).unwrap(), "ekr.revision", "canonical").unwrap();
    let event = NewEvent::new(seeded.name(), 2, serde_json::to_value(&seeded).unwrap()).unwrap();
    provider
        .append(
            &stream,
            Expected::NoStream,
            &[event],
            &metadata("adversary-revision"),
        )
        .await
        .unwrap();
}

/// Flips one byte inside every copy of `bytes` in the SQLite database and its WAL, as disk damage
/// would. The file provider cannot be damaged this way: it hashes every blob at open and refuses
/// the whole store, so these faults are written into a SQLite file database, whose blob read
/// hashes the bytes it serves.
fn damage_sqlite_blob(db: &Path, bytes: &[u8]) {
    let mut damaged = 0;
    for path in [db.to_path_buf(), db.with_extension("db-wal")] {
        let Ok(mut contents) = std::fs::read(&path) else {
            continue;
        };
        let mut at = 0;
        while let Some(found) = contents[at..]
            .windows(bytes.len())
            .position(|window| window == bytes)
        {
            let index = at + found + bytes.len() / 2;
            contents[index] ^= 1;
            damaged += 1;
            at = at + found + bytes.len();
        }
        std::fs::write(&path, contents).unwrap();
    }
    assert!(damaged > 0, "the victim's bytes were found in the database");
}

/// A long, distinctive payload, so a search of the database file finds only the blob.
fn payload(label: &str) -> Vec<u8> {
    format!("{label}:{}", "adversary-p1-15-blob-body-".repeat(8)).into_bytes()
}

fn sqlite_provider(rt: &tokio::runtime::Runtime, db: &Path) -> SqliteEventStore {
    rt.block_on(SqliteEventStore::open(db.to_str().unwrap(), "ekr"))
        .unwrap()
}

/// The seed names an object whose stream does not exist. The single-object read calls that object
/// absent, and the per-object load refused it as `required-object-missing` without reading its
/// blob. A blob is bound under that address and damaged on disk.
///
/// The batched load reads every object's blob before it looks at any object's stream, so the
/// provider's integrity refusal of a blob nobody needed replaces the missing-object refusal.
#[test]
fn an_absent_object_is_refused_as_absent_by_the_batched_load_as_by_the_single_read() {
    let directory = TempDir::new().unwrap();
    let db = directory.path().join("state.db");
    let rt = runtime();
    let seed_bytes = payload("absent seed");
    let seed = ContentHash::of_bytes(&seed_bytes);
    {
        let provider = sqlite_provider(&rt, &db);
        let record = rt.block_on(intact_object(
            &provider,
            &payload("record"),
            "adversary-record",
        ));
        // A blob bound at the seed's address with no object stream behind it.
        rt.block_on(provider.put_blob(
            &TenantId::new(TENANT).unwrap(),
            &seed.to_hex(),
            &seed_bytes,
        ))
        .unwrap();
        rt.block_on(seeded(&provider, record, seed));
    }
    drop(rt);
    damage_sqlite_blob(&db, &seed_bytes);

    let store = SqliteStore::sqlite(&db, TENANT, None).unwrap();
    // The single-object read: the object is absent.
    assert_eq!(
        store.get(&seed).unwrap(),
        None,
        "single read: an object with no stream is absent"
    );
    // The history load must refuse the same object the same way.
    let refused = store.head().unwrap_err();
    assert!(
        matches!(&refused, StoreError::Document(code) if code == "required-object-missing"),
        "batched load refused an absent object as {refused:?}; the per-object load and the \
         single read call it absent (`required-object-missing`)"
    );
}

/// Two faults, one per object, in the order the load visits them: the seed is absent and sorts
/// first, and the record's blob is damaged on disk. The doc comment promises the absent object is
/// refused "in its place in that order"; the per-object load answered `required-object-missing`.
#[test]
fn the_first_object_in_order_decides_the_refusal_of_a_batched_load() {
    let directory = TempDir::new().unwrap();
    let db = directory.path().join("state.db");
    let rt = runtime();
    let record_bytes = payload("ordered record");
    let record_hash = ContentHash::of_bytes(&record_bytes);
    // An absent seed whose address sorts before the record's.
    let seed = (0u32..)
        .map(|n| ContentHash::of_bytes(format!("adversary absent seed {n}").as_bytes()))
        .find(|candidate| *candidate < record_hash)
        .unwrap();
    {
        let provider = sqlite_provider(&rt, &db);
        let record = rt.block_on(intact_object(&provider, &record_bytes, "adversary-record"));
        rt.block_on(seeded(&provider, record, seed));
    }
    drop(rt);
    damage_sqlite_blob(&db, &record_bytes);

    let store = SqliteStore::sqlite(&db, TENANT, None).unwrap();
    assert_eq!(
        store.get(&seed).unwrap(),
        None,
        "single read: the seed is absent"
    );
    let refused = store.head().unwrap_err();
    assert!(
        matches!(&refused, StoreError::Document(code) if code == "required-object-missing"),
        "batched load refused as {refused:?}; the first object in order is absent, and the \
         per-object load refused it as `required-object-missing` before it read the record"
    );
}

/// A redacted `ObjectStored` event is refused by the single read and by the batched load alike.
#[test]
fn a_redacted_object_event_is_refused_alike_by_the_single_and_the_batched_path() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    let rt = runtime();
    let (record, seed);
    {
        let provider = rt.block_on(FileEventStore::open(root)).unwrap();
        record = rt.block_on(intact_object(&provider, b"adversary redaction record", "r"));
        seed = rt.block_on(intact_object(&provider, b"adversary redaction seed", "s"));
        rt.block_on(provider.redact(&object_stream(seed), 1, "adversary redaction"))
            .unwrap();
        rt.block_on(seeded(&provider, record, seed));
    }
    drop(rt);
    let store = FileStore::file(root, TENANT, None).unwrap();
    let single = store.get(&seed).unwrap_err().to_string();
    let batched = store.head().unwrap_err().to_string();
    assert!(
        single.contains("stream-envelope-disagrees"),
        "single: {single}"
    );
    assert_eq!(single, batched, "the two paths refused differently");
}

/// Eventlog 0.4.0 trusts an inode stamp taken two seconds after the journal last changed. A second
/// handle on the same directory — another `ekr` process — writes after the first handle's stamp is
/// trusted; the first handle must still read that write.
#[test]
fn a_write_by_another_handle_after_the_stamp_is_trusted_is_read() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    let first = FileStore::file(root, TENANT, None).unwrap();
    let early = first
        .put(
            StorageClass::Canonical,
            b"adversary early object",
            Timestamp::EPOCH,
        )
        .unwrap();
    std::thread::sleep(Duration::from_millis(2_300));
    // This read is the one that may now keep a trusted stamp.
    assert!(first.get(&early.content_hash).unwrap().is_some());
    let second = FileStore::file(root, TENANT, None).unwrap();
    let late = second
        .put(
            StorageClass::Canonical,
            b"adversary late object",
            Timestamp::EPOCH,
        )
        .unwrap();
    assert_eq!(
        first.get(&late.content_hash).unwrap().as_deref(),
        Some(&b"adversary late object"[..]),
        "a handle whose stamp was trusted did not see another handle's write"
    );
}
