//! Object metadata and native blob bytes must be one provider publication.
use ekr_core::{ContentHash, SchemaVersionId, Timestamp};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{FileStore, ObjectStore, SqliteStore, StorageClass};
use eventlog_core::{EventStore, StreamId, TenantId};

fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .unwrap()
}

#[test]
fn new_object_events_are_schema_two_metadata_with_verified_native_blobs() {
    let bytes = b"synthetic retained object bytes";
    let hash = ContentHash::of_bytes(bytes);
    let mut failures = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state.db");
        if file {
            FileStore::file(directory.path(), "ekr", ontology())
                .unwrap()
                .put(StorageClass::Provenance, bytes, Timestamp::EPOCH)
                .unwrap();
        } else {
            SqliteStore::sqlite(&path, "ekr", ontology())
                .unwrap()
                .put(StorageClass::Provenance, bytes, Timestamp::EPOCH)
                .unwrap();
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let tenant = TenantId::new("ekr").unwrap();
        let stream = StreamId::new(tenant.clone(), "ekr.store.object", hash.to_hex()).unwrap();
        let (events, blob) = runtime.block_on(async {
            if file {
                let provider = eventlog_file::FileEventStore::open(directory.path())
                    .await
                    .unwrap();
                (
                    provider.read_stream(&stream, 0, 100).await.unwrap().events,
                    provider.get_blob(&tenant, &hash.to_hex()).await.unwrap(),
                )
            } else {
                let provider =
                    eventlog_sqlite::SqliteEventStore::open(path.to_str().unwrap(), "ekr")
                        .await
                        .unwrap();
                (
                    provider.read_stream(&stream, 0, 100).await.unwrap().events,
                    provider.get_blob(&tenant, &hash.to_hex()).await.unwrap(),
                )
            }
        });
        assert_eq!(events.len(), 1);
        let event = &events[0];
        if event.name != "ekr.store.ObjectStored"
            || event.schema_version != 2
            || event.data.get("bytes").is_some()
            || event.data.as_object().unwrap().len() != 4
            || blob.as_deref() != Some(bytes.as_slice())
        {
            failures.push(format!("file={file}: event={event:?}, blob={blob:?}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("; "));
}
