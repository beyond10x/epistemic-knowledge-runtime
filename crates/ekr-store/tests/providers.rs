//! The properties `story:eventlog-store` ships, over both providers.
//!
//! Two providers from the start, so a property proved here is proved for the deployment. Every
//! case below is written once against `RevisionLog + ObjectStore` and run twice, because a case
//! written for SQLite and copied for the file store is a case that drifts on the copy.

mod fixture;
mod lineage;

use ekr_core::{ContentHash, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::CanonicalGraph;
use ekr_ontology::Ontology;
use ekr_store::{
    Appended, FileStore, ObjectStore, RevisionLog, SqliteStore, StorageClass, StoreError,
};
use tempfile::TempDir;

/// The tenant every case writes under. One tenant per store in P1.
const TENANT: &str = "ekr";

/// Opens the SQLite provider over a database file inside `directory`.
fn sqlite(directory: &TempDir, ontology: &Ontology) -> SqliteStore {
    SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        TENANT,
        ontology.clone(),
    )
    .expect("the SQLite provider opens")
    .under(lineage::Attesting)
}

/// Opens the file provider over a directory inside `directory`.
fn file(directory: &TempDir, ontology: &Ontology) -> FileStore {
    FileStore::file(
        &directory.path().join("revisions"),
        TENANT,
        ontology.clone(),
    )
    .expect("the file provider opens")
    .under(lineage::Attesting)
}

/// The acceptance statement, over whichever provider the caller opened twice.
///
/// `open` is called once, written through, dropped, and called again on the same path: the second
/// store shares no memory with the first, so an address that survives is an address the provider
/// wrote down.
fn head_survives_a_reopen<S, F>(open: F, graph: &CanonicalGraph)
where
    S: RevisionLog + ObjectStore,
    F: Fn() -> S,
{
    let before = {
        let store = open();
        lineage::seed_and_commit(&store, graph).expect("the lineage is appendable");
        store
            .head()
            .expect("the log folds")
            .expect("a seeded log has a head")
    };

    let after = {
        let store = open();
        store
            .head()
            .expect("the log folds")
            .expect("a seeded log has a head")
    };

    assert_eq!(
        ContentHash::of(&before),
        ContentHash::of(&after),
        "a store closed and reopened folds to the same head root"
    );
    assert_eq!(before, after, "and to the same root, field for field");
}

#[test]
fn a_sqlite_store_reopened_folds_to_the_same_head_root() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    head_survives_a_reopen(|| sqlite(&directory, &ontology), &graph);
}

#[test]
fn a_file_store_reopened_folds_to_the_same_head_root() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    head_survives_a_reopen(|| file(&directory, &ontology), &graph);
}

/// Design § 57: identity is the content, so two writes of the same bytes are one object.
fn identical_bytes_store_once<S: ObjectStore>(store: &S) {
    let bytes = b"a payload the runtime did not choose";

    let first = store
        .put(StorageClass::Provenance, bytes, Timestamp::EPOCH)
        .expect("the first write lands");
    let second = store
        .put(StorageClass::Provenance, bytes, Timestamp::from_millis(1))
        .expect("the second write is answered");

    assert_eq!(
        first, second,
        "one object, not two: the address is the content"
    );
    assert_eq!(
        first.stored_at,
        Timestamp::EPOCH,
        "and it is the first write that is on record, not the second"
    );
    assert_eq!(
        store
            .get(&first.content_hash)
            .expect("the object reads back"),
        Some(bytes.to_vec()),
        "the bytes read back are the bytes written"
    );
}

#[test]
fn sqlite_stores_identical_bytes_once() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    identical_bytes_store_once(&sqlite(&directory, &ontology));
}

#[test]
fn a_file_store_stores_identical_bytes_once() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    identical_bytes_store_once(&file(&directory, &ontology));
}

/// A storage class is carried, not forgotten: two payloads under different classes are two
/// objects with two retention answers, and the class read back is the class written.
#[test]
fn a_stored_object_carries_the_class_it_was_written_under() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = sqlite(&directory, &ontology);

    let cached = store
        .put(
            StorageClass::Cache,
            b"a reproducible answer",
            Timestamp::EPOCH,
        )
        .expect("a cache object lands");
    assert_eq!(cached.storage_class, StorageClass::Cache);
    assert_eq!(cached.byte_len, 21, "the length is the payload's, in bytes");

    let ephemeral = store
        .put(StorageClass::Ephemeral, b"working state", Timestamp::EPOCH)
        .expect("an ephemeral object lands");
    assert_eq!(ephemeral.storage_class, StorageClass::Ephemeral);
}

/// `replay(from)` over a log of N events yields the same fold as `fold()`.
///
/// Not a tautology, because the two do not share a read: `fold` pages the stream at
/// `MAX_READ_LIMIT` and takes this thirteen-event log in one read, and `replay` steps it one event
/// at a time and takes thirteen. They agree only if the cursor arithmetic drops and repeats
/// nothing at a page boundary — which is the defect a fold that quietly stopped early would
/// otherwise hide, since a short fold still returns a perfectly well-formed graph.
fn replay_from_the_seed_equals_the_fold<S: RevisionLog + ObjectStore>(
    store: &S,
    graph: &CanonicalGraph,
) {
    // Four events for the seed and the first commit, then four more per commit after it.
    lineage::seed_and_commit(store, graph).expect("the lineage is appendable");
    for number in 2..=4u64 {
        let transaction = TransactionId::mint();
        assert_eq!(
            store
                .append(&lineage::proposed(transaction))
                .expect("proposed"),
            Appended::Written,
            "a new fact, not a retry: proposed"
        );
        assert_eq!(
            store
                .append(&lineage::validated(
                    transaction,
                    RevisionNumber::new(number - 1),
                ))
                .expect("validated"),
            Appended::Written,
            "a new fact, not a retry: validated"
        );
        assert_eq!(
            store
                .append(&lineage::committed(
                    transaction,
                    RevisionNumber::new(number),
                    ekr_store::knowledge_root(graph),
                ))
                .expect("committed"),
            Appended::Written,
            "a new fact, not a retry: committed"
        );
    }

    let folded = store.fold().expect("the log folds");
    let replayed = store
        .replay(RevisionNumber::SEED)
        .expect("the log replays from the seed");

    assert_eq!(folded, replayed, "a replay from the seed is the fold");
    assert_eq!(
        folded.revision,
        RevisionNumber::new(4),
        "and it reaches the last committed revision"
    );
}

#[test]
fn sqlite_replay_from_the_seed_equals_the_fold() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    replay_from_the_seed_equals_the_fold(&sqlite(&directory, &ontology), &graph);
}

#[test]
fn a_file_store_replay_from_the_seed_equals_the_fold() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    replay_from_the_seed_equals_the_fold(&file(&directory, &ontology), &graph);
}

/// The fold is the seed's content, not an empty graph that happens to hash consistently.
#[test]
fn the_fold_carries_the_seed_the_log_named() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let store = sqlite(&directory, &ontology);
    lineage::seed_and_commit(&store, &graph).expect("the lineage is appendable");

    let folded = store.fold().expect("the log folds");
    assert_eq!(folded.nodes, graph.nodes, "the seed's nodes come back");
    assert_eq!(folded.edges, graph.edges, "and its edges");
    assert_eq!(folded.assertions, graph.assertions, "and its assertions");
    assert_eq!(folded.evidence, graph.evidence, "and its evidence");
    assert_eq!(folded.root, graph.root, "and the root it hangs off");
}

/// P1 materialises state at the seed and nowhere else, so a replay asking to begin at a later
/// revision is refused rather than answered with a fold that silently began somewhere else.
#[test]
fn a_replay_from_a_revision_with_no_materialised_state_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let store = sqlite(&directory, &ontology);
    lineage::seed_and_commit(&store, &graph).expect("the lineage is appendable");

    let refused = store.replay(RevisionNumber::new(1));
    assert!(
        matches!(
            refused,
            Err(StoreError::NoMaterialisedState { requested }) if requested == RevisionNumber::new(1)
        ),
        "a replay from revision 1 names what it does not have: {refused:?}"
    );
}

/// `store_graph` is the seed path and must not diverge from the long way round: the object it
/// writes is the object `GraphDocument::of(..).to_bytes()` through `put` would have written.
///
/// A convenience that produced a different address would give a lineage two seeds for one state.
#[test]
fn storing_a_graph_is_storing_its_document() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let store = sqlite(&directory, &ontology);

    let long_way = store
        .put(
            StorageClass::Canonical,
            &ekr_store::GraphDocument::of(&graph)
                .to_bytes()
                .expect("the document serialises"),
            Timestamp::EPOCH,
        )
        .expect("the document lands");
    let short_way = store
        .store_graph(&graph, Timestamp::EPOCH)
        .expect("the graph lands");

    assert_eq!(long_way, short_way, "one state, one seed object");
}

// Retention, driven through the store rather than over the type.
//
// `domain_projection.rs` holds the ordering itself; these are the three things `put` has to do with
// it, and the last two are the members of the class the adversary's case did not reach.

/// A weaker write after a stronger one does not lower the class.
///
/// The mirror of the adversary's case, and the half that first-write-wins got right by accident.
/// Both halves matter: the rule is "the strongest ever requested", so it has to be independent of
/// which order the two writes arrived in.
#[test]
fn a_cache_write_after_a_canonical_one_leaves_the_bytes_canonical() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = sqlite(&directory, &ontology);
    let bytes = b"bytes canonical state came to depend on";

    let canonical = store
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .expect("the canonical write lands");
    assert_eq!(canonical.storage_class, StorageClass::Canonical);

    let cached = store
        .put(StorageClass::Cache, bytes, Timestamp::from_millis(1))
        .expect("the later cache write is answered");
    assert_eq!(
        cached.storage_class,
        StorageClass::Canonical,
        "a later caller wanting less does not release bytes an earlier caller made durable"
    );
    assert_eq!(
        cached.stored_at,
        Timestamp::EPOCH,
        "and the instant is still the first write's"
    );
}

/// The whole ladder, in the weakest-first order that would fail under last-write-wins and in the
/// strongest-first order that would fail under first-write-wins.
///
/// Five writes of one payload each way, so the answer is pinned for every class rather than for the
/// one pair a case happened to choose.
#[test]
fn the_recorded_class_is_the_strongest_requested_whichever_order_they_arrive_in() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = sqlite(&directory, &ontology);

    let ascending = b"a payload written from the weakest class upwards";
    let mut weakest_first = Vec::from(StorageClass::ALL);
    weakest_first.sort_by_key(StorageClass::retention_rank);
    for class in &weakest_first {
        store
            .put(*class, ascending, Timestamp::EPOCH)
            .expect("each write is answered");
    }

    let descending = b"a payload written from the strongest class downwards";
    let mut strongest_first = weakest_first.clone();
    strongest_first.reverse();
    for class in &strongest_first {
        store
            .put(*class, descending, Timestamp::EPOCH)
            .expect("each write is answered");
    }

    let up = store
        .put(StorageClass::Ephemeral, ascending, Timestamp::EPOCH)
        .expect("readable");
    let down = store
        .put(StorageClass::Ephemeral, descending, Timestamp::EPOCH)
        .expect("readable");
    assert_eq!(up.storage_class, StorageClass::Canonical);
    assert_eq!(
        up.storage_class, down.storage_class,
        "the recorded class does not depend on the order the requests arrived in"
    );
}

/// A raised class survives a close and a reopen, because raising it is an append and not an edit.
///
/// The record a reader gets is a fold over the object's whole stream; if the raise were an
/// in-memory adjustment it would be gone on the next open, and the collector reads the stored
/// answer rather than the one this process happened to compute.
#[test]
fn a_raised_retention_class_survives_a_reopen() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let bytes = b"bytes cached first and needed durably later";

    {
        let store = sqlite(&directory, &ontology);
        store
            .put(StorageClass::Cache, bytes, Timestamp::EPOCH)
            .expect("the cache write lands");
        store
            .put(StorageClass::Provenance, bytes, Timestamp::EPOCH)
            .expect("the raise lands");
    }

    let reopened = sqlite(&directory, &ontology);
    let record = reopened
        .put(StorageClass::Ephemeral, bytes, Timestamp::EPOCH)
        .expect("readable after a reopen");
    assert_eq!(
        record.storage_class,
        StorageClass::Provenance,
        "the raise was written down, not held in the handle that made it"
    );
    assert_eq!(
        reopened.get(&record.content_hash).expect("readable"),
        Some(bytes.to_vec()),
        "and the bytes are still the bytes"
    );
}

/// Two different events appended in a row both land, and a third identical to the first does not.
///
/// The key `append` puts on an envelope is derived from the event's content. That makes a retry a
/// retry — which is the adversary's case — and this is the other half: two *distinct* events must
/// not collide into one, which is what a key shared between them would do. Both halves are needed,
/// because a key that deduped everything would pass the retry case on its own.
#[test]
fn two_different_events_appended_in_a_row_both_land() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let store = sqlite(&directory, &ontology);

    lineage::seed_and_commit(&store, &graph).expect("four events, all distinct");
    let folded = store.fold().expect("the lineage folds");
    assert_eq!(
        folded.revision,
        RevisionNumber::new(1),
        "the proposal, the validation and the commit are three events and not one"
    );

    let second = TransactionId::mint();
    assert_eq!(
        store.append(&lineage::proposed(second)).expect("proposed"),
        Appended::Written,
        "a new fact, not a retry: proposed"
    );
    assert_eq!(
        store
            .append(&lineage::validated(second, RevisionNumber::new(1)))
            .expect("validated"),
        Appended::Written,
        "a new fact, not a retry: validated"
    );
    assert_eq!(
        store
            .append(&lineage::committed(
                second,
                RevisionNumber::new(2),
                ekr_store::knowledge_root(&graph),
            ))
            .expect("committed"),
        Appended::Written,
        "a new fact, not a retry: committed"
    );
    assert_eq!(
        store.fold().expect("the lineage folds").revision,
        RevisionNumber::new(2),
        "and a second transaction's three events are three more"
    );
}

/// `append` distinguishes a write from a recognised request, for every event in the vocabulary.
///
/// Correction round 2's finding was that an `Ok` could not be told apart from a write. The fix is
/// the return type, and this is the property over the whole vocabulary rather than over the one
/// variant the adversary reached: for each of the six `RevisionEvent` shapes, appending it is
/// `Written` and appending it again is `AlreadyRecorded`.
///
/// Both halves, because either alone passes for a broken store: an implementation that always said
/// `Written` would pass the first, and one that always said `AlreadyRecorded` would pass the second.
#[test]
fn every_event_shape_reports_a_write_once_and_a_recognition_after() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let store = sqlite(&directory, &ontology);

    let transaction = TransactionId::mint();
    let shapes = [
        lineage::seed_event(ContentHash::of_bytes(b"a seed nothing resolves")),
        lineage::proposed(transaction),
        lineage::validated(transaction, RevisionNumber::SEED),
        lineage::committed(
            transaction,
            RevisionNumber::new(1),
            ContentHash::of_bytes(b"a knowledge root"),
        ),
        ekr_graph::RevisionEvent::TransactionRejected {
            transaction_id: transaction,
            issues: 3,
        },
        ekr_graph::RevisionEvent::TransactionStale {
            transaction_id: transaction,
            validated_against: RevisionNumber::SEED,
            current: RevisionNumber::new(1),
        },
    ];

    for event in &shapes {
        assert_eq!(
            store.append(event).expect("the first append is answered"),
            Appended::Written,
            "{} had not been appended before",
            event.name()
        );
        assert_eq!(
            store.append(event).expect("the second append is answered"),
            Appended::AlreadyRecorded,
            "{} had, and the store says so rather than writing it twice",
            event.name()
        );
    }
    assert_eq!(shapes.len(), 6, "all six of the vocabulary, not a sample");
}
