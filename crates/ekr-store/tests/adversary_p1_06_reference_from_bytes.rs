//! The old unchecked dangling-reference conversion is gone. Semantic refusal with real
//! authority is tested in ekr-kernel/tests/seed.rs; a store alone cannot admit even a seed.
mod fixture;
use ekr_core::{NodeId, RevisionId, Timestamp};
use ekr_graph::RevisionEvent;
use ekr_store::{GraphDocument, ObjectStore, RevisionLog, SqliteStore, StorageClass, StoreError};
#[test]
fn a_store_cannot_turn_dangling_document_bytes_into_canonical_state() {
    let directory = tempfile::TempDir::new().unwrap();
    let ontology = fixture::ontology();
    let mut document = GraphDocument::of(&fixture::seed_graph(&ontology));
    document.edges.values_mut().next().unwrap().target = NodeId::mint();
    let store = SqliteStore::sqlite(&directory.path().join("state.db"), "ekr", ontology).unwrap();
    let object = store
        .put(
            StorageClass::Canonical,
            &document.to_bytes().unwrap(),
            Timestamp::EPOCH,
        )
        .unwrap();
    let _appended = store
        .append(&RevisionEvent::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash: object.content_hash,
        })
        .unwrap();
    assert_eq!(store.fold(), Err(StoreError::NoSeedAuthority));
}
