//! The properties `story:eventlog-store` ships, over both providers.
//!
//! Two providers from the start, so a property proved here is proved for the deployment. Every
//! case below is written once against `RevisionLog + ObjectStore + Initialize` and run twice where
//! it is about the provider, because a case written for SQLite and copied for the file store is a
//! case that drifts on the copy.
//!
//! # Written through the current port
//!
//! The public `RevisionLog::append` that took a bare event is gone
//! (`architecture-decision-record:0007-the-commit-path-is-the-kernels`, design § 91.6). A lineage
//! moves only through [`Initialize::initialize`] and [`RevisionLog::publish`], and each replays the
//! staged candidate through the injected [`CommitAuthority`] before anything is written. These
//! cases are about what the *store* does with a publication — persist it, page it back, recognise
//! a retry by its occurrence identity, keep the bytes the lineage names — so they inject
//! [`Mechanics`], a stand-in that admits no semantics, rather than the kernel. It is not acceptance
//! evidence: the real authority is `ekr-kernel`'s, and `crates/ekr-kernel/tests/seed.rs` and
//! `crates/ekr-kernel/tests/durable_commands.rs` hold it on both providers.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EventId, EvidenceId, GraphRootId, NodeId,
    PropertyId, RevisionId, RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue,
    Confidence, Edge, Evidence, EvidenceSource, GraphRoot, Node, Object, Predicate, RevisionEvent,
    RevisionPayload, Root, Space, Subject, TemporalRange, TransactionTime,
};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{
    evidence_root, knowledge_root, AdmittedRevision, Appended, CommitAuthority, FileStore,
    GraphDocument, Initialize, ObjectStore, Publication, PublicationObject, RetainedHistory,
    RevisionLog, SqliteStore, StorageClass, StoreError,
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
    .under(Mechanics)
}

/// Opens the file provider over a directory inside `directory`.
fn file(directory: &TempDir, ontology: &Ontology) -> FileStore {
    FileStore::file(
        &directory.path().join("revisions"),
        TENANT,
        ontology.clone(),
    )
    .expect("the file provider opens")
    .under(Mechanics)
}

/// The acceptance statement, over whichever provider the caller opened twice.
///
/// `open` is called once, written through, dropped, and called again on the same path: the second
/// store shares no memory with the first, so an address that survives is an address the provider
/// wrote down.
fn head_survives_a_reopen<S, F>(open: F, graph: &CanonicalGraph)
where
    S: RevisionLog + ObjectStore + Initialize,
    F: Fn() -> S,
{
    let before = {
        let store = open();
        seed_and_commit(&store, graph);
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
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    head_survives_a_reopen(|| sqlite(&directory, &ontology), &graph);
}

#[test]
fn a_file_store_reopened_folds_to_the_same_head_root() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    head_survives_a_reopen(|| file(&directory, &ontology), &graph);
}

/// Design § 57: identity is the content, so two writes of the same bytes are one object.
fn identical_bytes_store_once<S: ObjectStore>(store: &S) {
    let bytes = b"a payload the runtime did not choose";

    let first: ekr_store::StoredObject = store
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
    let ontology = ontology();
    identical_bytes_store_once(&sqlite(&directory, &ontology));
}

#[test]
fn a_file_store_stores_identical_bytes_once() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    identical_bytes_store_once(&file(&directory, &ontology));
}

/// A storage class is carried, not forgotten: two payloads under different classes are two
/// objects with two retention answers, and the class read back is the class written.
#[test]
fn a_stored_object_carries_the_class_it_was_written_under() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
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

/// `replay(n)` of the head revision over a log of N occurrences yields the same state as `fold()`.
///
/// Design § 91.5 made `replay` select a committed revision rather than a starting point:
/// "historical reconstruction stops at the selected committed revision". So the fold is the
/// replay *of the head*, and a replay of the seed is the seed, not the fold.
///
/// Still not a tautology, because the two do not share a read: `fold` pages the stream at
/// `MAX_READ_LIMIT` and takes this thirteen-occurrence log in one read, and `replay` and
/// `history_at` step it one occurrence at a time and take thirteen. They agree only if the cursor
/// arithmetic drops and repeats nothing at a page boundary — which is the defect a fold that
/// quietly stopped early would otherwise hide, since a short fold still returns a perfectly
/// well-formed graph.
fn replay_of_the_head_revision_equals_the_fold<S: RevisionLog + ObjectStore + Initialize>(
    store: &S,
    graph: &CanonicalGraph,
) {
    // Four occurrences for the seed and the first commit, then three more per commit after it.
    let mut version = seed_and_commit(store, graph);
    for number in 2..=4u64 {
        version = commit(store, graph, version, number);
    }
    assert_eq!(version, 13, "thirteen occurrences, one per page");

    let folded = store.fold().expect("the log folds");
    let replayed = store
        .replay(RevisionNumber::new(4))
        .expect("the log replays to its head revision");

    assert_eq!(
        folded, replayed,
        "a replay of the head revision is the fold"
    );
    assert_eq!(
        folded.revision,
        RevisionNumber::new(4),
        "and it reaches the last committed revision"
    );
    assert_eq!(
        store
            .history_at(RevisionNumber::new(4))
            .expect("the history through revision 4 reads one page at a time"),
        store.history().expect("the whole history reads"),
        "one occurrence per page and one page for all of them retain the same history"
    );
    assert_eq!(
        store
            .replay(RevisionNumber::SEED)
            .expect("the seed replays"),
        *graph,
        "a replay of the seed is the seed, not the fold: replay selects a revision"
    );
}

#[test]
fn sqlite_replay_of_the_head_revision_equals_the_fold() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    replay_of_the_head_revision_equals_the_fold(&sqlite(&directory, &ontology), &graph);
}

#[test]
fn a_file_store_replay_of_the_head_revision_equals_the_fold() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    replay_of_the_head_revision_equals_the_fold(&file(&directory, &ontology), &graph);
}

/// The fold is the seed's content, not an empty graph that happens to hash consistently.
///
/// The content reaches the fold only through the bytes the store retained under the address the
/// seed occurrence names, so those bytes are checked first and directly.
#[test]
fn the_fold_carries_the_seed_the_log_named() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let store = sqlite(&directory, &ontology);
    seed_and_commit(&store, &graph);

    assert_eq!(
        store.seed_bytes().expect("the seed is retained"),
        Some(document(&graph)),
        "the bytes the seed names are the bytes published with it"
    );
    let folded = store.fold().expect("the log folds");
    assert_eq!(folded.nodes, graph.nodes, "the seed's nodes come back");
    assert_eq!(folded.edges, graph.edges, "and its edges");
    assert_eq!(folded.assertions, graph.assertions, "and its assertions");
    assert_eq!(folded.evidence, graph.evidence, "and its evidence");
    assert_eq!(folded.root, graph.root, "and the root it hangs off");
}

/// A replay of a revision the lineage has not reached is refused rather than answered with a fold
/// that silently stopped somewhere else.
///
/// Design § 91.5 reversed the rule this case first held: P1 materialised state at the seed only,
/// and `replay(1)` was refused. Replay now reconstructs any committed revision, so revision 1 of
/// a lineage at revision 1 answers, and the refusal belongs to the revision the lineage does not
/// hold. The refusal is the authority's to raise — `ekr-kernel`'s replay raises this same
/// variant — and the store's to pass through unchanged rather than answer with what it has.
#[test]
fn a_replay_of_a_revision_the_lineage_has_not_reached_is_refused() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let store = sqlite(&directory, &ontology);
    seed_and_commit(&store, &graph);

    assert_eq!(
        store
            .replay(RevisionNumber::new(1))
            .expect("revision 1 is committed and replays"),
        store.fold().expect("the lineage folds"),
        "revision 1 is this lineage's head"
    );
    let refused = store.replay(RevisionNumber::new(2));
    assert!(
        matches!(
            refused,
            Err(StoreError::NoMaterialisedState { requested }) if requested == RevisionNumber::new(2)
        ),
        "a replay of revision 2 names what it does not have: {refused:?}"
    );
}

/// `store_graph` is the seed path and must not diverge from the long way round: the object it
/// writes is the object `GraphDocument::of(..).to_bytes()` through `put` would have written.
///
/// A convenience that produced a different address would give a lineage two seeds for one state.
#[test]
fn storing_a_graph_is_storing_its_document() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let store = sqlite(&directory, &ontology);

    let long_way = store
        .put(StorageClass::Canonical, &document(&graph), Timestamp::EPOCH)
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
    let ontology = ontology();
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
    let ontology = ontology();
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
    let ontology = ontology();
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

/// Two different occurrences published in a row both land.
///
/// A publication is recognised as a retry by its occurrence identity (design § 89: "use
/// occurrence identity for idempotency"). That makes a retry a retry — which is the adversary's
/// case — and this is the other half: two *distinct* occurrences must not collide into one, which
/// is what a key shared between them would do. Both halves are needed, because a key that deduped
/// everything would pass the retry case on its own.
#[test]
fn two_different_events_appended_in_a_row_both_land() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let store = sqlite(&directory, &ontology);

    let version = seed_and_commit(&store, &graph);
    let folded = store.fold().expect("the lineage folds");
    assert_eq!(
        folded.revision,
        RevisionNumber::new(1),
        "the proposal, the validation and the commit are three occurrences and not one"
    );

    // `commit` asserts each of its three publications is `Appended::Written`.
    commit(&store, &graph, version, 2);
    assert_eq!(
        store.fold().expect("the lineage folds").revision,
        RevisionNumber::new(2),
        "and a second transaction's three occurrences are three more"
    );
}

/// Publication distinguishes a write from a recognised request, for every event in the vocabulary.
///
/// Correction round 2's finding was that an `Ok` could not be told apart from a write. The fix is
/// the return type, and this is the property over the whole vocabulary rather than over the one
/// variant the adversary reached: for each of the six `RevisionPayload` shapes, publishing it is
/// `Written` and publishing it again is `AlreadyRecorded`.
///
/// Both halves, because either alone passes for a broken store: an implementation that always said
/// `Written` would pass the first, and one that always said `AlreadyRecorded` would pass the second.
#[test]
fn every_event_shape_reports_a_write_once_and_a_recognition_after() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = ontology();
    let graph = seed_graph(&ontology);
    let store = sqlite(&directory, &ontology);

    let transaction = TransactionId::mint();
    let shapes = [
        seed(&graph),
        occurrence(proposed(transaction), 1, None),
        occurrence(validated(transaction, RevisionNumber::SEED), 2, None),
        occurrence(
            committed(transaction, RevisionNumber::new(1), knowledge_root(&graph)),
            3,
            None,
        ),
        occurrence(
            RevisionPayload::TransactionRejected {
                transaction_id: transaction,
                issues: 3,
            },
            4,
            None,
        ),
        occurrence(
            RevisionPayload::TransactionStale {
                transaction_id: transaction,
                validated_against: RevisionNumber::SEED,
                current: RevisionNumber::new(1),
            },
            5,
            None,
        ),
    ];

    for publication in &shapes {
        let name = publication.event.name();
        let publish = || {
            if matches!(publication.event.payload, RevisionPayload::Seeded { .. }) {
                store.initialize(publication)
            } else {
                store.publish(publication)
            }
        };
        assert_eq!(
            publish().expect("the first publication is answered"),
            Appended::Written,
            "{name} had not been published before"
        );
        assert_eq!(
            publish().expect("the second publication is answered"),
            Appended::AlreadyRecorded,
            "{name} had, and the store says so rather than writing it twice"
        );
    }
    assert_eq!(shapes.len(), 6, "all six of the vocabulary, not a sample");
    assert_eq!(
        shapes
            .iter()
            .map(|publication| publication.event.variant_index())
            .collect::<BTreeSet<_>>(),
        (0..6).collect::<BTreeSet<_>>(),
        "six distinct kinds, not one kind six times"
    );
    assert_eq!(
        store
            .history()
            .expect("the lineage reads")
            .occurrences
            .len(),
        6,
        "six occurrences were written and six retries were not"
    );
}

// ---------------------------------------------------------------------------------------------
// The lineage these cases write, and the stand-in authority they write it under.
// ---------------------------------------------------------------------------------------------

/// Replays retained history for provider mechanics, and admits no semantics.
///
/// Reads every record and the seed through [`RetainedHistory::content`] at `Canonical`, so byte
/// integrity and retention are the store's to answer; decodes the seed document the lineage
/// names; and advances one revision per `RevisionCommitted`, rooting each at the address of the
/// root before it and at the record that produced it. Every other occurrence is retained and not
/// interpreted. A function of the retained history and nothing else, so two replays agree exactly
/// when the store handed them the same history.
struct Mechanics;

impl CommitAuthority for Mechanics {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }

    fn replay(
        &self,
        history: &RetainedHistory,
        ontology: Option<&Ontology>,
        selected: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        let Some((first, later)) = history.occurrences.split_first() else {
            return Ok(None);
        };
        let RevisionPayload::Seeded {
            revision_id,
            seed_hash,
        } = first.event.payload
        else {
            return Err(StoreError::NotSeeded);
        };
        let ontology =
            ontology.ok_or_else(|| StoreError::Document("no ontology was supplied".into()))?;
        history.content(first.event.record_hash, StorageClass::Canonical)?;
        let graph = decode(
            history.content(seed_hash, StorageClass::Canonical)?,
            ontology,
        )?;
        let mut state = AdmittedRevision {
            root: Root {
                revision: RevisionNumber::SEED,
                parent: None,
                ontology_root: seed_hash,
                knowledge_root: knowledge_root(&graph),
                evidence_root: evidence_root(&graph),
                agent_root: seed_hash,
                transaction: first.event.record_hash,
            },
            graph,
            revision_id,
            event_id: first.event.event_id,
            record_hash: first.event.record_hash,
            committed_at: Timestamp::EPOCH,
        };
        let mut rest = later.iter();
        while selected != Some(state.root.revision) {
            let Some(occurrence) = rest.next() else {
                return match selected {
                    Some(requested) => Err(StoreError::NoMaterialisedState { requested }),
                    None => Ok(Some(state)),
                };
            };
            let event = &occurrence.event;
            history.content(event.record_hash, StorageClass::Canonical)?;
            match event.payload {
                RevisionPayload::Seeded { .. } => return Err(StoreError::SeedIsNotFirst),
                RevisionPayload::RevisionCommitted {
                    revision_id,
                    number,
                    ..
                } => {
                    state.root = Root {
                        revision: number,
                        parent: Some(ContentHash::of(&state.root)),
                        transaction: event.record_hash,
                        ..state.root
                    };
                    state.graph.revision = number;
                    state.revision_id = revision_id;
                    state.event_id = event.event_id;
                    state.record_hash = event.record_hash;
                }
                _ => {}
            }
        }
        Ok(Some(state))
    }
}

/// The graph a seed document carries, read with serde and admitted by nothing.
fn decode(bytes: &[u8], ontology: &Ontology) -> Result<CanonicalGraph, StoreError> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Envelope {
        format: String,
        graph: Fields,
    }
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Fields {
        root: GraphRoot,
        revision: RevisionNumber,
        nodes: BTreeMap<NodeId, Node>,
        edges: BTreeMap<EdgeId, Edge>,
        assertions: BTreeMap<AssertionId, Assertion>,
        evidence: BTreeMap<EvidenceId, Evidence>,
    }
    let Envelope { format, graph } =
        serde_json::from_slice(bytes).map_err(|error| StoreError::Document(error.to_string()))?;
    if format != "ekr.graph-document/2" {
        return Err(StoreError::Document(format!(
            "unsupported document {format}"
        )));
    }
    Ok(CanonicalGraph {
        root: graph.root,
        revision: graph.revision,
        ontology: ontology.clone(),
        nodes: graph.nodes,
        edges: graph.edges,
        assertions: graph.assertions,
        evidence: graph.evidence,
    })
}

/// One occurrence of `payload` at stream position `expected_version`, with a record of its own and
/// any `extra` object published beside it.
fn occurrence(
    payload: RevisionPayload,
    expected_version: u64,
    extra: Option<(ContentHash, Vec<u8>)>,
) -> Publication {
    let event_id = EventId::mint();
    let record = format!("provider-mechanics {} record {event_id}", payload.name()).into_bytes();
    let record_hash = ContentHash::of_bytes(&record);
    let canonical = |bytes| PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
        bytes,
    };
    let mut objects = BTreeMap::from([(record_hash, canonical(record))]);
    if let Some((hash, bytes)) = extra {
        objects.insert(hash, canonical(bytes));
    }
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id,
            record_hash,
            payload,
        },
        objects,
        expected_version,
    }
}

/// The document bytes a seed of `graph` publishes.
fn document(graph: &CanonicalGraph) -> Vec<u8> {
    GraphDocument::of(graph)
        .to_bytes()
        .expect("the document serialises")
}

/// The seed occurrence naming `graph`'s document, published with it.
fn seed(graph: &CanonicalGraph) -> Publication {
    let bytes = document(graph);
    let seed_hash = ContentHash::of_bytes(&bytes);
    occurrence(
        RevisionPayload::Seeded {
            revision_id: RevisionId::mint(),
            seed_hash,
        },
        0,
        Some((seed_hash, bytes)),
    )
}

/// A proposal for `transaction`, addressed by its operations.
fn proposed(transaction: TransactionId) -> RevisionPayload {
    RevisionPayload::TransactionProposed {
        transaction_id: transaction,
        proposer: AgentId::mint(),
        operations_hash: Some(ContentHash::of_bytes(b"one operation that changes nothing")),
    }
}

/// Validation accepting `transaction` against `against`.
fn validated(transaction: TransactionId, against: RevisionNumber) -> RevisionPayload {
    RevisionPayload::TransactionValidated {
        transaction_id: transaction,
        against,
        validation_hash: ContentHash::of_bytes(b"seven validators, no issues"),
    }
}

/// `transaction` committing as revision `number`, publishing `knowledge_root`.
fn committed(
    transaction: TransactionId,
    number: RevisionNumber,
    knowledge_root: ContentHash,
) -> RevisionPayload {
    RevisionPayload::RevisionCommitted {
        transaction_id: transaction,
        revision_id: RevisionId::mint(),
        number,
        knowledge_root,
    }
}

/// Seeds `graph` and commits one transaction: four occurrences. Returns the next stream position.
fn seed_and_commit<S: RevisionLog + Initialize>(store: &S, graph: &CanonicalGraph) -> u64 {
    assert_eq!(
        store.initialize(&seed(graph)).expect("the seed publishes"),
        Appended::Written,
        "the seed is a new fact in this lineage, not a retry"
    );
    commit(store, graph, 1, 1)
}

/// Proposes, validates and commits one transaction as revision `number`, from stream position
/// `version`. Returns the next stream position.
///
/// Every one of the three is a new fact, so every one must be written rather than recognised: a
/// helper that discarded `AlreadyRecorded` would build lineages shorter than the caller asked for.
fn commit<S: RevisionLog>(store: &S, graph: &CanonicalGraph, version: u64, number: u64) -> u64 {
    let transaction = TransactionId::mint();
    // A commit that changed no graph state publishes the address the seed already had.
    let payloads = [
        proposed(transaction),
        validated(transaction, RevisionNumber::new(number - 1)),
        committed(
            transaction,
            RevisionNumber::new(number),
            knowledge_root(graph),
        ),
    ];
    let mut version = version;
    for payload in payloads {
        let publication = occurrence(payload, version, None);
        assert_eq!(
            store
                .publish(&publication)
                .expect("the occurrence publishes"),
            Appended::Written,
            "{} is a new fact in this lineage, not a retry",
            publication.event.name()
        );
        version += 1;
    }
    version
}

/// An ontology with no types. The store type-checks nothing — `AGENTS.md` invariant 7 puts type
/// validity in the kernel — so what a graph needs from it is that it exists.
fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("a document with no declarations coheres")
}

/// A seed with two nodes, an edge between them, an assertion and the evidence it rests on.
///
/// Content in all four maps, so that `knowledge_root` and `evidence_root` are functions of
/// something rather than constants: an empty seed would make every "the address is the same"
/// assertion above pass for a store that lost the lot. The runtime's own vocabulary throughout.
fn seed_graph(ontology: &Ontology) -> CanonicalGraph {
    let root_id = GraphRootId::mint();
    let type_id = TypeId::mint();
    let property = PropertyId::mint();
    let (subject, object) = (NodeId::mint(), NodeId::mint());
    let evidence_id = EvidenceId::mint();

    let mut observed = Node::new(subject, root_id, type_id, "revision-lineage");
    observed
        .properties
        .insert(property, vec![CanonicalValue::Decimal("1.0".to_owned())]);
    let reached = Node::new(object, root_id, type_id, "revision-lineage-target");
    let mut holds = Edge::new(
        EdgeId::mint(),
        root_id,
        type_id,
        CanonicalRef::new(subject),
        CanonicalRef::new(object),
    );
    holds
        .properties
        .insert(property, vec![CanonicalValue::Enum("canonical".to_owned())]);
    let evidence = Evidence {
        id: evidence_id,
        source: EvidenceSource::Document {
            document_id: "docs/roadmap.md".to_owned(),
            section: Some("P1".to_owned()),
        },
        content_hash: ContentHash::of_bytes(b"the roadmap's P1 exit criterion"),
        extracted_by: AgentId::mint(),
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::CERTAIN,
    };
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id,
        subject: Subject::Node(CanonicalRef::new(subject)),
        predicate: Predicate::Relation(type_id),
        object: Object::Node(CanonicalRef::new(object)),
        evidence: BTreeSet::from([CanonicalRef::new(evidence_id)]),
        proposed_by: AgentId::mint(),
        assessment: Assessment::Accepted {
            validators: BTreeSet::new(),
        },
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::EPOCH),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };

    CanonicalGraph {
        root: GraphRoot {
            id: root_id,
            space: Space::Canonical,
            schema_version_id: ontology.version().id,
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::SEED,
        ontology: ontology.clone(),
        nodes: BTreeMap::from([(subject, observed), (object, reached)]),
        edges: BTreeMap::from([(holds.id, holds)]),
        assertions: BTreeMap::from([(assertion.id, assertion)]),
        evidence: BTreeMap::from([(evidence_id, evidence)]),
    }
}

/// Every path under `root`, relative to it, directories included.
fn tree(root: &std::path::Path) -> BTreeSet<std::path::PathBuf> {
    fn walk(root: &std::path::Path, at: &std::path::Path, into: &mut BTreeSet<std::path::PathBuf>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            into.insert(path.strip_prefix(root).unwrap().to_path_buf());
            if path.is_dir() {
                walk(root, &path, into);
            }
        }
    }
    let mut all = BTreeSet::new();
    walk(root, root, &mut all);
    all
}

/// `story:store-open-semantics`: the existing-only constructors refuse a path holding no store and
/// create nothing there, on both providers; a store the open-or-create
/// constructor made opens through them.
#[test]
fn opening_an_existing_store_refuses_a_path_with_none_and_creates_nothing() {
    for missing in ["absent", "absent/nested/store"] {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join(missing);
        let file = FileStore::file_existing(&path, TENANT, None).map(|_| ());
        assert!(file.is_err(), "file at {missing}: {file:?}");
        let sqlite = SqliteStore::sqlite_existing(&path, TENANT, None).map(|_| ());
        assert!(sqlite.is_err(), "sqlite at {missing}: {sqlite:?}");
        assert!(
            tree(directory.path()).is_empty(),
            "{missing}: an existing-only open created {:?}",
            tree(directory.path())
        );
    }
    let directory = TempDir::new().unwrap();
    let files = directory.path().join("revisions");
    let database = directory.path().join("revisions.db");
    drop(FileStore::file(&files, TENANT, None).unwrap());
    drop(SqliteStore::sqlite(&database, TENANT, None).unwrap());
    FileStore::file_existing(&files, TENANT, None).expect("the created file store opens");
    SqliteStore::sqlite_existing(&database, TENANT, None).expect("the created database opens");
}

/// Each way a path can exist and still hold no store: an empty directory, an empty file and a
/// symlink whose target does not exist. Each is created under `directory` and returned.
fn empty_paths(directory: &TempDir) -> Vec<std::path::PathBuf> {
    let empty_directory = directory.path().join("empty-directory");
    std::fs::create_dir(&empty_directory).unwrap();
    let empty_file = directory.path().join("empty-file");
    std::fs::write(&empty_file, b"").unwrap();
    let dangling = directory.path().join("dangling");
    std::os::unix::fs::symlink(directory.path().join("no-target"), &dangling).unwrap();
    vec![empty_directory, empty_file, dangling]
}

/// `story:store-open-semantics`, correction round 1: a path that exists but holds no store is
/// [`StoreError::NoStore`] from both existing-only constructors, whatever the provider, and is
/// left exactly as it was.
#[test]
fn an_existing_path_holding_no_store_is_no_store_and_is_left_as_it_was() {
    let directory = TempDir::new().unwrap();
    let paths = empty_paths(&directory);
    let before = tree(directory.path());
    for path in &paths {
        let file = FileStore::file_existing(path, TENANT, None).map(|_| ());
        assert!(
            matches!(file, Err(StoreError::NoStore(_))),
            "file {path:?}: {file:?}"
        );
        let sqlite = SqliteStore::sqlite_existing(path, TENANT, None).map(|_| ());
        assert!(
            matches!(sqlite, Err(StoreError::NoStore(_))),
            "sqlite {path:?}: {sqlite:?}"
        );
    }
    assert_eq!(
        tree(directory.path()),
        before,
        "an existing-only open wrote something"
    );
    for (missing, file) in [("absent", true), ("absent", false)] {
        let path = directory.path().join(missing);
        let opened = if file {
            FileStore::file_existing(&path, TENANT, None).map(|_| ())
        } else {
            SqliteStore::sqlite_existing(&path, TENANT, None).map(|_| ())
        };
        assert!(
            matches!(opened, Err(StoreError::NoStore(_))),
            "{missing}: {opened:?}"
        );
    }
}

/// Correction round 1: every constructor refuses a tenant the provider would refuse before it
/// opens or creates anything, so no constructor refusal leaves a store behind.
#[test]
fn every_constructor_refuses_an_invalid_tenant_before_creating_anything() {
    let directory = TempDir::new().unwrap();
    let files = directory.path().join("revisions");
    let database = directory.path().join("revisions.db");
    let refusals = [
        ("file", FileStore::file(&files, "", None).map(|_| ())),
        (
            "sqlite",
            SqliteStore::sqlite(&database, "", None).map(|_| ()),
        ),
        (
            "file_existing",
            FileStore::file_existing(&files, "", None).map(|_| ()),
        ),
        (
            "sqlite_existing",
            SqliteStore::sqlite_existing(&database, "", None).map(|_| ()),
        ),
    ];
    for (constructor, refused) in refusals {
        assert!(
            matches!(refused, Err(StoreError::Backend(_))),
            "{constructor}: {refused:?}"
        );
    }
    assert!(
        tree(directory.path()).is_empty(),
        "a constructor refused a tenant after creating {:?}",
        tree(directory.path())
    );
}

/// Page 1 of an empty WAL-mode SQLite database: what `SqliteEventStore::open` leaves after
/// `PRAGMA journal_mode=WAL` and before its owner tables commit.
fn empty_wal_database() -> Vec<u8> {
    let mut page = vec![0_u8; 4096];
    page[..16].copy_from_slice(b"SQLite format 3\0");
    page[16..28].copy_from_slice(&[
        0x10, 0x00, 0x02, 0x02, 0x00, 0x40, 0x20, 0x20, 0x00, 0x00, 0x00, 0x01,
    ]);
    page[28..32].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    page[92..100].copy_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x00, 0x2e, 0x95, 0xcc]);
    page[100..105].copy_from_slice(&[0x0d, 0x00, 0x00, 0x00, 0x00]);
    page[105] = 0x10;
    page
}

/// Correction round 2: a store whose creation has begun and not reached its first commit point —
/// a SQLite database without the owner tables, a File directory holding only what the provider
/// writes before `manifest.json` — is [`StoreError::NoStore`] too, and is left as it was. A File
/// directory with history but no manifest is not "no store": the provider refuses it as corrupt.
#[test]
fn a_store_whose_creation_has_not_committed_is_no_store_and_is_left_as_it_was() {
    let directory = TempDir::new().unwrap();
    let database = directory.path().join("begun.db");
    std::fs::write(&database, empty_wal_database()).unwrap();
    let lock_only = directory.path().join("lock-only");
    std::fs::create_dir(&lock_only).unwrap();
    std::fs::write(lock_only.join("writer.lock"), b"").unwrap();
    let staged = directory.path().join("staged");
    std::fs::create_dir(&staged).unwrap();
    std::fs::write(staged.join("writer.lock"), b"").unwrap();
    std::fs::write(staged.join("events.jsonl"), b"").unwrap();
    std::fs::write(staged.join(".write-0"), b"{}").unwrap();
    let history = directory.path().join("history-without-manifest");
    std::fs::create_dir(&history).unwrap();
    std::fs::write(history.join("writer.lock"), b"").unwrap();
    std::fs::write(history.join("events.jsonl"), b"{}\n").unwrap();
    let before = tree(directory.path());

    let sqlite = SqliteStore::sqlite_existing(&database, TENANT, None).map(|_| ());
    assert!(matches!(sqlite, Err(StoreError::NoStore(_))), "{sqlite:?}");
    for begun in [&lock_only, &staged] {
        let file = FileStore::file_existing(begun, TENANT, None).map(|_| ());
        assert!(
            matches!(file, Err(StoreError::NoStore(_))),
            "{begun:?}: {file:?}"
        );
    }
    let corrupt = FileStore::file_existing(&history, TENANT, None).map(|_| ());
    assert!(
        matches!(corrupt, Err(StoreError::Backend(_))),
        "{corrupt:?}"
    );
    assert_eq!(
        tree(directory.path()),
        before,
        "an existing-only open wrote something"
    );
}
