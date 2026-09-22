//! The old unchecked dangling-reference conversion is gone. Semantic refusal with real
//! authority is tested in ekr-kernel/tests/seed.rs
//! (`a_seed_with_a_dangling_edge_is_refused_by_both_backends`); a store alone cannot admit even a
//! seed.
//!
//! The raw `append` this case used to write its seed through is gone too, so the refusal now lands
//! where a seed is written: `initialize` and `publish` both ask the injected authority, and a store
//! opened without one has nobody to ask. The document bytes themselves are retained — an object
//! store keeps what it is given — and are still not canonical state.
//!
//! Self-contained rather than declaring `tests/fixture`: this binary needs one document with one
//! dangling edge, not the shared seed graph.

use std::collections::BTreeMap;

use ekr_core::{
    ContentHash, EdgeId, EventId, GraphRootId, NodeId, RevisionId, RevisionNumber, SchemaVersionId,
    Timestamp, TypeId,
};
use ekr_graph::{Edge, GraphRoot, Node, RevisionEvent, RevisionPayload, Space};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion, Value};
use ekr_store::{
    GraphDocument, Initialize, ObjectStore, Publication, PublicationObject, RevisionLog,
    SqliteStore, StorageClass, StoreError,
};

/// An ontology with no declarations: the store type-checks nothing (invariant 7).
fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("a document with no declarations coheres")
}

/// A document holding one node and one edge from it to a node the document does not carry.
fn dangling(ontology: &Ontology) -> GraphDocument {
    let root_id = GraphRootId::mint();
    let type_id = TypeId::mint();
    let source = NodeId::mint();
    let edge = Edge::<Value>::new(EdgeId::mint(), root_id, type_id, source, NodeId::mint());
    GraphDocument {
        root: GraphRoot {
            id: root_id,
            space: Space::Canonical,
            schema_version_id: ontology.version().id,
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::SEED,
        nodes: BTreeMap::from([(source, Node::new(source, root_id, type_id, "edge-source"))]),
        edges: BTreeMap::from([(edge.id, edge)]),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    }
}

#[test]
fn a_store_cannot_turn_dangling_document_bytes_into_canonical_state() {
    let directory = tempfile::TempDir::new().unwrap();
    let ontology = ontology();
    let document = dangling(&ontology);
    let target = document.edges.values().next().unwrap().target;
    assert!(
        !document.nodes.contains_key(&target),
        "the fixture's edge must point at a node the document does not carry"
    );
    let store = SqliteStore::sqlite(&directory.path().join("state.db"), "ekr", ontology).unwrap();
    let bytes = document.to_bytes().unwrap();
    let object = store
        .put(StorageClass::Canonical, &bytes, Timestamp::EPOCH)
        .unwrap();

    let record = b"a seed result nothing validated";
    let record_hash = ContentHash::of_bytes(record);
    let seed = Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.to_owned(),
            event_id: EventId::mint(),
            record_hash,
            payload: RevisionPayload::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: object.content_hash,
            },
        },
        objects: BTreeMap::from([(
            record_hash,
            PublicationObject {
                storage_class: StorageClass::Canonical,
                stored_at: Timestamp::EPOCH,
                bytes: record.to_vec(),
            },
        )]),
        expected_version: 0,
    };
    assert_eq!(store.initialize(&seed), Err(StoreError::NoSeedAuthority));
    assert_eq!(store.publish(&seed), Err(StoreError::NoSeedAuthority));

    assert_eq!(store.head(), Ok(None), "no seed occurrence was written");
    assert_eq!(store.fold(), Err(StoreError::NotSeeded));
    assert_eq!(
        store.get(&object.content_hash),
        Ok(Some(bytes)),
        "the dangling document is retained as bytes and is still not canonical state"
    );
}
