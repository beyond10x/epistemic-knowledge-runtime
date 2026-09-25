//! Adversary pass 2 on `story:eventlog-0-4-batched-reads`.
//!
//! `EventlogStore::required` (`crates/ekr-store/src/eventlog.rs`) documents that a batched load
//! "refuses what the per-object load refused, and first the refusal it met first", and names one
//! exception only: "a provider failure of the stream batch as a whole comes before any blob
//! check". The blob batch is one `read_many` call too. On SQLite that call is eventlog-core's
//! default, which returns the first failing read's error for the whole batch, so a provider
//! failure on a later object's blob pre-empts the refusal the per-object load gave an earlier one.

use std::path::Path;

use ekr_core::{ContentHash, EventId, RevisionId, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_store::{ObjectStore, RevisionLog, SqliteStore, StoreError};
use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};
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

fn tenant() -> TenantId {
    TenantId::new(TENANT).unwrap()
}

fn object_stream(hash: ContentHash) -> StreamId {
    StreamId::new(tenant(), "ekr.store.object", hash.to_hex()).unwrap()
}

/// An intact current object: its `ObjectStored` schema 2 stream and its blob.
async fn intact_object<S: EventStore>(provider: &S, bytes: &[u8], key: &str) -> ContentHash {
    let hash = ContentHash::of_bytes(bytes);
    provider
        .put_blob(&tenant(), &hash.to_hex(), bytes)
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
    let stream = StreamId::new(tenant(), "ekr.revision", "canonical").unwrap();
    let event = NewEvent::new(seeded.name(), 2, serde_json::to_value(&seeded).unwrap()).unwrap();
    provider
        .append(
            &stream,
            Expected::NoStream,
            &[event],
            &metadata("adversary2-revision"),
        )
        .await
        .unwrap();
}

/// Flips one byte inside every copy of `bytes` in the SQLite database and its WAL, as disk damage
/// would; the SQLite provider's blob read hashes what it serves and refuses the damaged blob.
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

fn payload(label: &str) -> Vec<u8> {
    format!("{label}:{}", "adversary2-p1-15-blob-batch-".repeat(8)).into_bytes()
}

/// Two objects in the order the load visits them. The first has its stream and no blob; the
/// second has its stream and a blob damaged on disk. The per-object load read the first object's
/// stream, then its blob, and refused it `object-integrity: native blob missing` before it read
/// anything of the second. The batched load reads both blobs in one `read_many`, and the second
/// blob's provider refusal fails that call before the first object's blob check runs.
///
/// Rewritten by the coordinator to today's documented state (`story:eventlog-0-4-batched-reads`,
/// `EventlogStore::required` doc comment): the blob batch's provider refusal comes first. The load
/// still refuses; only the refusal differs. Invert this when the blob batch reads per object.
#[test]
fn a_later_blob_provider_failure_comes_before_an_earlier_objects_refusal_as_documented() {
    let directory = TempDir::new().unwrap();
    let db = directory.path().join("state.db");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let damaged_bytes = payload("damaged later");
    let damaged = ContentHash::of_bytes(&damaged_bytes);
    let missing_bytes = (0u32..)
        .map(|n| payload(&format!("blob-less earlier {n}")))
        .find(|bytes| ContentHash::of_bytes(bytes) < damaged)
        .unwrap();
    let missing;
    {
        let provider = rt
            .block_on(SqliteEventStore::open(db.to_str().unwrap(), "ekr"))
            .unwrap();
        missing = rt.block_on(intact_object(
            &provider,
            &missing_bytes,
            "adversary2-missing",
        ));
        rt.block_on(intact_object(
            &provider,
            &damaged_bytes,
            "adversary2-damaged",
        ));
        rt.block_on(provider.delete_blob(&tenant(), &missing.to_hex()))
            .unwrap();
        rt.block_on(seeded(&provider, missing, damaged));
    }
    drop(rt);
    damage_sqlite_blob(&db, &damaged_bytes);
    assert!(missing < damaged, "the blob-less object is visited first");

    let store = SqliteStore::sqlite(&db, TENANT, None).unwrap();
    // The single-object read of the first object in order: its refusal.
    let single = store.get(&missing).unwrap_err().to_string();
    assert!(
        single.contains("object-integrity: native blob missing"),
        "single read of the blob-less object: {single}"
    );
    // The single-object read of the second: a provider refusal, which the per-object load never
    // reached because the first object had already been refused.
    let later = store.get(&damaged).unwrap_err().to_string();
    assert_ne!(single, later, "the two faults are distinguishable");
    // The history load visits the same two objects in the same order.
    let batched = store.head().unwrap_err();
    assert!(
        matches!(&batched, StoreError::Backend(_)),
        "batched load refused as {batched:?}; the documented order puts the blob batch's provider \
         refusal first (story:eventlog-0-4-batched-reads). If it now refuses as `{single}`, the \
         blob batch reads per object: invert this case and the doc comment"
    );
}
