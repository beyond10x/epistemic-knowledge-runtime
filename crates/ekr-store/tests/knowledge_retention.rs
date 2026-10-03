//! Independent physical retention pins exact document bytes and refuses changed coordinates.
use ekr_core::contract_data::EkrIntegrateRetainedInterpretation;
use ekr_core::generated_identity::{Identity, InterpretationId};
use ekr_core::{bytes, ContentHash, GraphRootId, SchemaVersionId, Timestamp};
use ekr_store::{IncubationRetention, ObjectStore};

#[test]
fn incubation_port_pins_document_bytes_on_both_providers() {
    fn check(store: impl IncubationRetention + ObjectStore) {
        let id = InterpretationId::mint();
        let root = GraphRootId::mint();
        let document = serde_json::json!({"version":{"interpretation_id":id,"version":1},"root_id":root,
            "observations":[],"local_schema":{"node_types":[],"edge_types":[]},"entities":[],"facts":[],"evidence":[]});
        let payload = serde_json::to_vec_pretty(&document).unwrap();
        let hash = ContentHash::of_bytes(&payload);
        let record: EkrIntegrateRetainedInterpretation = serde_json::from_value(serde_json::json!({
            "version":{"interpretation_id":id,"version":1,"document_digest":hash.to_hex()},
            "interpretation":{"document":document,"blockers":[],"receipts":[]},"payload":bytes::encode(&payload),
            "root":{"id":root,"space":"Transient","schema_version_id":SchemaVersionId::mint(),"created_at":"1970-01-01T00:00:00Z"}
        })).unwrap();
        assert!(
            store
                .retain_interpretation(&record, Timestamp::EPOCH)
                .unwrap()
                .1
        );
        assert!(
            !store
                .retain_interpretation(&record, Timestamp::from_millis(5))
                .unwrap()
                .1
        );
        assert_eq!(
            store.retained_interpretations().unwrap(),
            std::slice::from_ref(&record)
        );
        assert_eq!(store.get(&hash).unwrap().unwrap(), payload);
        let mut changed = record;
        changed.payload = bytes::encode(b"{}");
        assert!(store
            .retain_interpretation(&changed, Timestamp::EPOCH)
            .is_err());
        assert_eq!(store.retained_interpretations().unwrap().len(), 1);
    }
    let dir = tempfile::tempdir().unwrap();
    check(ekr_store::FileStore::file(&dir.path().join("file"), "test", None).unwrap());
    check(ekr_store::SqliteStore::sqlite(&dir.path().join("sqlite"), "test", None).unwrap());
}
