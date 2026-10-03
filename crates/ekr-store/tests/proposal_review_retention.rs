//! Physical retention does not authenticate a signature or grant schema application authority.
use ekr_core::contract_data::{
    EkrIntegrateRetainedProposalReview as Review, EkrIntegrateRetainedSchemaProposal as Proposal,
};
use ekr_core::generated_identity::{Identity, ProposalReviewId, SchemaProposalId};
use ekr_core::{bytes, AgentId, ContentHash, EvidenceId, SchemaVersionId, Timestamp};
use ekr_store::{
    ObjectStore, ProposalReviewRetention, SchemaProposalRetention, StorageClass, StoreError,
};
use std::path::Path;
use std::sync::{Arc, Barrier};

trait Store: ObjectStore + ProposalReviewRetention + SchemaProposalRetention {}
impl<T: ObjectStore + ProposalReviewRetention + SchemaProposalRetention> Store for T {}
fn open(path: &Path, sqlite: bool) -> Box<dyn Store> {
    if sqlite {
        Box::new(ekr_store::SqliteStore::sqlite(path, "test", None).unwrap())
    } else {
        Box::new(ekr_store::FileStore::file(path, "test", None).unwrap())
    }
}
fn proposal() -> Proposal {
    let proposal = serde_json::json!({"proposal_id":SchemaProposalId::mint(),"base_schema":SchemaVersionId::mint(),"observations":[],"sources":[],"evidence":[],"additions":[],"mappings":[],"corrections":[],"explanation":"Synthetic proposal"});
    let payload = serde_json::to_vec(&proposal).unwrap();
    serde_json::from_value(serde_json::json!({"proposal":proposal,"payload":bytes::encode(&payload),"proposal_digest":ContentHash::of_bytes(&payload).to_hex()})).unwrap()
}
fn review(proposal: &Proposal) -> Review {
    let actor = AgentId::mint();
    let operator = serde_json::json!({"actor":actor,"authentication_subject":"synthetic-reviewer"});
    let evidence = EvidenceId::mint();
    let proof = format!("opaque proof {}", AgentId::mint()).into_bytes();
    let policy = b"opaque retained policy";
    let statement = b"I reviewed this synthetic proposal";
    // Protocol digests are opaque to this physical port and intentionally not object addresses.
    let protocol = ContentHash::from_bytes([17; 32]).to_hex();
    let proof_digest =
        ContentHash::of_bytes(&[proof.clone(), b"protocol fixture".to_vec()].concat()).to_hex();
    serde_json::from_value(serde_json::json!({
        "review":{"review_id":ProposalReviewId::mint(),"proposal_id":proposal.proposal.proposal_id,"proposal_digest":proposal.proposal_digest,"basis":{"observed_revision":0,"evidence_digest":protocol,"options_digest":protocol,"effects_digest":protocol},"decision":"Approved","operator":operator,"evidence_id":evidence,"recorded_at":"1970-01-01T00:00:00Z","human_proof_digest":proof_digest},
        "decision":{"decision_id":AgentId::mint(),"operator":operator,"proof_digest":proof_digest,"policy_digest":protocol,"statement_digest":protocol,"proof_object_hash":ContentHash::of_bytes(&proof).to_hex(),"policy_object_hash":ContentHash::of_bytes(policy).to_hex(),"statement_object_hash":ContentHash::of_bytes(statement).to_hex(),"recorded_at":"1970-01-01T00:00:00Z"},
        "proof":bytes::encode(&proof),"policy":bytes::encode(policy),"statement":{"payload":bytes::encode(statement),"evidence":{"id":evidence,"content_hash":ContentHash::of_bytes(statement).to_hex(),"source":{"kind":"HumanStatement","identity":"synthetic-reviewer"},"extracted_by":actor,"observed_at":"1970-01-01T00:00:00Z","confidence_bp":10000}}
    })).unwrap()
}
fn pin_proposal(store: &dyn Store, proposal: &Proposal) {
    store
        .retain_schema_proposal(proposal, Timestamp::EPOCH)
        .unwrap();
}
fn proof_hash(review: &Review) -> ContentHash {
    review.decision.proof_digest.0.parse().unwrap()
}
fn payloads(review: &Review) -> Vec<Vec<u8>> {
    [&review.proof, &review.policy, &review.statement.payload]
        .into_iter()
        .map(|s| bytes::decode(s).unwrap())
        .collect()
}

#[test]
fn exact_retry_after_later_decision_and_reopen_returns_original() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let proposal = proposal();
        let first = review(&proposal);
        let mut second = review(&proposal);
        *second.review.decision = ekr_core::contract_data::EkrIntegrateReviewDecision::V1;
        {
            let store = open(&path, sqlite);
            pin_proposal(&*store, &proposal);
            assert_eq!(
                store
                    .retain_proposal_review(&first, None, Timestamp::EPOCH)
                    .unwrap(),
                (first.clone(), true)
            );
            assert_eq!(
                store
                    .retain_proposal_review(&second, Some(proof_hash(&first)), Timestamp::EPOCH)
                    .unwrap(),
                (second.clone(), true)
            );
            assert_eq!(
                store
                    .retain_proposal_review(&first, None, Timestamp::from_millis(7))
                    .unwrap(),
                (first.clone(), false)
            );
        }
        let store = open(&path, sqlite);
        assert_eq!(
            store
                .retained_proposal_reviews(&proposal.proposal.proposal_id)
                .unwrap(),
            vec![first.clone(), second]
        );
        assert_eq!(
            store
                .retain_proposal_review(&first, None, Timestamp::EPOCH)
                .unwrap(),
            (first.clone(), false)
        );
        for payload in payloads(&first) {
            assert_eq!(
                store
                    .get(&ContentHash::of_bytes(&payload))
                    .unwrap()
                    .unwrap(),
                payload
            );
        }
    }
}

#[test]
fn missing_or_changed_proposal_and_stale_predecessor_leave_no_review_or_orphan_bytes() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = open(&dir.path().join("store"), sqlite);
        let proposal = proposal();
        let first = review(&proposal);
        assert!(store
            .retain_proposal_review(&first, None, Timestamp::EPOCH)
            .is_err());
        pin_proposal(&*store, &proposal);
        let mut changed = first.clone();
        changed.review.proposal_digest.0 = ContentHash::of_bytes(b"wrong").to_hex();
        assert!(store
            .retain_proposal_review(&changed, None, Timestamp::EPOCH)
            .is_err());
        assert_eq!(
            store.retain_proposal_review(
                &first,
                Some(ContentHash::of_bytes(b"stale")),
                Timestamp::EPOCH
            ),
            Err(StoreError::Conflict)
        );
        assert!(store
            .retained_proposal_reviews(&proposal.proposal.proposal_id)
            .unwrap()
            .is_empty());
        for payload in payloads(&first) {
            assert!(store
                .get(&ContentHash::of_bytes(&payload))
                .unwrap()
                .is_none());
        }
        assert!(
            store
                .retain_proposal_review(&first, None, Timestamp::EPOCH)
                .unwrap()
                .1
        );
        let next = review(&proposal);
        assert_eq!(
            store.retain_proposal_review(&next, None, Timestamp::EPOCH),
            Err(StoreError::Conflict)
        );
        assert!(store
            .get(&next.decision.proof_object_hash.0.parse().unwrap())
            .unwrap()
            .is_none());
        assert!(
            store
                .retain_proposal_review(&next, Some(proof_hash(&first)), Timestamp::EPOCH)
                .unwrap()
                .1
        );
    }
}

#[test]
fn changed_proof_policy_statement_or_duplicated_fields_refuse_before_append() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = open(&dir.path().join("store"), sqlite);
        let proposal = proposal();
        pin_proposal(&*store, &proposal);
        let first = review(&proposal);
        let original = serde_json::to_value(&first).unwrap();
        for (pointer, value) in [
            ("/proof", serde_json::json!(bytes::encode(b"changed proof"))),
            (
                "/policy",
                serde_json::json!(bytes::encode(b"changed policy")),
            ),
            (
                "/statement/payload",
                serde_json::json!(bytes::encode(b"changed statement")),
            ),
            ("/review/operator/actor", serde_json::json!(AgentId::mint())),
            (
                "/review/recorded_at",
                serde_json::json!("1970-01-01T00:00:01Z"),
            ),
            (
                "/review/human_proof_digest",
                serde_json::json!(ContentHash::of_bytes(b"wrong").to_hex()),
            ),
            ("/review/evidence_id", serde_json::json!(EvidenceId::mint())),
            (
                "/statement/evidence/content_hash",
                serde_json::json!(ContentHash::of_bytes(b"wrong").to_hex()),
            ),
            ("/decision/decision_id", serde_json::json!("invalid")),
            ("/review/review_id", serde_json::json!("invalid")),
            ("/decision/policy_digest", serde_json::json!("NOT A HASH")),
        ] {
            let mut changed = original.clone();
            *changed.pointer_mut(pointer).unwrap() = value;
            let changed: Review = serde_json::from_value(changed).unwrap();
            assert!(
                store
                    .retain_proposal_review(&changed, None, Timestamp::EPOCH)
                    .is_err(),
                "accepted {pointer}"
            );
            assert!(store
                .retained_proposal_reviews(&proposal.proposal.proposal_id)
                .unwrap()
                .is_empty());
            for payload in payloads(&changed) {
                assert!(
                    store
                        .get(&ContentHash::of_bytes(&payload))
                        .unwrap()
                        .is_none(),
                    "orphan at {pointer}"
                );
            }
        }
    }
}

#[test]
fn decision_and_review_identities_are_independently_unique_across_proposals() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = open(&dir.path().join("store"), sqlite);
        let p = proposal();
        let other = proposal();
        pin_proposal(&*store, &p);
        pin_proposal(&*store, &other);
        let first = review(&p);
        store
            .retain_proposal_review(&first, None, Timestamp::EPOCH)
            .unwrap();
        for decision_identity in [false, true] {
            let mut changed = review(&other);
            if decision_identity {
                changed.decision.decision_id = first.decision.decision_id.clone();
            } else {
                changed.review.review_id = first.review.review_id.clone();
            }
            assert_eq!(
                store.retain_proposal_review(&changed, None, Timestamp::EPOCH),
                Err(StoreError::PublicationInputConflict)
            );
            assert!(store
                .retained_proposal_reviews(&other.proposal.proposal_id)
                .unwrap()
                .is_empty());
            assert!(store
                .get(&changed.decision.proof_object_hash.0.parse().unwrap())
                .unwrap()
                .is_none());
        }
        let mut changed = first.clone();
        *changed.review.decision = ekr_core::contract_data::EkrIntegrateReviewDecision::V1;
        assert_eq!(
            store.retain_proposal_review(&changed, None, Timestamp::EPOCH),
            Err(StoreError::PublicationInputConflict)
        );
    }
}

#[test]
fn all_review_bytes_are_promoted_and_stronger_retention_survives() {
    for sqlite in [false, true] {
        for initial in [StorageClass::Ephemeral, StorageClass::Canonical] {
            let dir = tempfile::tempdir().unwrap();
            let store = open(&dir.path().join("store"), sqlite);
            let proposal = proposal();
            pin_proposal(&*store, &proposal);
            let review = review(&proposal);
            assert_ne!(
                review.decision.proof_digest,
                review.decision.proof_object_hash
            );
            for payload in payloads(&review) {
                store.put(initial, &payload, Timestamp::EPOCH).unwrap();
            }
            store
                .retain_proposal_review(&review, None, Timestamp::EPOCH)
                .unwrap();
            for payload in payloads(&review) {
                assert_eq!(
                    store
                        .put(StorageClass::Ephemeral, &payload, Timestamp::EPOCH)
                        .unwrap()
                        .storage_class,
                    initial.strongest(StorageClass::Provenance)
                );
            }
            assert_eq!(
                store
                    .retained_proposal_reviews(&proposal.proposal.proposal_id)
                    .unwrap(),
                vec![review]
            );
        }
    }
}

fn race(same: bool, reused_identity: bool) {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let other = proposal();
        let proposal = proposal();
        {
            let store = open(&path, sqlite);
            pin_proposal(&*store, &proposal);
            pin_proposal(&*store, &other);
        }
        let first = review(&proposal);
        let mut second = if same {
            first.clone()
        } else if reused_identity {
            review(&other)
        } else {
            review(&proposal)
        };
        if reused_identity {
            second.decision.decision_id = first.decision.decision_id.clone();
        }
        let barrier = Arc::new(Barrier::new(2));
        let handles: Vec<_> = [first.clone(), second.clone()]
            .into_iter()
            .map(|record| {
                let barrier = barrier.clone();
                let path = path.clone();
                std::thread::spawn(move || {
                    let store = open(&path, sqlite);
                    barrier.wait();
                    store.retain_proposal_review(&record, None, Timestamp::EPOCH)
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
        if same {
            assert_eq!(
                results
                    .iter()
                    .filter(|r| matches!(r, Ok((_, false))))
                    .count(),
                1,
                "{results:?}"
            );
        } else {
            let expected = if reused_identity {
                StoreError::PublicationInputConflict
            } else {
                StoreError::Conflict
            };
            assert_eq!(
                results
                    .iter()
                    .filter(|r| matches!(r,Err(e) if *e==expected))
                    .count(),
                1,
                "{results:?}"
            );
        }
        let store = open(&path, sqlite);
        let count = store
            .retained_proposal_reviews(&proposal.proposal.proposal_id)
            .unwrap()
            .len()
            + store
                .retained_proposal_reviews(&other.proposal.proposal_id)
                .unwrap()
                .len();
        assert_eq!(count, 1);
        for (record, result) in [first, second].iter().zip(&results) {
            if result.is_err() {
                assert!(store
                    .get(&record.decision.proof_object_hash.0.parse().unwrap())
                    .unwrap()
                    .is_none());
            }
        }
    }
}
#[test]
fn concurrent_identical_reviews_deduplicate() {
    race(true, false);
}
#[test]
fn concurrent_distinct_decisions_have_one_winner() {
    race(false, false);
}
#[test]
fn concurrent_decision_identity_reuse_across_proposals_has_one_winner() {
    race(false, true);
}

#[test]
fn legacy_review_events_keep_their_original_identity_rules_and_reserve_decisions() {
    use ekr_store::HumanDecisionRetention;
    use eventlog_core::{
        AppendGroup, AtomicEventStore, CommandMeta, Expected, NewEvent, StreamAppend, StreamId,
        TenantId,
    };
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let p = proposal();
        let original = review(&p);
        {
            let store = open(&path, sqlite);
            pin_proposal(&*store, &p);
            for payload in payloads(&original) {
                store
                    .put(StorageClass::Provenance, &payload, Timestamp::EPOCH)
                    .unwrap();
            }
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let provider: Box<dyn AtomicEventStore> = if sqlite {
                Box::new(
                    eventlog_sqlite::SqliteEventStore::open(&path.to_string_lossy(), "ekr")
                        .await
                        .unwrap(),
                )
            } else {
                Box::new(eventlog_file::FileEventStore::open(&path).await.unwrap())
            };
            let tenant = TenantId::new("test").unwrap();
            let data = serde_json::to_value(&original).unwrap();
            let mut appends = vec![StreamAppend {
                stream: StreamId::new(
                    tenant.clone(),
                    "ekr.integrate.proposal-reviews",
                    &p.proposal.proposal_id.0,
                )
                .unwrap(),
                expected: Expected::NoStream,
                events: vec![NewEvent::new(
                    "ekr.integrate.ProposalReviewRetained",
                    1,
                    data.clone(),
                )
                .unwrap()],
            }];
            for key in [
                format!("review-{}", original.review.review_id.0),
                format!("decision-{}", original.decision.decision_id),
            ] {
                appends.push(StreamAppend {
                    stream: StreamId::new(
                        tenant.clone(),
                        "ekr.integrate.proposal-review-identities",
                        key,
                    )
                    .unwrap(),
                    expected: Expected::NoStream,
                    events: vec![NewEvent::new(
                        "ekr.integrate.ProposalReviewIdentityBound",
                        1,
                        data.clone(),
                    )
                    .unwrap()],
                });
            }
            let key = "legacy-review-fixture";
            let meta = CommandMeta {
                idempotency_key: key.into(),
                request_hash: key.into(),
                subject: "fixture".into(),
                actor: "fixture".into(),
                request_id: key.into(),
                trace_id: key.into(),
                causation_id: None,
                causation_depth: 0,
                occurred_at: time::OffsetDateTime::UNIX_EPOCH,
                claim: None,
            };
            provider
                .append_group(&AppendGroup {
                    tenant,
                    appends,
                    meta,
                })
                .await
                .unwrap();
        });
        let store = open(&path, sqlite);
        assert_eq!(
            store
                .retained_proposal_reviews(&p.proposal.proposal_id)
                .unwrap(),
            vec![original.clone()]
        );
        assert_eq!(
            store
                .retain_proposal_review(&original, None, Timestamp::EPOCH)
                .unwrap(),
            (original.clone(), false)
        );
        let lookup = if sqlite {
            ekr_store::SqliteStore::sqlite(&path, "test", None)
                .unwrap()
                .human_decision(&original.decision.decision_id)
        } else {
            ekr_store::FileStore::file(&path, "test", None)
                .unwrap()
                .human_decision(&original.decision.decision_id)
        };
        assert_eq!(lookup.unwrap(), Some(*original.decision.clone()));
        let next = review(&p);
        assert!(
            store
                .retain_proposal_review(&next, Some(proof_hash(&original)), Timestamp::EPOCH)
                .unwrap()
                .1
        );
        assert_eq!(
            store
                .retained_proposal_reviews(&p.proposal.proposal_id)
                .unwrap(),
            vec![original, next]
        );
    }
}
