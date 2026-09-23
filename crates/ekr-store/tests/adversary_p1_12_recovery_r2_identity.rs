//! Adversary pass 2 on unit p1-12-recovery: the occurrence-identity check in
//! `authorize_preparation` (`preparation.rs`), which now runs on elect, resume and every read.
//!
//! * A genuine retry whose occurrence landed through attempt 2, response lost, retried from a
//!   fresh handle, must resume as `AlreadyRecorded` and every read of the slot must still pass.
//! * The `occurrence-already-retained` branch (identical content already in the prefix) is the
//!   sole guard when the identical decision is elected in another slot at an unused attempt
//!   number: no other case reaches it.
//!
//! Store level, `AdmitAll` authority, **fresh handle** reopen; no fault injection.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, FileStore, Publication, PublicationCommandKey,
    PublicationCommandKind, PublicationObject, RetainedHistory, RevisionLog, SqliteStore,
    StorageClass, StoreError,
};

fn id<T: std::str::FromStr>(n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-0000-4000-8000-{n:012x}").parse().unwrap()
}

struct AdmitAll;
impl CommitAuthority for AdmitAll {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }
    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        Ok(None)
    }
}

fn open(path: &std::path::Path, file: bool) -> Box<dyn RevisionLog> {
    if file {
        Box::new(FileStore::file(path, "ekr", None).unwrap().under(AdmitAll))
    } else {
        Box::new(
            SqliteStore::sqlite(&path.join("state.db"), "ekr", None)
                .unwrap()
                .under(AdmitAll),
        )
    }
}

fn decision(event_id: u64, payload: RevisionPayload, record: &[u8], at: u64) -> Publication {
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::FORMAT.into(),
            event_id: id::<EventId>(event_id),
            record_hash: ContentHash::of_bytes(record),
            payload,
        },
        objects: BTreeMap::from([(
            ContentHash::of_bytes(record),
            PublicationObject {
                storage_class: StorageClass::Canonical,
                stored_at: Timestamp::EPOCH,
                bytes: record.to_vec(),
            },
        )]),
        expected_version: at,
    }
}

fn proposed(event: u64, transaction: u64, record: &[u8], at: u64) -> Publication {
    decision(
        event,
        RevisionPayload::TransactionProposed {
            transaction_id: id(transaction),
            proposer: id::<AgentId>(0x03),
            operations_hash: None,
        },
        record,
        at,
    )
}

fn propose_key(transaction: u64) -> PublicationCommandKey {
    PublicationCommandKey {
        kind: PublicationCommandKind::Propose,
        transaction_id: Some(id(transaction)),
        predecessor_event_id: None,
        predecessor_record_hash: None,
    }
}

fn seed(store: &dyn RevisionLog) {
    let mut seed = decision(
        0x40,
        RevisionPayload::Seeded {
            revision_id: id::<RevisionId>(0x31),
            seed_hash: ContentHash::of_bytes(b"adversary seed envelope"),
        },
        b"adversary seed record",
        0,
    );
    seed.objects.insert(
        ContentHash::of_bytes(b"adversary seed envelope"),
        PublicationObject {
            storage_class: StorageClass::Canonical,
            stored_at: Timestamp::EPOCH,
            bytes: b"adversary seed envelope".to_vec(),
        },
    );
    let key = PublicationCommandKey {
        kind: PublicationCommandKind::Bootstrap,
        transaction_id: None,
        predecessor_event_id: None,
        predecessor_record_hash: None,
    };
    let elected = store
        .prepare(&key, ContentHash::of_bytes(b"seed input"), &seed, None)
        .unwrap();
    let _ = store.resume(&elected).unwrap();
}

/// Publishes an unrelated proposal at `at`, moving the head.
fn unrelated(store: &dyn RevisionLog, event: u64, transaction: u64, at: u64) {
    let record = format!("unrelated record {event}");
    let elected = store
        .prepare(
            &propose_key(transaction),
            ContentHash::of_bytes(record.as_bytes()),
            &proposed(event, transaction, record.as_bytes(), at),
            None,
        )
        .unwrap();
    let _ = store.resume(&elected).unwrap();
}

/// § 94 item 2 at attempt 2: an occurrence that landed through its third attempt, response lost,
/// is reported as already recorded from a fresh handle, and the slot still reads.
#[test]
fn adv2_a_landed_third_attempt_retried_from_a_fresh_handle_is_already_recorded() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let key = propose_key(0x51);
        let input = ContentHash::of_bytes(b"first input");
        let original = proposed(0x41, 0x51, b"adversary proposal record", 1);
        let third = {
            let store = open(path, file);
            seed(store.as_ref());
            let first = store.prepare(&key, input, &original, None).unwrap();
            unrelated(store.as_ref(), 0x60, 0x70, 1);
            assert!(matches!(store.resume(&first), Err(StoreError::Conflict)));
            let second = store
                .prepare(
                    &key,
                    input,
                    &Publication {
                        expected_version: 2,
                        ..original.clone()
                    },
                    Some(&first),
                )
                .unwrap();
            unrelated(store.as_ref(), 0x61, 0x71, 2);
            assert!(matches!(store.resume(&second), Err(StoreError::Conflict)));
            let third = store
                .prepare(
                    &key,
                    input,
                    &Publication {
                        expected_version: 3,
                        ..original.clone()
                    },
                    Some(&second),
                )
                .unwrap();
            assert_eq!(third.attempt_number, 2, "file={file}");
            // Applied; the response is treated as lost.
            let _ = store.resume(&third).unwrap();
            third
        };
        let store = open(path, file);
        let retained = store.history().unwrap();
        assert_eq!(retained.occurrences.len(), 4, "file={file}");
        match store.preparation(&key) {
            Ok(Some(held)) if held == third => {}
            other => wrong.push(format!("file={file}: slot read after landing: {other:?}")),
        }
        match store.resume(&third) {
            Ok(Appended::AlreadyRecorded) => {}
            other => wrong.push(format!("file={file}: retry of landed attempt 2: {other:?}")),
        }
        match store.prepare(&key, input, &original, None) {
            Ok(held) if held == third => {}
            other => wrong.push(format!("file={file}: re-election after landing: {other:?}")),
        }
        if store.history().ok().as_ref() != Some(&retained) {
            wrong.push(format!("file={file}: history changed on retry"));
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// The identical retained decision, elected under a second slot at an attempt number whose native
/// key was never used: the provider cannot deduplicate it, so only `occurrence-already-retained`
/// stands between it and a second recording of one occurrence.
#[test]
fn adv2_an_identical_retained_decision_under_another_slot_is_refused_as_already_retained() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let store = open(directory.path(), file);
        seed(store.as_ref());
        // Two proposals of one transaction (AdmitAll permits it), each owning a Validate slot.
        let first_proposal = proposed(0x42, 0x51, b"first proposal record", 1);
        let second_proposal = proposed(0x43, 0x51, b"second proposal record", 2);
        let elected = store
            .prepare(
                &propose_key(0x51),
                ContentHash::of_bytes(b"p1"),
                &first_proposal,
                None,
            )
            .unwrap();
        let _ = store.resume(&elected).unwrap();
        // The Propose slot of 0x51 is taken; the second proposal goes through the legacy port.
        let _ = store.publish(&second_proposal).unwrap();
        let validate_key = |proposal: &Publication| PublicationCommandKey {
            kind: PublicationCommandKind::Validate,
            transaction_id: Some(id(0x51)),
            predecessor_event_id: Some(proposal.event.event_id),
            predecessor_record_hash: Some(proposal.event.record_hash),
        };
        let validated = |at| {
            decision(
                0x44,
                RevisionPayload::TransactionValidated {
                    transaction_id: id(0x51),
                    against: RevisionNumber::SEED,
                    validation_hash: ContentHash::of_bytes(b"validation"),
                },
                b"validation record",
                at,
            )
        };
        // Slot one: attempt 0 at head 3 is overtaken; attempt 1 lands (native key `.1`).
        let k1 = validate_key(&first_proposal);
        let input = ContentHash::of_bytes(b"v1");
        let a0 = store.prepare(&k1, input, &validated(3), None).unwrap();
        unrelated(store.as_ref(), 0x62, 0x72, 3);
        assert!(matches!(store.resume(&a0), Err(StoreError::Conflict)));
        let a1 = store.prepare(&k1, input, &validated(4), Some(&a0)).unwrap();
        let _ = store.resume(&a1).unwrap();
        let retained = store.history().unwrap();
        assert_eq!(retained.occurrences.len(), 5, "file={file}");

        // Slot two: the identical decision at the head, attempt 0 (native key `.0`, never used).
        let k2 = validate_key(&second_proposal);
        let outcome = store
            .prepare(&k2, ContentHash::of_bytes(b"v2"), &validated(5), None)
            .and_then(|elected| store.resume(&elected));
        if !matches!(&outcome, Err(StoreError::Document(code)) if code == "occurrence-already-retained")
        {
            wrong.push(format!(
                "file={file}: identical retained decision in a second slot returned {outcome:?}"
            ));
        }
        match store.history() {
            Ok(history) if history == retained => {}
            Ok(history) => wrong.push(format!(
                "file={file}: history changed to {} occurrences",
                history.occurrences.len()
            )),
            Err(error) => wrong.push(format!("file={file}: history no longer reads: {error:?}")),
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
