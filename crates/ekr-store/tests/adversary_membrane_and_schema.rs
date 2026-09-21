//! What the one named membrane crossing does not look at.
//!
//! `task:the-membrane-stops-at-the-store-boundary` says the document is not self-describing
//! because `ekr.graph.Space` sits on `GraphRoot` and not on a node. A [`GraphDocument`] carries
//! that `GraphRoot`, so at the *document* level the marker is present and readable; the crossing
//! ignores it, and it ignores the schema version beside it.

mod fixture;

use ekr_core::{RevisionId, Timestamp};
use ekr_graph::{RevisionEvent, Space};
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
}

/// The one named crossing reads the document's own `GraphRoot`, and `GraphRoot` carries `space`.
///
/// `snapshot.rs`'s module documentation says the crossing "does not make a candidate's document
/// distinguishable from a canonical one" and cites `tests/membrane_boundary.rs` as pinning that.
/// That case compares a candidate **node** against a canonical **node**, which is true and is a
/// different claim: a `GraphDocument` is not a node, it is a root plus four maps, and the root
/// says which space the state belongs to. So the bytes do carry the marker the task says they do
/// not, and the crossing has it in hand and does not read it.
#[test]
fn the_crossing_refuses_a_document_whose_root_declares_transient_space() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let mut document = GraphDocument::of(&graph);
    document.root.space = Space::Transient;

    match document.into_canonical(ontology) {
        Err(_) => {}
        Ok(crossed) => panic!(
            "a document whose root declares {:?} crossed into canonical state, and the state it \
             produced is rooted in {:?}: the one named crossing had the space marker in hand and \
             did not read it",
            Space::Transient,
            crossed.root.space
        ),
    }
}

/// The same thing, reached through the store's public surface and nothing else.
///
/// `put` takes bytes, `Seeded` names an address, and `fold` resolves the address and crosses what
/// it finds. Nothing between them asks which space wrote the document, so a transient root's
/// snapshot folds into canonical state — which is the silent failure
/// `task:the-membrane-stops-at-the-store-boundary` names under "What reaches it".
#[test]
fn a_fold_does_not_return_canonical_state_rooted_in_a_transient_root() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let mut document = GraphDocument::of(&graph);
    document.root.space = Space::Transient;
    let bytes = document.to_bytes().expect("the document serialises");

    let store = store(&directory, &ontology);
    let seed = store
        .put(StorageClass::Canonical, &bytes, Timestamp::EPOCH)
        .expect("the bytes land");
    assert_eq!(
        store
            .append(&RevisionEvent::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: seed.content_hash,
            })
            .expect("the append itself is not the check"),
        Appended::Written,
        "a new fact, not a retry: the append itself is not the check"
    );

    match store.fold() {
        Err(_) => {}
        Ok(folded) => assert_eq!(
            folded.root.space,
            Space::Canonical,
            "the fold answered with canonical state hanging off a root that declares itself \
             transient"
        ),
    }
}

/// A `CanonicalGraph` says twice which schema it is typed by, and the fold lets the two disagree.
///
/// `GraphRoot.schema_version_id` is "the schema version its contents are typed by" and
/// `CanonicalGraph.ontology` is "the schema its contents are valid against". The store takes the
/// ontology as an **open-time argument** and `GraphDocument::into_canonical` puts the caller's
/// ontology beside the stored root without comparing them, so opening a store against a different
/// schema version silently re-types every folded record and reports nothing. The head `Root` does
/// not move either — `knowledge_root` hashes nodes, edges and assertions and not the root — so the
/// acceptance case cannot see it.
#[test]
fn a_fold_does_not_retype_stored_state_under_a_schema_version_it_was_not_written_against() {
    let directory = TempDir::new().expect("a temporary directory");
    let written_against = fixture::ontology();
    let opened_with = fixture::ontology();
    assert_ne!(
        written_against.version().id,
        opened_with.version().id,
        "two loads of the fixture are two schema versions"
    );

    let graph = fixture::seed_graph(&written_against);
    let bytes = GraphDocument::of(&graph)
        .to_bytes()
        .expect("the document serialises");

    let store = store(&directory, &opened_with);
    let seed = store
        .put(StorageClass::Canonical, &bytes, Timestamp::EPOCH)
        .expect("the bytes land");
    assert_eq!(
        store
            .append(&RevisionEvent::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: seed.content_hash,
            })
            .expect("seeded"),
        Appended::Written,
        "a new fact, not a retry: seeded"
    );

    match store.fold() {
        Err(_) => {}
        Ok(folded) => assert_eq!(
            folded.root.schema_version_id,
            folded.ontology.version().id,
            "the folded state is typed by one schema version and validated against another"
        ),
    }
}
