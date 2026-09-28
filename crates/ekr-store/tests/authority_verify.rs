//! `CommitAuthority::verify`: the admission a caller that discards the admitted state asks for.
//!
//! The store's own history reads and its preparation authorization replay a history only to have
//! the authority stand behind it; the admitted head graph `CommitAuthority::replay` returns was
//! built and dropped by each of them (`task:write-verbs-cost-most-of-an-ingest`). `verify` asks
//! for the verdict alone. An authority that does not implement it answers exactly as its
//! `replay` does, which these cases hold for an admitted, an absent and a refused history.

use std::collections::BTreeSet;

use ekr_core::{ContentHash, RevisionNumber, TransactionId};
use ekr_ontology::Ontology;
use ekr_store::{AdmittedRevision, CommitAuthority, RetainedHistory, StoreError};

/// Answers `replay` with whatever it was given, and implements nothing else.
struct Fixed(Result<Option<AdmittedRevision>, StoreError>);
impl CommitAuthority for Fixed {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }
    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        self.0.clone()
    }
}

#[test]
fn verify_answers_as_replay_does_for_an_authority_that_implements_only_replay() {
    let history = RetainedHistory::default();
    assert_eq!(Fixed(Ok(None)).verify(&history, None, None), Ok(()));
    let refusal = StoreError::ValidationMissing {
        transaction_id: TransactionId::mint(),
    };
    assert_eq!(
        Fixed(Err(refusal.clone())).verify(&history, None, Some(RevisionNumber::SEED)),
        Err(refusal)
    );
    assert_eq!(
        Fixed(Err(StoreError::NotSeeded)).verify(&history, None, None),
        Err(StoreError::NotSeeded)
    );
}
