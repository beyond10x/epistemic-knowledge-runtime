//! Physical cross-operation identity membrane; the test authority grants no real kernel admission.
use ekr_core::contract_data::{
    EkrIntegrateRetainedProposalReview as Review, EkrIntegrateRetainedSchemaProposal as Proposal,
    EkrKernelHumanDecisionRecord as Decision,
};
use ekr_core::generated_identity::{Identity, ProposalReviewId, SchemaProposalId};
use ekr_core::{
    bytes, AgentId, ContentHash, EventId, EvidenceId, RevisionId, RevisionNumber, SchemaVersionId,
    Timestamp,
};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_ontology::Ontology;
use ekr_store::{
    AdmittedRevision, Appended, CommitAuthority, HumanDecisionRetention, ObjectStore,
    ProposalReviewRetention, Publication, PublicationObject, RetainedHistory, RevisionLog,
    SchemaProposalRetention, StorageClass, StoreError,
};
use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::{Arc, Barrier},
};
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
trait Store:
    RevisionLog
    + ObjectStore
    + HumanDecisionRetention
    + ProposalReviewRetention
    + SchemaProposalRetention
{
}
impl<
        T: RevisionLog
            + ObjectStore
            + HumanDecisionRetention
            + ProposalReviewRetention
            + SchemaProposalRetention,
    > Store for T
{
}
fn open(path: &Path, sqlite: bool) -> Box<dyn Store> {
    if sqlite {
        Box::new(
            ekr_store::SqliteStore::sqlite(path, "test", None)
                .unwrap()
                .under(Admit),
        )
    } else {
        Box::new(
            ekr_store::FileStore::file(path, "test", None)
                .unwrap()
                .under(Admit),
        )
    }
}
fn decision(id: AgentId) -> Decision {
    let hash = ContentHash::of_bytes(b"synthetic").to_hex();
    serde_json::from_value(serde_json::json!({"decision_id":id,"operator":{"actor":AgentId::mint(),"authentication_subject":"synthetic"},"recorded_at":"1970-01-01T00:00:00Z","proof_digest":ContentHash::of_bytes(id.to_string().as_bytes()).to_hex(),"policy_digest":hash,"statement_digest":hash,"proof_object_hash":hash,"policy_object_hash":hash,"statement_object_hash":hash})).unwrap()
}
fn publication(decision: &Decision, answer: bool, format: &str) -> Publication {
    let raw = serde_json::to_vec(&serde_json::json!({"review":decision})).unwrap();
    let root = ContentHash::of_bytes(b"root");
    let payload = if answer {
        RevisionPayload::AttentionAnswered(ekr_graph::AnswerOccurrence::try_from(serde_json::from_value::<ekr_core::contract_data::EkrKernelAttentionAnsweredPayload>(serde_json::json!({"answer_id":decision.decision_id,"revision_id":RevisionId::mint(),"transaction_id":ekr_core::TransactionId::mint(),"number":1,"knowledge_root":root})).unwrap()).unwrap())
    } else {
        RevisionPayload::AuthorityUpgraded {
            transition_id: EventId::mint(),
            revision_id: RevisionId::mint(),
            number: RevisionNumber::new(1),
            knowledge_root: root,
        }
    };
    let hash = ContentHash::of_bytes(&raw);
    Publication {
        event: RevisionEvent {
            application: None,
            format: format.into(),
            event_id: EventId::mint(),
            record_hash: hash,
            payload,
        },
        objects: BTreeMap::from([(
            hash,
            PublicationObject {
                bytes: raw,
                storage_class: StorageClass::Canonical,
                stored_at: Timestamp::EPOCH,
            },
        )]),
        expected_version: 1,
    }
}
fn seed(store: &dyn Store) {
    let raw = b"synthetic seed";
    let hash = ContentHash::of_bytes(raw);
    let appended = store
        .publish(&Publication {
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
            objects: BTreeMap::from([(
                hash,
                PublicationObject {
                    bytes: raw.to_vec(),
                    storage_class: StorageClass::Canonical,
                    stored_at: Timestamp::EPOCH,
                },
            )]),
            expected_version: 0,
        })
        .unwrap();
    assert_eq!(appended, Appended::Written);
}
fn proposal() -> Proposal {
    let doc = serde_json::json!({"proposal_id":SchemaProposalId::mint(),"base_schema":SchemaVersionId::mint(),"observations":[],"sources":[],"evidence":[],"additions":[],"mappings":[],"corrections":[],"explanation":"fixture"});
    let raw = serde_json::to_vec(&doc).unwrap();
    serde_json::from_value(serde_json::json!({"proposal":doc,"payload":bytes::encode(&raw),"proposal_digest":ContentHash::of_bytes(&raw).to_hex()})).unwrap()
}
fn review(p: &Proposal, d: &Decision) -> Review {
    let evidence = EvidenceId::mint();
    let hash = ContentHash::of_bytes(b"synthetic").to_hex();
    let payload = bytes::encode(b"synthetic");
    serde_json::from_value(serde_json::json!({"decision":d,"proof":payload,"policy":payload,"statement":{"payload":payload,"evidence":{"id":evidence,"content_hash":hash,"source":{"kind":"HumanStatement","identity":"synthetic"},"extracted_by":d.operator.actor,"observed_at":d.recorded_at,"confidence_bp":10000}},"review":{"review_id":ProposalReviewId::mint(),"proposal_id":p.proposal.proposal_id,"proposal_digest":p.proposal_digest,"operator":d.operator,"recorded_at":d.recorded_at,"human_proof_digest":d.proof_digest,"evidence_id":evidence,"decision":"Approved","basis":{"observed_revision":0,"evidence_digest":hash,"options_digest":hash,"effects_digest":hash}}})).unwrap()
}
fn inject(path: &Path, sqlite: bool, p: &Publication) {
    {
        let store = open(path, sqlite);
        for object in p.objects.values() {
            store
                .put(object.storage_class, &object.bytes, object.stored_at)
                .unwrap();
        }
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let provider: Box<dyn EventStore> = if sqlite {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(&path.to_string_lossy(), "ekr")
                    .await
                    .unwrap(),
            )
        } else {
            Box::new(eventlog_file::FileEventStore::open(path).await.unwrap())
        };
        let key = EventId::mint().to_string();
        let meta = CommandMeta {
            idempotency_key: key.clone(),
            request_hash: key.clone(),
            subject: "fixture".into(),
            actor: "fixture".into(),
            request_id: key.clone(),
            trace_id: key,
            causation_id: None,
            causation_depth: 0,
            occurred_at: time::OffsetDateTime::UNIX_EPOCH,
            claim: None,
        };
        let stream =
            StreamId::new(TenantId::new("test").unwrap(), "ekr.revision", "canonical").unwrap();
        provider
            .append(
                &stream,
                Expected::Exact(p.expected_version),
                &[NewEvent::new(
                    p.event.name(),
                    if p.event.format == "ekr.revision-event/5" {
                        5
                    } else {
                        p.event.schema_version()
                    },
                    serde_json::to_value(&p.event).unwrap(),
                )
                .unwrap()],
                &meta,
            )
            .await
            .unwrap();
    });
}
#[test]
fn historical_signed_decisions_reserve_ids_without_requiring_new_indexes() {
    for sqlite in [false, true] {
        for answer in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("store");
            let d = decision(AgentId::mint());
            {
                let store = open(&path, sqlite);
                seed(&*store);
            }
            let publication = publication(
                &d,
                answer,
                if answer {
                    RevisionEvent::ANSWER_FORMAT
                } else {
                    RevisionEvent::TRANSITION_FORMAT
                },
            );
            inject(&path, sqlite, &publication);
            let store = open(&path, sqlite);
            assert!(store.history().is_ok());
            assert_eq!(
                store.human_decision(&d.decision_id).unwrap(),
                Some(d.clone())
            );
            let p = proposal();
            store.retain_schema_proposal(&p, Timestamp::EPOCH).unwrap();
            let mut changed = d;
            changed.operator.authentication_subject.push_str(" changed");
            assert_eq!(
                store.retain_proposal_review(&review(&p, &changed), None, Timestamp::EPOCH),
                Err(StoreError::PublicationInputConflict)
            );
        }
    }
}
#[test]
fn new_signed_occurrences_require_matching_index_on_cold_replay() {
    for sqlite in [false, true] {
        for answer in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("store");
            {
                seed(&*open(&path, sqlite));
            }
            let d = decision(AgentId::mint());
            let p = publication(&d, answer, "ekr.revision-event/5");
            inject(&path, sqlite, &p);
            let error = open(&path, sqlite).history().unwrap_err();
            assert!(
                error.to_string().contains("human-decision-binding"),
                "{error}"
            );
        }
    }
}
#[test]
fn legacy_direct_publication_refuses_before_any_mutation() {
    for sqlite in [false, true] {
        for answer in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let store = open(&dir.path().join("store"), sqlite);
            seed(&*store);
            let p = publication(
                &decision(AgentId::mint()),
                answer,
                if answer {
                    RevisionEvent::ANSWER_FORMAT
                } else {
                    RevisionEvent::TRANSITION_FORMAT
                },
            );
            assert!(
                matches!(store.publish(&p),Err(StoreError::Document(ref reason)) if reason.contains("human-decision-revalidation-required"))
            );
            assert_eq!(store.history().unwrap().occurrences.len(), 1);
            assert!(store.get(&p.event.record_hash).unwrap().is_none());
        }
    }
}
#[test]
fn canonical_and_proposal_decisions_race_on_one_atomic_identity() {
    for sqlite in [false, true] {
        for answer in [false, true] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("store");
            let p = proposal();
            {
                let store = open(&path, sqlite);
                seed(&*store);
                store.retain_schema_proposal(&p, Timestamp::EPOCH).unwrap();
            }
            let d = decision(AgentId::mint());
            let canonical = publication(&d, answer, "ekr.revision-event/5");
            let mut different = d.clone();
            different
                .operator
                .authentication_subject
                .push_str(" different");
            let retained = review(&p, &different);
            let barrier = Arc::new(Barrier::new(2));
            let handles: Vec<_> = [false, true]
                .into_iter()
                .map(|write_review| {
                    let path = path.clone();
                    let barrier = barrier.clone();
                    let canonical = canonical.clone();
                    let retained = retained.clone();
                    std::thread::spawn(move || {
                        let store = open(&path, sqlite);
                        barrier.wait();
                        if write_review {
                            store
                                .retain_proposal_review(&retained, None, Timestamp::EPOCH)
                                .map(|(_, inserted)| inserted)
                        } else {
                            store.publish(&canonical).map(|r| r == Appended::Written)
                        }
                    })
                })
                .collect();
            let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
            assert_eq!(
                results.iter().filter(|r| matches!(r, Ok(true))).count(),
                1,
                "{results:?}"
            );
            assert_eq!(
                results
                    .iter()
                    .filter(|r| matches!(r, Err(StoreError::PublicationInputConflict)))
                    .count(),
                1,
                "{results:?}"
            );
            let store = open(&path, sqlite);
            let held = store.human_decision(&d.decision_id).unwrap().unwrap();
            assert!(held == d || held == different);
            assert!(store.history().is_ok());
        }
    }
}

/// Models the authority's established root-by-checkpoint binding fast path. The store must
/// enforce its physical signed-index rule before it delegates to this shortcut.
struct CheckpointAdmit(ekr_graph::Root);
impl CommitAuthority for CheckpointAdmit {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }
    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        Err(StoreError::Document("fixture-full-replay".into()))
    }
    fn checkpointed_head(
        &self,
        _: &RetainedHistory,
        _: ContentHash,
    ) -> Result<Option<ekr_graph::Root>, StoreError> {
        Ok(Some(self.0))
    }
}
fn checkpoint_store(path: &Path, sqlite: bool, root: ekr_graph::Root) -> Box<dyn RevisionLog> {
    if sqlite {
        Box::new(
            ekr_store::SqliteStore::sqlite(path, "test", None)
                .unwrap()
                .under(CheckpointAdmit(root)),
        )
    } else {
        Box::new(
            ekr_store::FileStore::file(path, "test", None)
                .unwrap()
                .under(CheckpointAdmit(root)),
        )
    }
}
fn checkpoint_fixture(path: &Path, sqlite: bool, signed: bool) -> ekr_graph::Root {
    seed(&*open(path, sqlite));
    if signed {
        // Corrupt native history: the answer occurrence exists, its mandatory index does not.
        let answer = publication(
            &decision(AgentId::mint()),
            true,
            RevisionEvent::SIGNED_FORMAT,
        );
        inject(path, sqlite, &answer);
    }
    let hash = ContentHash::of_bytes(b"checkpoint-fixture");
    let root = ekr_graph::Root {
        revision: RevisionNumber::new(if signed { 2 } else { 1 }),
        parent: Some(hash),
        ontology_root: hash,
        knowledge_root: hash,
        evidence_root: hash,
        agent_root: hash,
        transaction: hash,
    };
    let raw = serde_json::to_vec(&root).unwrap();
    let record_hash = ContentHash::of_bytes(&raw);
    let commit = Publication {
        event: RevisionEvent {
            application: None,
            format: RevisionEvent::FORMAT.into(),
            event_id: EventId::mint(),
            record_hash,
            payload: RevisionPayload::RevisionCommitted {
                transaction_id: ekr_core::TransactionId::mint(),
                revision_id: RevisionId::mint(),
                number: root.revision,
                knowledge_root: root.knowledge_root,
            },
        },
        objects: BTreeMap::from([(
            record_hash,
            PublicationObject {
                bytes: raw,
                storage_class: StorageClass::Canonical,
                stored_at: Timestamp::EPOCH,
            },
        )]),
        expected_version: if signed { 2 } else { 1 },
    };
    inject(path, sqlite, &commit);
    assert!(open(path, sqlite)
        .write_checkpoint(
            commit.expected_version + 1,
            hash,
            Some(b"synthetic checkpoint")
        )
        .unwrap());
    root
}
#[test]
fn checkpoint_head_after_signed_answer_checks_its_mandatory_identity_binding() {
    let mut failures = Vec::new();
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let root = checkpoint_fixture(&path, sqlite, true);
        let store = checkpoint_store(&path, sqlite, root);
        assert!(
            matches!(store.history(), Err(StoreError::Document(reason)) if reason.contains("human-decision-binding"))
        );
        let head = store.head();
        if !matches!(&head, Err(StoreError::Document(reason)) if reason.contains("human-decision-binding"))
        {
            failures.push(format!(
                "sqlite={sqlite}: checkpoint head bypassed binding: {head:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn ordinary_checkpoint_head_keeps_the_existing_shortcut() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let root = checkpoint_fixture(&path, sqlite, false);
        let store = checkpoint_store(&path, sqlite, root);
        assert!(
            matches!(store.history(), Err(StoreError::Document(reason)) if reason == "fixture-full-replay")
        );
        assert_eq!(store.head().unwrap(), Some(root));
    }
}
