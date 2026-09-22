//! § 89: "The original ObjectStored/schema1 body with inline bytes is frozen for verification and
//! preservation-first migration; it is not new-format ingestion after activation." A schema-1
//! inline record written straight through the provider, as an original store holds it, is
//! refused on every current read and write path that reaches it, on both providers, and its
//! provider records are left exactly as they were. The preserving migration is not this case.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ekr_core::{ContentHash, Timestamp};
use ekr_store::{FileStore, ObjectStore, SqliteStore, StorageClass, StoreError};
use eventlog_core::{
    CommandMeta, EventStore, Expected, NewEvent, RecordedEvent, StreamId, TenantId,
};

const BYTES: &[u8] = b"original inline object bytes";
const REFUSAL: &str = "unsupported-object-envelope: legacy inline records require migration";

fn meta() -> CommandMeta {
    CommandMeta {
        idempotency_key: "legacy-inline-object".into(),
        request_hash: "legacy-inline-object".into(),
        subject: "ekr.test".into(),
        actor: "ekr.test".into(),
        request_id: "legacy-inline-object".into(),
        trace_id: "legacy-inline-object".into(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: time::OffsetDateTime::UNIX_EPOCH,
        claim: None,
    }
}

fn stream(hash: ContentHash) -> StreamId {
    StreamId::new(
        TenantId::new("ekr").unwrap(),
        "ekr.store.object",
        hash.to_hex(),
    )
    .unwrap()
}

/// The provider's own view: the object stream's events and whether a blob is bound.
fn raw(path: &Path, file: bool, hash: ContentHash) -> (Vec<RecordedEvent>, Option<Vec<u8>>) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let tenant = TenantId::new("ekr").unwrap();
    runtime.block_on(async {
        if file {
            let provider = eventlog_file::FileEventStore::open(path).await.unwrap();
            (
                provider
                    .read_stream(&stream(hash), 0, 100)
                    .await
                    .unwrap()
                    .events,
                provider.get_blob(&tenant, &hash.to_hex()).await.unwrap(),
            )
        } else {
            let provider = eventlog_sqlite::SqliteEventStore::open(
                path.join("state.db").to_str().unwrap(),
                "ekr",
            )
            .await
            .unwrap();
            (
                provider
                    .read_stream(&stream(hash), 0, 100)
                    .await
                    .unwrap()
                    .events,
                provider.get_blob(&tenant, &hash.to_hex()).await.unwrap(),
            )
        }
    })
}

/// Writes one schema-1 `ObjectStored` with its bytes inline and no blob binding.
fn install(path: &Path, file: bool, hash: ContentHash) {
    let body = serde_json::json!({
        "content_hash": hash,
        "storage_class": "Canonical",
        "byte_len": BYTES.len(),
        "stored_at": Timestamp::EPOCH,
        "bytes": BYTES,
    });
    let event = NewEvent::new("ekr.store.ObjectStored", 1, body).unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        if file {
            let provider = eventlog_file::FileEventStore::open(path).await.unwrap();
            provider
                .append(&stream(hash), Expected::NoStream, &[event], &meta())
                .await
                .unwrap();
        } else {
            let provider = eventlog_sqlite::SqliteEventStore::open(
                path.join("state.db").to_str().unwrap(),
                "ekr",
            )
            .await
            .unwrap();
            provider
                .append(&stream(hash), Expected::NoStream, &[event], &meta())
                .await
                .unwrap();
        }
    });
}

fn bytes_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, into: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, into);
            } else {
                into.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut into = BTreeMap::new();
    visit(root, root, &mut into);
    into
}

fn refused<T: std::fmt::Debug>(outcome: Result<T, StoreError>) -> bool {
    matches!(&outcome, Err(StoreError::Document(message)) if message == REFUSAL)
}

#[test]
fn a_schema_one_inline_object_is_refused_on_read_and_left_untouched_on_both_providers() {
    let hash = ContentHash::of_bytes(BYTES);
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        install(directory.path(), file, hash);
        let before = raw(directory.path(), file, hash);
        assert_eq!(before.0.len(), 1);
        assert_eq!(before.0[0].schema_version, 1);
        assert_eq!(before.1, None, "an inline record has no blob binding");
        let files = file.then(|| bytes_under(directory.path()));

        let store: Box<dyn ObjectStore> = if file {
            Box::new(FileStore::file(directory.path(), "ekr", None).unwrap())
        } else {
            Box::new(SqliteStore::sqlite(&directory.path().join("state.db"), "ekr", None).unwrap())
        };
        let read = store.get(&hash);
        if !refused(read.clone()) {
            wrong.push(format!("file={file} get: {read:?}"));
        }
        let rewrite = store.put(StorageClass::Canonical, BYTES, Timestamp::EPOCH);
        if !refused(rewrite.clone()) {
            wrong.push(format!("file={file} put of the same bytes: {rewrite:?}"));
        }
        drop(store);

        if raw(directory.path(), file, hash) != before {
            wrong.push(format!("file={file}: provider records changed"));
        }
        if let Some(files) = files {
            if bytes_under(directory.path()) != files {
                wrong.push("file provider bytes changed".into());
            }
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
