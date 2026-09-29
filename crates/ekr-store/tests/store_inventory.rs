//! `task:object-payloads-belong-in-provider-blobs`, at the store: the inventory a preserving
//! migration reads (design §§ 90, 100.3). It is the one reader of the frozen
//! `ObjectStored`/schema-1 inline body: a legacy object is read under that shape and its inline
//! bytes checked against its address and length, a current object as every read checks one, and
//! a retention raise is reported as the stream records it. Reading writes nothing, on both
//! providers; current reads keep refusing the legacy object, as before.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ekr_core::{ContentHash, Timestamp};
use ekr_store::{
    EventlogStore, FileStore, InventoriedObject, Inventory, ObjectStore, SqliteStore, StorageClass,
    StoreError, StoreInventory,
};
use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};

const INLINE: &[u8] = b"original inline object bytes";
const CURRENT: &[u8] = b"current object bytes";

fn stream(hash: ContentHash) -> StreamId {
    StreamId::new(
        TenantId::new("ekr").unwrap(),
        "ekr.store.object",
        hash.to_hex(),
    )
    .unwrap()
}

/// Writes one schema-1 `ObjectStored` with `body` as its inline record and no blob binding.
fn install(path: &Path, file: bool, hash: ContentHash, body: serde_json::Value) {
    let event = NewEvent::new("ekr.store.ObjectStored", 1, body).unwrap();
    let meta = CommandMeta {
        idempotency_key: format!("legacy-{hash}"),
        request_hash: "legacy".into(),
        subject: "ekr.test".into(),
        actor: "ekr.test".into(),
        request_id: "legacy".into(),
        trace_id: "legacy".into(),
        causation_id: None,
        causation_depth: 0,
        occurred_at: time::OffsetDateTime::UNIX_EPOCH,
        claim: None,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let provider: Box<dyn EventStore> = if file {
            Box::new(eventlog_file::FileEventStore::open(path).await.unwrap())
        } else {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(
                    path.join("state.db").to_str().unwrap(),
                    "ekr",
                )
                .await
                .unwrap(),
            )
        };
        provider
            .append(&stream(hash), Expected::NoStream, &[event], &meta)
            .await
            .unwrap();
    });
}

fn inline_body(hash: ContentHash, bytes: &[u8]) -> serde_json::Value {
    serde_json::json!({
        "content_hash": hash,
        "storage_class": "Provenance",
        "byte_len": INLINE.len(),
        "stored_at": Timestamp::from_millis(5),
        "bytes": bytes,
    })
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

/// The inventory through the trait a migration reads it by.
fn inventory_of(store: &impl Inventory) -> Result<StoreInventory, StoreError> {
    store.inventory()
}

fn read<S: eventlog_core::AtomicBlobEventStore>(
    store: &EventlogStore<S>,
) -> Result<StoreInventory, StoreError> {
    let direct = store.inventory();
    assert_eq!(direct, inventory_of(store));
    direct
}

#[test]
fn an_inventory_reads_legacy_and_current_objects_and_writes_nothing_on_both_providers() {
    let legacy = ContentHash::of_bytes(INLINE);
    let current = ContentHash::of_bytes(CURRENT);
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        install(path, file, legacy, inline_body(legacy, INLINE));
        let inventory = if file {
            let store = FileStore::file(path, "ekr", None).unwrap();
            store
                .put(StorageClass::Cache, CURRENT, Timestamp::from_millis(7))
                .unwrap();
            store
                .put(StorageClass::Canonical, CURRENT, Timestamp::from_millis(8))
                .unwrap();
            drop(store);
            let before = bytes_under(path);
            let store = FileStore::file_existing(path, "ekr", None).unwrap();
            let inventory = read(&store).unwrap();
            drop(store);
            assert_eq!(bytes_under(path), before, "reading wrote nothing");
            inventory
        } else {
            let store = SqliteStore::sqlite(&path.join("state.db"), "ekr", None).unwrap();
            store
                .put(StorageClass::Cache, CURRENT, Timestamp::from_millis(7))
                .unwrap();
            store
                .put(StorageClass::Canonical, CURRENT, Timestamp::from_millis(8))
                .unwrap();
            let events = store.published_events().unwrap();
            let inventory = read(&store).unwrap();
            assert_eq!(
                store.published_events().unwrap(),
                events,
                "reading wrote nothing"
            );
            inventory
        };
        assert!(inventory.occurrences.is_empty(), "file={file}");
        assert!(inventory.prepared.is_empty(), "file={file}");
        assert_eq!(inventory.events, 3, "file={file}: two stored, one raise");
        let held: &InventoriedObject = &inventory.objects[&legacy];
        assert!(held.legacy, "file={file}");
        assert_eq!(*held.object.bytes, INLINE);
        assert_eq!(held.object.metadata.content_hash, legacy);
        assert_eq!(held.object.metadata.storage_class, StorageClass::Provenance);
        assert_eq!(held.object.metadata.stored_at, Timestamp::from_millis(5));
        assert_eq!(held.stored_as, StorageClass::Provenance);
        assert!(held.raised_to.is_empty());
        let held = &inventory.objects[&current];
        assert!(!held.legacy, "file={file}");
        assert_eq!(*held.object.bytes, CURRENT);
        assert_eq!(held.stored_as, StorageClass::Cache);
        assert_eq!(held.raised_to, [StorageClass::Canonical]);
        assert_eq!(held.object.metadata.storage_class, StorageClass::Canonical);
        assert_eq!(held.object.metadata.stored_at, Timestamp::from_millis(7));
        assert_eq!(inventory.objects.len(), 2);
    }
}

#[test]
fn an_inline_object_whose_bytes_are_not_its_address_is_refused_on_both_providers() {
    let claimed = ContentHash::of_bytes(INLINE);
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let mut other = INLINE.to_vec();
        other[0] ^= 1;
        install(path, file, claimed, inline_body(claimed, &other));
        let refused = if file {
            FileStore::file(path, "ekr", None).unwrap().inventory()
        } else {
            SqliteStore::sqlite(&path.join("state.db"), "ekr", None)
                .unwrap()
                .inventory()
        };
        assert_eq!(
            refused,
            Err(StoreError::Document(
                "object-integrity: address or byte length disagrees".into()
            )),
            "file={file}"
        );
    }
}
