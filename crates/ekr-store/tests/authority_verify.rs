//! `CommitAuthority::verify` and `CommitAuthority::replay_root`: what a caller that discards the
//! admitted state asks for.
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

/// `CommitAuthority::replay_root`, design § 99: the root alone, which `head` asks for when no
/// checkpoint pointer names every occurrence. An authority that does not implement it answers as
/// its `replay` does.
#[test]
fn replay_root_answers_as_replay_does_for_an_authority_that_implements_only_replay() {
    let history = RetainedHistory::default();
    assert_eq!(Fixed(Ok(None)).replay_root(&history, None, None), Ok(None));
    let refusal = StoreError::ValidationMissing {
        transaction_id: TransactionId::mint(),
    };
    assert_eq!(
        Fixed(Err(refusal.clone())).replay_root(&history, None, Some(RevisionNumber::SEED)),
        Err(refusal)
    );
    assert_eq!(
        Fixed(Err(StoreError::NotSeeded)).replay_root(&history, None, None),
        Err(StoreError::NotSeeded)
    );
}

/// Admits every history, refuses to build an admitted revision, and answers the root alone.
struct RootOnly(ekr_graph::Root);
impl CommitAuthority for RootOnly {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }
    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        Err(StoreError::Document("admitted-revision-built".into()))
    }
    fn verify(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<(), StoreError> {
        Ok(())
    }
    fn replay_root(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<ekr_graph::Root>, StoreError> {
        Ok(Some(self.0))
    }
}

/// A head that no checkpoint pointer answers asks the authority for the root alone, and never
/// for an admitted revision whose head graph it would drop.
#[test]
fn a_head_no_pointer_answers_asks_the_authority_for_the_root_alone() {
    use ekr_core::{EventId, RevisionId, Timestamp};
    use ekr_graph::{RevisionEvent, RevisionPayload};
    use ekr_store::{
        Appended, Initialize, Publication, PublicationObject, RevisionLog, StorageClass,
    };
    let root = ekr_graph::Root {
        revision: RevisionNumber::SEED,
        parent: None,
        ontology_root: ContentHash::of_bytes(b"ontology"),
        knowledge_root: ContentHash::of_bytes(b"knowledge"),
        evidence_root: ContentHash::of_bytes(b"evidence"),
        agent_root: ContentHash::of_bytes(b"agents"),
        transaction: ContentHash::of_bytes(b"transaction"),
    };
    let canonical = |bytes: &[u8]| PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
        bytes: bytes.to_vec(),
    };
    let (record, document) = (b"seed record".as_slice(), b"seed document".as_slice());
    let seed = Publication {
        event: RevisionEvent {
            application: None,
            format: RevisionEvent::FORMAT.into(),
            event_id: EventId::mint(),
            record_hash: ContentHash::of_bytes(record),
            payload: RevisionPayload::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: ContentHash::of_bytes(document),
            },
        },
        objects: [record, document]
            .into_iter()
            .map(|bytes| (ContentHash::of_bytes(bytes), canonical(bytes)))
            .collect(),
        expected_version: 0,
    };
    let directory = tempfile::tempdir().unwrap();
    let sqlite = ekr_store::SqliteStore::sqlite(&directory.path().join("state.db"), "ekr", None)
        .unwrap()
        .under(RootOnly(root));
    assert_eq!(sqlite.initialize(&seed), Ok(Appended::Written));
    assert_eq!(sqlite.head(), Ok(Some(root)), "sqlite");
    let file = ekr_store::FileStore::file(&directory.path().join("state"), "ekr", None)
        .unwrap()
        .under(RootOnly(root));
    assert_eq!(file.initialize(&seed), Ok(Appended::Written));
    assert_eq!(file.head(), Ok(Some(root)), "file");
}
