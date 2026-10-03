//! Recovery tests use a permissive physical authority, not kernel conformance.
use super::*;
use ekr_core::{AgentId, RevisionId};
struct Admit;
impl CommitAuthority for Admit {
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
fn object(bytes: Vec<u8>) -> crate::PublicationObject {
    crate::PublicationObject {
        bytes,
        storage_class: StorageClass::Canonical,
        stored_at: Timestamp::EPOCH,
    }
}
fn record() -> ekr_core::contract_data::EkrKernelHumanDecisionRecord {
    let hash = ContentHash::of_bytes(b"synthetic");
    serde_json::from_value(serde_json::json!({"decision_id":AgentId::mint(),"operator":{"actor":AgentId::mint(),"authentication_subject":"fixture"},"recorded_at":"1970-01-01T00:00:00Z","proof_digest":hash,"policy_digest":hash,"statement_digest":hash,"proof_object_hash":hash,"policy_object_hash":hash,"statement_object_hash":hash})).unwrap()
}
fn seeded<S: AtomicBlobEventStore>(store: &EventlogStore<S>) -> Publication {
    let bytes = b"synthetic seed".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let seed = Publication {
        event: RevisionEvent {
            application: None,
            format: RevisionEvent::FORMAT.into(),
            event_id: EventId::mint(),
            record_hash: hash,
            payload: RevisionPayload::Seeded {
                revision_id: RevisionId::mint(),
                seed_hash: hash,
            },
        },
        objects: BTreeMap::from([(hash, object(bytes))]),
        expected_version: 0,
    };
    assert_eq!(store.publish(&seed).unwrap(), Appended::Written);
    seed
}
fn candidate(seed: &Publication, answer: bool, new: bool) -> (PublicationCommandKey, Publication) {
    let review = record();
    let bytes = serde_json::to_vec(&serde_json::json!({"review":review})).unwrap();
    let hash = ContentHash::of_bytes(&bytes);
    let answer_id: EventId = review.decision_id.parse().unwrap();
    let payload = if answer {
        RevisionPayload::AttentionAnswered(ekr_graph::AnswerOccurrence::try_from(serde_json::from_value::<ekr_core::contract_data::EkrKernelAttentionAnsweredPayload>(serde_json::json!({"answer_id":answer_id,"revision_id":RevisionId::mint(),"transaction_id":TransactionId::mint(),"number":1,"knowledge_root":hash})).unwrap()).unwrap())
    } else {
        RevisionPayload::AuthorityUpgraded {
            transition_id: EventId::mint(),
            revision_id: RevisionId::mint(),
            number: RevisionNumber::new(1),
            knowledge_root: hash,
        }
    };
    let key = PublicationCommandKey {
        kind: if answer {
            PublicationCommandKind::AnswerAttention
        } else {
            PublicationCommandKind::UpgradeAuthority
        },
        answer_id: answer.then_some(answer_id.into()),
        transaction_id: None,
        predecessor_event_id: Some(seed.event.event_id),
        predecessor_record_hash: Some(seed.event.record_hash),
    };
    let publication = Publication {
        event: RevisionEvent {
            application: None,
            format: if new {
                RevisionEvent::SIGNED_FORMAT
            } else {
                payload.format()
            }
            .into(),
            event_id: EventId::mint(),
            record_hash: hash,
            payload,
        },
        objects: BTreeMap::from([(hash, object(bytes))]),
        expected_version: 1,
    };
    (key, publication)
}
fn legacy<S: AtomicBlobEventStore>(store: EventlogStore<S>, answer: bool) {
    let seed = seeded(&store);
    let (key, decision) = candidate(&seed, answer, false);
    let request = store.native_for(&decision, 0).unwrap();
    let prepared = PublicationPreparationV1 {
        format: if answer {
            PublicationPreparationV1::FORMAT_V5
        } else {
            PublicationPreparationV1::FORMAT_V4
        }
        .into(),
        command_key: key.clone(),
        input_hash: ContentHash::of_bytes(b"input"),
        decision: decision.clone(),
        attempt_number: 0,
        previous_attempt_hash: None,
        native_fingerprint: request.fingerprint().unwrap(),
        native_request: NativePublicationRequest::capture(&request).unwrap(),
    };
    let bytes = serde_json::to_vec(&prepared).unwrap();
    let hash = ContentHash::of_bytes(&bytes);
    // Install an immutable legacy election exactly as an older writer did. No new writer path
    // is allowed to elect or publish this format after the identity boundary.
    store
        .atomic(&BlobAppendGroup {
            group: AppendGroup {
                tenant: store.tenant.clone(),
                appends: vec![StreamAppend {
                    stream: store.preparation_stream(&key).unwrap(),
                    expected: Expected::NoStream,
                    events: vec![NewEvent::new(
                        "ekr.store.PublicationPrepared",
                        1,
                        serde_json::to_value(Selection {
                            preparation_hash: hash,
                            attempt_number: 0,
                            previous_attempt_hash: None,
                        })
                        .unwrap(),
                    )
                    .unwrap()],
                }],
                meta: envelope("legacy-fixture-election", hash.to_hex()),
            },
            blobs: vec![BlobWrite {
                digest: private_key(hash),
                bytes: bytes.clone(),
            }],
        })
        .unwrap();
    let read = store.preparation(&key).unwrap().unwrap();
    assert_eq!(serde_json::to_vec(&read).unwrap(), bytes);
    assert!(
        matches!(store.resume(&read),Err(StoreError::Document(ref reason)) if reason=="human-decision-revalidation-required")
    );
    assert_eq!(store.history().unwrap().occurrences.len(), 1);
    assert!(store.get(&decision.event.record_hash).unwrap().is_none());
    // Simulate the older writer having already committed before the new process recovered.
    store.atomic(&request).unwrap();
    assert_eq!(store.resume(&read).unwrap(), Appended::AlreadyRecorded);
    assert_eq!(store.history().unwrap().occurrences.len(), 2);
    assert_eq!(
        serde_json::to_vec(&store.preparation(&key).unwrap().unwrap()).unwrap(),
        bytes
    );
}
#[test]
fn legacy_signed_preparations_recover_only_already_published_occurrences() {
    for answer in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        legacy(
            FileStore::file(&dir.path().join("file"), "test", None)
                .unwrap()
                .under(Admit),
            answer,
        );
        legacy(
            SqliteStore::sqlite(&dir.path().join("sqlite"), "test", None)
                .unwrap()
                .under(Admit),
            answer,
        );
    }
}
fn new<S: AtomicBlobEventStore>(store: EventlogStore<S>, answer: bool) {
    let seed = seeded(&store);
    let (key, decision) = candidate(&seed, answer, true);
    let prepared = store
        .prepare(&key, ContentHash::of_bytes(b"input"), &decision, None)
        .unwrap();
    assert_eq!(prepared.format, PublicationPreparationV1::FORMAT_V6);
    let mut missing = prepared.clone();
    missing
        .native_request
        .appends
        .retain(|a| a.stream.stream_type != human_decisions::STREAM);
    missing.native_fingerprint = missing
        .native_request
        .restore()
        .unwrap()
        .fingerprint()
        .unwrap();
    assert!(
        matches!(store.authorize_preparation(&missing, false),Err(StoreError::Document(ref reason)) if reason=="preparation-human-binding-missing")
    );
    let mut wrong = prepared.clone();
    let append = wrong
        .native_request
        .appends
        .iter_mut()
        .find(|a| a.stream.stream_type == human_decisions::STREAM)
        .unwrap();
    append.stream.stream_id = AgentId::mint().to_string();
    wrong.native_fingerprint = wrong
        .native_request
        .restore()
        .unwrap()
        .fingerprint()
        .unwrap();
    assert!(
        matches!(store.authorize_preparation(&wrong, false),Err(StoreError::Document(ref reason)) if reason=="preparation-human-binding")
    );
    assert_eq!(store.resume(&prepared).unwrap(), Appended::Written);
    assert_eq!(store.resume(&prepared).unwrap(), Appended::AlreadyRecorded);
    assert_eq!(store.history().unwrap().occurrences.len(), 2);
}
#[test]
fn new_signed_preparations_require_exact_shared_index_and_retry_idempotently() {
    for answer in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        new(
            FileStore::file(&dir.path().join("file"), "test", None)
                .unwrap()
                .under(Admit),
            answer,
        );
        new(
            SqliteStore::sqlite(&dir.path().join("sqlite"), "test", None)
                .unwrap()
                .under(Admit),
            answer,
        );
    }
}
