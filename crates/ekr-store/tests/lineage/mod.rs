//! The events that move a revision lineage, and the shortest lineage worth folding.
//!
//! Declared by `providers.rs` and `fold_rules.rs`. Written out one event at a time rather than
//! hidden behind a helper on the store, because the fold's rules are about which of these may
//! follow which, and a case that cannot write a wrong order cannot test them.

use ekr_core::{AgentId, ContentHash, RevisionId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{CanonicalGraph, RevisionEvent};
use ekr_store::{
    Appended, CommitAuthority, GraphDocument, ObjectStore, RecordedValidation, RevisionLog,
    StorageClass, StoreError,
};

/// The bytes [`validated`] gives a validation result, and the only address [`Attesting`] stands
/// behind.
const VALIDATION_RESULT: &[u8] = b"seven validators, no issues";

/// The seed event, naming the state at `seed_hash`.
#[must_use]
pub fn seed_event(seed_hash: ContentHash) -> RevisionEvent {
    RevisionEvent::Seeded {
        revision_id: RevisionId::mint(),
        seed_hash,
    }
}

/// A proposal for `transaction`, addressed by its operations.
#[must_use]
pub fn proposed(transaction: TransactionId) -> RevisionEvent {
    RevisionEvent::TransactionProposed {
        transaction_id: transaction,
        proposer: AgentId::mint(),
        operations_hash: ContentHash::of_bytes(b"one operation that changes nothing"),
    }
}

/// Validation accepting `transaction` against `against`.
#[must_use]
pub fn validated(transaction: TransactionId, against: RevisionNumber) -> RevisionEvent {
    RevisionEvent::TransactionValidated {
        transaction_id: transaction,
        against,
        validation_hash: ContentHash::of_bytes(VALIDATION_RESULT),
    }
}

/// The stand-in for `ekr-kernel` in this crate's own suite:
/// `architecture-decision-record:0007-the-commit-path-is-the-kernels`.
///
/// `ekr-store` sits below `ekr-kernel` and cannot depend on it, so a case here that needs a
/// lineage to advance needs something to stand behind its validations. This does, and **only for
/// the validations [`validated`] wrote**: a hand-written event carrying any other address is a
/// claim this authority does not answer for, which is what
/// `tests/fold_rules.rs::a_commit_whose_validation_this_authority_does_not_stand_behind_does_not_advance_the_lineage`
/// reads.
///
/// It is not a second writer to canonical state and no `src/` implements this trait but the
/// kernel's, which `crates/ekr/tests/story_contract.rs` holds.
pub struct Attesting;

impl CommitAuthority for Attesting {
    fn attests(&self, validation: &RecordedValidation) -> bool {
        validation.validation_hash == ContentHash::of_bytes(VALIDATION_RESULT)
    }
}

/// `transaction` committing as revision `number`, publishing `knowledge_root`.
#[must_use]
pub fn committed(
    transaction: TransactionId,
    number: RevisionNumber,
    knowledge_root: ContentHash,
) -> RevisionEvent {
    RevisionEvent::RevisionCommitted {
        transaction_id: transaction,
        revision_id: RevisionId::mint(),
        number,
        knowledge_root,
    }
}

/// Writes the seed graph as a stored object and appends the four events one commit costs.
///
/// # Errors
///
/// Whatever the store returned.
pub fn seed_and_commit<S: RevisionLog + ObjectStore>(
    store: &S,
    graph: &CanonicalGraph,
) -> Result<(), StoreError> {
    let document = GraphDocument::of(graph).to_bytes()?;
    let seed = store.put(StorageClass::Canonical, &document, Timestamp::EPOCH)?;

    let transaction = TransactionId::mint();
    // Every one of the four is a new fact, so every one must be written rather than recognised.
    // `append` answers `AlreadyRecorded` for an event byte-identical to one already in the log, and
    // a helper that discarded that answer would build lineages shorter than the caller asked for.
    for event in [
        seed_event(seed.content_hash),
        proposed(transaction),
        validated(transaction, RevisionNumber::SEED),
        // A commit that changed no graph state publishes the address the seed already had. P1's
        // events carry no operations, so this is the only knowledge root a commit can honestly
        // claim.
        committed(
            transaction,
            RevisionNumber::new(1),
            ekr_store::knowledge_root(graph),
        ),
    ] {
        assert_eq!(
            store.append(&event)?,
            Appended::Written,
            "{} is a new fact in this lineage, not a retry",
            event.name()
        );
    }
    Ok(())
}
