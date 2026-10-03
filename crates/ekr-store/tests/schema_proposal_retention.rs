//! Physical proposal retention is independent of canonical admission and review.
use ekr_core::contract_data::EkrIntegrateRetainedSchemaProposal;
use ekr_core::generated_identity::{Identity, SchemaProposalId};
use ekr_core::{bytes, ContentHash, SchemaVersionId, Timestamp};
use ekr_store::{ObjectStore, SchemaProposalRetention, StorageClass, StoreError};
use std::path::Path;
use std::sync::{Arc, Barrier};

trait Store: ObjectStore + SchemaProposalRetention {}
impl<T: ObjectStore + SchemaProposalRetention> Store for T {}

fn open(path: &Path, sqlite: bool) -> Box<dyn Store> {
    if sqlite {
        Box::new(ekr_store::SqliteStore::sqlite(path, "test", None).unwrap())
    } else {
        Box::new(ekr_store::FileStore::file(path, "test", None).unwrap())
    }
}

fn record() -> EkrIntegrateRetainedSchemaProposal {
    let proposal = serde_json::json!({
        "proposal_id": SchemaProposalId::mint(), "base_schema": SchemaVersionId::mint(),
        "observations": [], "sources": [], "evidence": [], "additions": [],
        "mappings": [], "corrections": [], "explanation": "Synthetic retained proposal"
    });
    let payload = serde_json::to_vec_pretty(&proposal).unwrap();
    serde_json::from_value(serde_json::json!({
        "proposal": proposal, "payload": bytes::encode(&payload),
        "proposal_digest": ContentHash::of_bytes(&payload).to_hex()
    }))
    .unwrap()
}

fn repack(record: &mut EkrIntegrateRetainedSchemaProposal) {
    let bytes = serde_json::to_vec_pretty(&record.proposal).unwrap();
    record.payload = bytes::encode(&bytes);
    record.proposal_digest.0 = ContentHash::of_bytes(&bytes).to_hex();
}

#[test]
fn exact_retries_and_reopen_preserve_the_original_on_both_providers() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let record = record();
        {
            let store = open(&path, sqlite);
            assert_eq!(
                store
                    .retain_schema_proposal(&record, Timestamp::EPOCH)
                    .unwrap(),
                (record.clone(), true)
            );
            assert_eq!(
                store
                    .retain_schema_proposal(&record, Timestamp::from_millis(17))
                    .unwrap(),
                (record.clone(), false)
            );
        }
        let store = open(&path, sqlite);
        assert_eq!(
            store.retained_schema_proposals().unwrap(),
            vec![record.clone()]
        );
        assert!(
            !store
                .retain_schema_proposal(&record, Timestamp::EPOCH)
                .unwrap()
                .1
        );
        assert_eq!(
            store
                .get(&record.proposal_digest.0.parse().unwrap())
                .unwrap()
                .unwrap(),
            bytes::decode(&record.payload).unwrap()
        );
    }
}

#[test]
fn changed_bytes_under_one_id_refuse_without_pinning_the_loser() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = open(&dir.path().join("store"), sqlite);
        let original = record();
        store
            .retain_schema_proposal(&original, Timestamp::EPOCH)
            .unwrap();
        let mut changed = original.clone();
        changed.proposal.explanation.push_str(" changed");
        repack(&mut changed);
        assert_eq!(
            store.retain_schema_proposal(&changed, Timestamp::EPOCH),
            Err(StoreError::PublicationInputConflict)
        );
        assert!(store
            .get(&changed.proposal_digest.0.parse().unwrap())
            .unwrap()
            .is_none());
        assert_eq!(store.retained_schema_proposals().unwrap(), vec![original]);
    }
}

#[test]
fn malformed_mismatched_and_noncanonical_identity_inputs_publish_nothing() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = open(&dir.path().join("store"), sqlite);
        let original = record();
        let mut cases = vec![];
        let mut bad = original.clone();
        bad.payload = "not base64!".into();
        cases.push(bad);
        let mut bad = original.clone();
        bad.payload = bytes::encode(b"{}");
        bad.proposal_digest.0 = ContentHash::of_bytes(b"{}").to_hex();
        cases.push(bad);
        let mut bad = original.clone();
        bad.proposal.explanation.push_str(" mismatched");
        cases.push(bad);
        let mut bad = original.clone();
        bad.proposal_digest.0 = ContentHash::of_bytes(b"wrong digest").to_hex();
        cases.push(bad);
        for identity in ["invalid", "00000000-0000-7000-8000-00000000000A"] {
            let mut bad = original.clone();
            bad.proposal.proposal_id.0 = identity.into();
            repack(&mut bad);
            cases.push(bad);
        }
        for bad in cases {
            assert!(
                store
                    .retain_schema_proposal(&bad, Timestamp::EPOCH)
                    .is_err(),
                "{bad:?}"
            );
            assert!(store.retained_schema_proposals().unwrap().is_empty());
            if let Ok(payload) = bytes::decode(&bad.payload) {
                assert!(store
                    .get(&ContentHash::of_bytes(&payload))
                    .unwrap()
                    .is_none());
            }
        }
    }
}

#[test]
fn retained_payload_is_pinned_and_never_downgrades_stronger_retention() {
    for sqlite in [false, true] {
        for initial in [StorageClass::Ephemeral, StorageClass::Canonical] {
            let dir = tempfile::tempdir().unwrap();
            let store = open(&dir.path().join("store"), sqlite);
            let record = record();
            let payload = bytes::decode(&record.payload).unwrap();
            store.put(initial, &payload, Timestamp::EPOCH).unwrap();
            store
                .retain_schema_proposal(&record, Timestamp::EPOCH)
                .unwrap();
            // put reports the durable strongest retention even when asked to downgrade it.
            let held = store
                .put(StorageClass::Ephemeral, &payload, Timestamp::EPOCH)
                .unwrap();
            assert_eq!(
                held.storage_class,
                initial.strongest(StorageClass::Provenance)
            );
            assert_eq!(store.retained_schema_proposals().unwrap(), vec![record]);
        }
    }
}

fn race(changed: bool) {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        drop(open(&path, sqlite));
        let first = record();
        let mut second = first.clone();
        if changed {
            second.proposal.explanation.push_str(" competing");
            repack(&mut second);
        }
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = [first.clone(), second.clone()]
            .into_iter()
            .map(|record| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    let store = open(&path, sqlite);
                    barrier.wait();
                    store.retain_schema_proposal(&record, Timestamp::EPOCH)
                })
            })
            .collect();
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(
            results
                .iter()
                .filter(|r| matches!(r, Ok((_, true))))
                .count(),
            1,
            "{results:?}"
        );
        if changed {
            assert_eq!(
                results
                    .iter()
                    .filter(|r| **r == Err(StoreError::PublicationInputConflict))
                    .count(),
                1,
                "{results:?}"
            );
        } else {
            assert_eq!(
                results
                    .iter()
                    .filter(|r| matches!(r, Ok((_, false))))
                    .count(),
                1,
                "{results:?}"
            );
        }
        let store = open(&path, sqlite);
        let held = store.retained_schema_proposals().unwrap();
        assert_eq!(held.len(), 1);
        assert!(held[0] == first || held[0] == second);
    }
}

#[test]
fn concurrent_identical_retries_elect_one_retained_record() {
    race(false);
}

#[test]
fn concurrent_changed_input_elects_one_and_refuses_the_other() {
    race(true);
}
