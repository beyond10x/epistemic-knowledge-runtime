//! Whether "two byte-identical events cannot both be in the log" is safe for this vocabulary.
//!
//! Correction round 1 derived the append idempotency key from the event's own content, which fixed
//! the duplicate-append defect and bought a consequence the implementor states in `eventlog.rs`:
//!
//! > two byte-identical events cannot both be in the log, because the store cannot tell a second
//! > one from a retry of the first. That is right for this vocabulary rather than a limitation of
//! > it — every `RevisionEvent` variant carries a minted `RevisionId` or names a `TransactionId`,
//! > and each of the six says something that happens once for that id.
//!
//! Naming a `TransactionId` is not the same as happening once for it. Two of the six variants
//! carry **no minted id and no discriminating payload**: `TransactionRejected { transaction_id,
//! issues }` and `TransactionStale { transaction_id, validated_against, current }`. And the fold
//! is built for a transaction id that recurs — `Fold::apply` *removes* a rejected or stale
//! transaction from `proposed` rather than marking it terminal, which is what lets design § 72's
//! "revalidated, not committed" happen to the same transaction.

mod fixture;

use ekr_core::{AgentId, ContentHash, RevisionId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::RevisionEvent;
use ekr_store::{Appended, ObjectStore, RevisionLog, SqliteStore, StorageClass};
use tempfile::TempDir;

/// A store over a fresh SQLite database, typed by `ontology`.
fn store(directory: &TempDir, ontology: &ekr_ontology::Ontology) -> SqliteStore {
    SqliteStore::sqlite(
        &directory.path().join("revisions.db"),
        "ekr",
        ontology.clone(),
    )
    .expect("the SQLite provider opens")
}

/// A rejection the store cannot tell from an earlier one is **reported**, not silently dropped.
///
/// The lineage the caller appends is ordinary and every event of it is one the vocabulary admits:
/// a transaction is proposed, validated, **rejected**, proposed again with different operations,
/// validated again, and **rejected again with the same issue count**. Only then does a commit
/// arrive.
///
/// `fold_rules.rs`'s `a_rejected_transaction_cannot_then_commit` is the guard: a refused
/// transaction is not a validated one. It holds for one rejection. The second rejection is
/// byte-identical to the first — `TransactionRejected` carries only the transaction and a count —
/// so the store reads it as a retry, does not write it, and the fold never sees the refusal that
/// was appended to it.
///
/// **What changed in correction round 2 is the `Ok`.** `append` returned a unit and the caller
/// could not tell a write from a recognised retry, which is `append` losing an event it reported
/// success for. It now returns [`Appended`], so the loss is visible at the call that caused it.
///
/// **What did not change is the vocabulary**, and it is what actually makes the commit land. Two
/// rejections of one transaction *should* be two events; that they are not is a shape defect in
/// `ekr_graph::RevisionEvent` and `systems/ekr/domains/kernel.yaml`, filed as
/// `task:two-revision-events-have-no-discriminator` and blocking
/// `story:commit-and-revision-lineage`. Neither crate is this story's, so this case pins both
/// halves: the report, which is fixed, and the commit, which is not.
#[test]
fn a_second_rejection_of_a_re_proposed_transaction_is_not_swallowed_as_a_retry() {
    let directory = TempDir::new().expect("a temporary directory");
    let ontology = fixture::ontology();
    let graph = fixture::seed_graph(&ontology);
    let store = store(&directory, &ontology);

    let document = ekr_store::GraphDocument::of(&graph)
        .to_bytes()
        .expect("the seed serialises");
    let seed = store
        .put(StorageClass::Canonical, &document, Timestamp::EPOCH)
        .expect("the seed lands");
    assert_eq!(
        store
            .append(&RevisionEvent::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: seed.content_hash,
            })
            .expect("seeded"),
        Appended::Written,
    );

    let transaction = TransactionId::mint();
    let proposer = AgentId::mint();
    let rejected = RevisionEvent::TransactionRejected {
        transaction_id: transaction,
        issues: 3,
    };
    let propose = |operations: &[u8]| RevisionEvent::TransactionProposed {
        transaction_id: transaction,
        proposer,
        operations_hash: ContentHash::of_bytes(operations),
    };
    let validate = |result: &[u8]| RevisionEvent::TransactionValidated {
        transaction_id: transaction,
        against: RevisionNumber::SEED,
        validation_hash: ContentHash::of_bytes(result),
    };

    // The first attempt: proposed, validated, refused. Each is a new fact and each is written.
    for (event, what) in [
        (propose(b"the first operations"), "proposed"),
        (validate(b"a first validation result"), "validated"),
        (rejected.clone(), "rejected"),
    ] {
        assert_eq!(
            store.append(&event).expect(what),
            Appended::Written,
            "the first attempt's {what} is a new fact"
        );
    }

    // The second attempt, with different operations and a different validation result. Both differ
    // from the first attempt's, so both are written.
    for (event, what) in [
        (propose(b"the second operations"), "proposed again"),
        (validate(b"a second validation result"), "validated again"),
    ] {
        assert_eq!(
            store.append(&event).expect(what),
            Appended::Written,
            "the second attempt's {what} differs from the first and is a new fact"
        );
    }

    // And refused again, for the same number of issues. This is the one the store cannot write.
    assert_eq!(
        store.append(&rejected).expect("rejected again"),
        Appended::AlreadyRecorded,
        "the second rejection is byte-identical to the first, and the store says so rather than \
         returning an Ok that cannot be told apart from a write"
    );

    // And a commit of it anyway.
    assert_eq!(
        store
            .append(&RevisionEvent::RevisionCommitted {
                transaction_id: transaction,
                revision_id: RevisionId::mint(),
                number: RevisionNumber::new(1),
                knowledge_root: ekr_store::knowledge_root(&graph),
            })
            .expect("the append itself is not the check"),
        Appended::Written,
    );

    // The log holds one rejection, so the fold sees the transaction proposed, validated and
    // committed, and the commit lands. That is **not** fixed here and is pinned deliberately:
    // `task:two-revision-events-have-no-discriminator` is the shape change that closes it, in
    // `ekr_graph::RevisionEvent` and `systems/ekr/domains/kernel.yaml` together, and neither is
    // this story's. The day it lands, the second rejection becomes a distinct event, the fold sees
    // it, and this assertion changes to the `ValidationMissing` the adversary originally wanted.
    //
    // What *is* fixed is the line above: the caller was told. A caller that did not intend a retry
    // now has an answer it can act on, which is the whole of what the store can do about a
    // vocabulary in which two facts share an encoding.
    let folded = store.fold();
    assert!(
        matches!(folded, Ok(graph) if graph.revision == RevisionNumber::new(1)),
        "with only one rejection in the log the commit lands; the store reported the loss rather \
         than hiding it, and the discriminator that would prevent it is another crate's: {:?}",
        store.fold()
    );
}
