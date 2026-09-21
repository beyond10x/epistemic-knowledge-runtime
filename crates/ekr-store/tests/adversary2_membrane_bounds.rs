//! The crossing's "cannot be checked here" list, taken at its word.
//!
//! `snapshot.rs` now says what it checks and what it does not, and the second half is the claim
//! worth driving:
//!
//! > Three fields are copied through unread and **cannot** be checked here, which is a bound
//! > rather than an omission: `root.parent` and `root.created_at` have nothing in the document to
//! > disagree with, and `revision` is the fold's to set.
//!
//! `root.parent` is a `GraphRootId` and the document carries another one three lines above it —
//! `root.id`. A root that is its own parent is a document disagreeing with itself, readable by the
//! same comparison the crossing already makes four times, so `parent` is on the wrong list.

mod fixture;

use ekr_store::GraphDocument;

/// A root that is its own parent is a disagreement inside the document, not a bound on the reader.
///
/// `GraphRoot.parent` is "the root it was derived from, if any". A root derived from itself is a
/// cycle of length one and is not a lineage; the crossing has both fields in hand and the
/// comparison is the same shape as the `root_id`-against-`root.id` check it already performs on
/// every node, edge and assertion.
#[test]
fn the_crossing_refuses_a_root_that_is_its_own_parent() {
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);

    let mut document = GraphDocument::of(&graph);
    document.root.parent = Some(document.root.id);

    match document.into_canonical(ontology) {
        Err(_) => {}
        Ok(crossed) => panic!(
            "a document whose root names itself as its own parent crossed into canonical state: \
             root {} has parent {:?}, and the module's documentation lists `root.parent` as \
             having nothing in the document to disagree with",
            crossed.root.id, crossed.root.parent
        ),
    }
}
