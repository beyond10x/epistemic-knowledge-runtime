//! Independent review of the P1 core: AGENTS.md invariant 1, read at the store boundary.
//!
//! The invariant has two halves. "Only `ekr-kernel` constructs a `ValidatedTransaction`" is held by
//! the two compile-fail cases in `crates/ekr-kernel/tests/compile_fail/`, and this review could not
//! fault it. "Only a `ValidatedTransaction` commits" is the half these cases attack, and they attack
//! the gap rather than the sentence: `ekr-store` sits *below* `ekr-kernel` in the workspace order
//! (`docs/roadmap.md` § 3, `crates/ekr-store/Cargo.toml`), so this test binary cannot name
//! `ValidatedTransaction` at all — nothing in this process has ever constructed one — and the
//! canonical lineage still advances.
//!
//! Every event below is built by hand. `RevisionLog::append` is public and takes a bare
//! `RevisionEvent`; the fold marks a transaction validated because a `TransactionValidated` event
//! was appended, by anyone, carrying any hash.
//!
//! The second case is the sibling: the fold has `TransactionValidated.against` in hand and never
//! reads it, so a commit validated against a revision that is no longer the head replays as valid,
//! which design § 72 calls stale.

#[allow(dead_code)]
mod fixture;
#[allow(dead_code)]
mod lineage;

use ekr_core::{ContentHash, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::RevisionEvent;
use ekr_store::{Appended, GraphDocument, ObjectStore, RevisionLog, SqliteStore, StorageClass};
use tempfile::TempDir;

/// A store over a fresh SQLite database, typed by `ontology`, seeded with `graph`.
fn seeded(
    directory: &TempDir,
    ontology: &ekr_ontology::Ontology,
    graph: &ekr_graph::CanonicalGraph,
) -> SqliteStore {
    let store = SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        ontology.clone(),
    )
    .expect("the SQLite provider opens");
    let document = GraphDocument::of(graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed = store
        .put(StorageClass::Canonical, &document, Timestamp::EPOCH)
        .expect("the seed lands");
    assert_eq!(
        store
            .append(&lineage::seed_event(seed.content_hash))
            .expect("seeded"),
        Appended::Written
    );
    store
}

/// The gap, not the sentence: a commit that lands with no `ValidatedTransaction` anywhere in the
/// process.
///
/// This binary links `ekr-core`, `ekr-ontology`, `ekr-graph` and `ekr-store`. It does not and
/// cannot link `ekr-kernel`, which is the only crate that constructs a `ValidatedTransaction`. So
/// there is none in this process, and a commit lands anyway.
#[test]
fn a_commit_lands_with_no_validated_transaction_anywhere_in_the_process() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let store = seeded(&directory, &ontology, &graph);

    let transaction = TransactionId::mint();
    for event in [
        lineage::proposed(transaction),
        // A validation nothing validated: no pipeline ran, and the hash is a string this test made
        // up. The store accepts it because the event is well-formed.
        RevisionEvent::TransactionValidated {
            transaction_id: transaction,
            against: RevisionNumber::SEED,
            validation_hash: ContentHash::of_bytes(b"no validator ran; this test wrote the event"),
        },
        lineage::committed(
            transaction,
            RevisionNumber::new(1),
            ekr_store::knowledge_root(&graph),
        ),
    ] {
        assert_eq!(
            store
                .append(&event)
                .expect("the append itself is not the check"),
            Appended::Written,
            "{} is a new fact, not a retry",
            event.name()
        );
    }

    let head = store
        .head()
        .expect("the log folds")
        .expect("a seeded log has a head");
    assert_eq!(
        head.revision,
        RevisionNumber::SEED,
        "AGENTS.md invariant 1 says only a ValidatedTransaction commits. This process links \
         ekr-core, ekr-ontology, ekr-graph and ekr-store and not ekr-kernel, so no \
         ValidatedTransaction exists anywhere in it — and the canonical lineage still advanced to \
         revision {} on three hand-written events appended through the public RevisionLog::append",
        head.revision
    );
}

/// The stand-in for `ekr-kernel` this case needs after ADR 0007, and the case above must not have.
///
/// `architecture-decision-record:0007-the-commit-path-is-the-kernels` closed the first finding by
/// making the fold ask an authority before a commit moves canonical state — so the lineage the
/// case above writes no longer advances *at all*, and a case about § 72's stale commit written
/// through that same path would be green for the wrong reason. It stands behind exactly the
/// validations `lineage::validated` writes, so the only thing left separating the two commits
/// below is the revision each was validated against.
struct Attesting;

impl ekr_store::CommitAuthority for Attesting {
    fn attests(&self, validation: &ekr_store::RecordedValidation) -> bool {
        validation.validation_hash == ContentHash::of_bytes(b"seven validators, no issues")
    }
}

/// Design § 72: a transaction is committed only against the revision it was validated against;
/// otherwise it is stale. The fold receives `against` in `TransactionValidated` and never reads it.
#[test]
fn a_commit_validated_against_a_revision_that_is_no_longer_the_head_replays_as_valid() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let store = seeded(&directory, &ontology, &graph).under(Attesting);

    let (first, second) = (TransactionId::mint(), TransactionId::mint());
    let root = ekr_store::knowledge_root(&graph);
    for event in [
        lineage::proposed(first),
        lineage::proposed(second),
        // Both validated against the seed.
        lineage::validated(first, RevisionNumber::SEED),
        lineage::validated(second, RevisionNumber::SEED),
        // The second commits first, so the head moves to 1 under the first.
        lineage::committed(second, RevisionNumber::new(1), root),
        // The first was validated against 0 and the head is now 1: stale, per § 72.
        lineage::committed(first, RevisionNumber::new(2), root),
    ] {
        assert_eq!(
            store
                .append(&event)
                .expect("the append itself is not the check"),
            Appended::Written,
            "{} is a new fact, not a retry",
            event.name()
        );
    }

    let head = store
        .head()
        .expect("the log folds")
        .expect("a seeded log has a head");
    assert_eq!(
        head.revision,
        RevisionNumber::new(1),
        "design § 72: transaction {first} was validated against revision 0, revision 1 landed under \
         it, and the fold committed it at revision {} without reading the `against` it was handed \
         in TransactionValidated; a replay reports as reproducible a lineage the design calls stale",
        head.revision
    );
}
