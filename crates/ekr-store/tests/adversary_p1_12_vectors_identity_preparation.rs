//! Adversary, unit p1-12-vectors. § 89: "Reusing an identity for different content refuses."
//!
//! `current_occurrence_identity.rs` holds that rule on `RevisionLog::publish`. No kernel command
//! calls `publish`: every durable command goes through `prepare` then `resume`
//! (`crates/ekr-kernel/src/commands.rs`, `crates/ekr-kernel/src/commit.rs`). This case drives the
//! same rule through that path, on both providers: an already retained `event_id` carried by a
//! newly elected preparation with different content must be refused, write nothing, and leave
//! the retained history readable and unchanged.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AgentId, ContentHash, EventId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, CommitAuthority, FileStore, Publication, PublicationCommandKey,
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

fn object(bytes: &[u8]) -> PublicationObject {
    PublicationObject {
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
        bytes: bytes.to_vec(),
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
        objects: BTreeMap::from([(ContentHash::of_bytes(record), object(record))]),
        expected_version: at,
    }
}

fn propose_key(transaction: u64) -> PublicationCommandKey {
    PublicationCommandKey {
        kind: PublicationCommandKind::Propose,
        transaction_id: Some(id(transaction)),
        predecessor_event_id: None,
        predecessor_record_hash: None,
    }
}

fn proposed(transaction: u64, record: &[u8], at: u64) -> Publication {
    decision(
        0x41,
        RevisionPayload::TransactionProposed {
            transaction_id: id(transaction),
            proposer: id::<AgentId>(0x03),
            operations_hash: None,
        },
        record,
        at,
    )
}

#[test]
fn a_retained_event_id_with_changed_content_is_refused_on_the_preparation_path_on_both_providers() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let store = open(directory.path(), file);

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
            object(b"adversary seed envelope"),
        );
        let bootstrap = PublicationCommandKey {
            kind: PublicationCommandKind::Bootstrap,
            transaction_id: None,
            predecessor_event_id: None,
            predecessor_record_hash: None,
        };
        let elected = store
            .prepare(
                &bootstrap,
                ContentHash::of_bytes(b"seed input"),
                &seed,
                None,
            )
            .unwrap();
        let _ = store.resume(&elected).unwrap();

        let original = proposed(0x51, b"adversary proposal record", 1);
        let elected = store
            .prepare(
                &propose_key(0x51),
                ContentHash::of_bytes(b"first input"),
                &original,
                None,
            )
            .unwrap();
        let _ = store.resume(&elected).unwrap();
        let retained = store.history().unwrap();
        assert_eq!(retained.occurrences.len(), 2, "file={file}");

        // The same occurrence identity, a different transaction and a different record, elected
        // under the slot that transaction owns and resumed at the current head.
        let changed = proposed(0x52, b"a different proposal record", 2);
        let outcome = store
            .prepare(
                &propose_key(0x52),
                ContentHash::of_bytes(b"second input"),
                &changed,
                None,
            )
            .and_then(|elected| store.resume(&elected));
        if !matches!(&outcome, Err(StoreError::Document(code)) if code == "occurrence-identity-conflict")
        {
            wrong.push(format!(
                "file={file}: prepare+resume of a reused event_id returned {outcome:?}"
            ));
        }
        match store.history() {
            Ok(history) if history == retained => {}
            Ok(history) => wrong.push(format!(
                "file={file}: history changed to {} occurrences",
                history.occurrences.len()
            )),
            Err(error) => wrong.push(format!(
                "file={file}: retained history no longer reads: {error:?}"
            )),
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}

/// The original occurrence is retained through a successor attempt — the shape the kernel's
/// `drive` loop produces after one `Conflict` — so its native idempotency key ends `.1`. A later
/// preparation reusing that `event_id` with different content runs at attempt 0, whose key is
/// unused, so nothing but the store's own identity check stands between it and the log.
#[test]
fn a_reused_event_id_after_a_retried_original_is_refused_and_history_stays_readable_on_both_providers(
) {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let store = open(directory.path(), file);

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
            object(b"adversary seed envelope"),
        );
        let bootstrap = PublicationCommandKey {
            kind: PublicationCommandKind::Bootstrap,
            transaction_id: None,
            predecessor_event_id: None,
            predecessor_record_hash: None,
        };
        let elected = store
            .prepare(
                &bootstrap,
                ContentHash::of_bytes(b"seed input"),
                &seed,
                None,
            )
            .unwrap();
        let _ = store.resume(&elected).unwrap();

        // Elect the original at head 1, then let another occurrence move the head first.
        let original = proposed(0x51, b"adversary proposal record", 1);
        let key = propose_key(0x51);
        let input = ContentHash::of_bytes(b"first input");
        let first = store.prepare(&key, input, &original, None).unwrap();
        let _ = store
            .prepare(
                &propose_key(0x53),
                ContentHash::of_bytes(b"other input"),
                &decision(
                    0x42,
                    RevisionPayload::TransactionProposed {
                        transaction_id: id(0x53),
                        proposer: id::<AgentId>(0x03),
                        operations_hash: None,
                    },
                    b"another proposal record",
                    1,
                ),
                None,
            )
            .and_then(|other| store.resume(&other))
            .unwrap();
        assert!(
            matches!(store.resume(&first), Err(StoreError::Conflict)),
            "file={file}"
        );
        let rebased = Publication {
            expected_version: 2,
            ..original.clone()
        };
        let second = store.prepare(&key, input, &rebased, Some(&first)).unwrap();
        assert_eq!(second.attempt_number, 1, "file={file}");
        let _ = store.resume(&second).unwrap();
        let retained = store.history().unwrap();
        assert_eq!(retained.occurrences.len(), 3, "file={file}");

        let changed = proposed(0x52, b"a different proposal record", 3);
        let outcome = store
            .prepare(
                &propose_key(0x52),
                ContentHash::of_bytes(b"second input"),
                &changed,
                None,
            )
            .and_then(|elected| store.resume(&elected));
        if !matches!(&outcome, Err(StoreError::Document(code)) if code == "occurrence-identity-conflict")
        {
            wrong.push(format!(
                "file={file}: prepare+resume of a reused event_id returned {outcome:?}"
            ));
        }
        match store.history() {
            Ok(history) if history == retained => {}
            Ok(history) => wrong.push(format!(
                "file={file}: history changed to {} occurrences",
                history.occurrences.len()
            )),
            Err(error) => wrong.push(format!(
                "file={file}: retained history no longer reads: {error:?}"
            )),
        }
    }
    assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
}
