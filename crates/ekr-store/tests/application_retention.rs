//! Tests the physical application protocol with an explicitly synthetic semantic authority.
//! These cases do not stand in for kernel approval/operation admission.
use ekr_core::{
    bytes, contract_data as w, AgentId, ContentHash, EventId, RevisionNumber, SchemaVersionId,
    Timestamp, TransactionId,
};
use ekr_graph::{events::ApplicationGuard, RevisionEvent, RevisionPayload};
use ekr_store::{
    AdmittedRevision, ApplicationRetention, CommitAuthority, ObjectStore, ProposalReviewRetention,
    Publication, PublicationCommandKey, PublicationCommandKind, PublicationObject, RetainedHistory,
    RevisionLog, SchemaProposalRetention, StorageClass, StoreError,
};
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
trait Store:
    ApplicationRetention + ProposalReviewRetention + SchemaProposalRetention + ObjectStore + RevisionLog
{
}
impl<
        T: ApplicationRetention
            + ProposalReviewRetention
            + SchemaProposalRetention
            + ObjectStore
            + RevisionLog,
    > Store for T
{
}
struct Authority;

#[test]
fn application_cannot_adopt_an_existing_ordinary_preparation_transaction() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let store = open(&path, sqlite);
        let (e, step, attempt, review) = fixture(&*store);
        let mut proposed = publication(&e, &step, &attempt, &review, 1);
        proposed.event.application = None;
        proposed.event.format = proposed.event.payload.format().into();
        let key = PublicationCommandKey {
            kind: PublicationCommandKind::Propose,
            transaction_id: Some(attempt.transaction_id.0.parse().unwrap()),
            predecessor_event_id: None,
            predecessor_record_hash: None,
            answer_id: None,
        };
        let input = ContentHash::of_bytes(b"original ordinary preparation");
        let original = store.prepare(&key, input, &proposed, None).unwrap();
        assert!(
            store
                .elect_application(&typed(json!({"election":e,"objects":{}})), Timestamp::EPOCH)
                .is_err(),
            "{sqlite}: application adopted an already prepared transaction"
        );
        assert!(store.retained_application_elections().unwrap().is_empty());
        // The refusal protects the original immutable request; it does not invalidate it.
        drop(store);
        let store = open(&path, sqlite);
        assert_eq!(
            store.prepare(&key, input, &proposed, None).unwrap(),
            original
        );
        assert_eq!(
            store.resume(&original).unwrap(),
            ekr_store::Appended::Written
        );
        assert_eq!(store.history().unwrap().occurrences.len(), 1);
    }
}

impl CommitAuthority for Authority {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }
    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&ekr_ontology::Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        Ok(None)
    }
    fn verify_application_history(&self, _: &RetainedHistory) -> Result<(), StoreError> {
        Ok(())
    }
    fn verify_application_attempt(
        &self,
        _: &RetainedHistory,
        _: &w::EkrIntegrateRetainedApplicationAttempt,
    ) -> Result<(), StoreError> {
        Ok(())
    }
}

#[test]
fn new_unguarded_preparation_refuses_a_current_application_reservation() {
    for sqlite in [false, true] {
        for retained_through in 0..3 {
            let dir = tempfile::tempdir().unwrap();
            let store = open(&dir.path().join("store"), sqlite);
            let (e, step, attempt, review) = fixture(&*store);
            store
                .elect_application(&typed(json!({"election":e,"objects":{}})), Timestamp::EPOCH)
                .unwrap();
            if retained_through > 0 {
                store
                    .elect_application_step(
                        &typed(json!({"step":step,"objects":{}})),
                        Timestamp::EPOCH,
                    )
                    .unwrap();
            }
            if retained_through > 1 {
                store
                    .elect_application_attempt(&typed(json!({"attempt":attempt})), Timestamp::EPOCH)
                    .unwrap();
            }
            let mut proposed = publication(&e, &step, &attempt, &review, 1);
            proposed.event.application = None;
            proposed.event.format = proposed.event.payload.format().into();
            let key = PublicationCommandKey {
                kind: PublicationCommandKind::Propose,
                transaction_id: Some(attempt.transaction_id.0.parse().unwrap()),
                predecessor_event_id: None,
                predecessor_record_hash: None,
                answer_id: None,
            };
            assert!(
                store
                    .prepare(
                        &key,
                        ContentHash::of_bytes(b"reserved ordinary request"),
                        &proposed,
                        None
                    )
                    .is_err(),
                "{sqlite}/{retained_through}: unguarded reservation prepared"
            );
            assert!(store.preparation(&key).unwrap().is_none());
            assert!(store.history().unwrap().occurrences.is_empty());
        }
    }
}
fn open(path: &Path, sqlite: bool) -> Box<dyn Store> {
    if sqlite {
        Box::new(
            ekr_store::SqliteStore::sqlite(path, "test", None)
                .unwrap()
                .under(Authority),
        )
    } else {
        Box::new(
            ekr_store::FileStore::file(path, "test", None)
                .unwrap()
                .under(Authority),
        )
    }
}
fn typed<T: serde::de::DeserializeOwned>(value: serde_json::Value) -> T {
    serde_json::from_value(value).unwrap()
}
fn fixture(
    store: &dyn Store,
) -> (
    w::EkrIntegrateRetainedApplicationElection,
    w::EkrIntegrateRetainedApplicationStep,
    w::EkrIntegrateRetainedApplicationAttempt,
    w::EkrIntegrateRetainedProposalReview,
) {
    let id = AgentId::mint();
    let actor = AgentId::mint();
    let schema = SchemaVersionId::mint();
    let document = json!({"proposal_id":id,"base_schema":schema,"observations":[],"sources":[],"evidence":[],"additions":[],"mappings":[],"corrections":[],"explanation":"Synthetic application"});
    let payload = serde_json::to_vec(&document).unwrap();
    let digest = ContentHash::of_bytes(&payload);
    let proposal = typed(
        json!({"proposal":document,"payload":bytes::encode(&payload),"proposal_digest":digest}),
    );
    store
        .retain_schema_proposal(&proposal, Timestamp::EPOCH)
        .unwrap();
    let proof = b"synthetic proof";
    let policy = b"synthetic policy";
    let statement = b"synthetic human statement";
    let proof_digest = ContentHash::of_bytes(b"protocol proof digest");
    let evidence = AgentId::mint();
    let operator = json!({"actor":actor,"authentication_subject":"synthetic-reviewer"});
    let review = typed::<w::EkrIntegrateRetainedProposalReview>(
        json!({"review":{"review_id":AgentId::mint(),"proposal_id":id,"proposal_digest":digest,"basis":{"observed_revision":0,"evidence_digest":digest,"options_digest":digest,"effects_digest":digest},"decision":"Approved","operator":operator,"evidence_id":evidence,"recorded_at":"1970-01-01T00:00:00Z","human_proof_digest":proof_digest},"decision":{"decision_id":AgentId::mint(),"operator":operator,"proof_digest":proof_digest,"policy_digest":digest,"statement_digest":digest,"proof_object_hash":ContentHash::of_bytes(proof),"policy_object_hash":ContentHash::of_bytes(policy),"statement_object_hash":ContentHash::of_bytes(statement),"recorded_at":"1970-01-01T00:00:00Z"},"proof":bytes::encode(proof),"policy":bytes::encode(policy),"statement":{"payload":bytes::encode(statement),"evidence":{"id":evidence,"content_hash":ContentHash::of_bytes(statement),"source":{"kind":"HumanStatement","identity":"synthetic-reviewer"},"extracted_by":actor,"observed_at":"1970-01-01T00:00:00Z","confidence_bp":10000}}}),
    );
    assert_eq!(
        store
            .proposal_coordination(&proposal.proposal.proposal_id)
            .unwrap()
            .stream_version
            .as_u64(),
        Some(0)
    );
    store
        .retain_proposal_review(&review, None, Timestamp::EPOCH)
        .unwrap();
    let tx = json!({"id":TransactionId::mint(),"proposer":actor,"operations":[],"evidence":[]});
    let election = typed::<w::EkrIntegrateRetainedApplicationElection>(
        json!({"application_id":AgentId::mint(),"base_schema":schema,"elected_at":"1970-01-01T00:00:00Z","initial_proof_digest":proof_digest,"initial_review_id":review.review.review_id,"proposal_digest":digest,"proposal_id":id,"schema_transaction":tx,"selected_items":[]}),
    );
    let step = typed::<w::EkrIntegrateRetainedApplicationStep>(
        json!({"application_id":election.application_id,"step_election_id":AgentId::mint(),"step":{"kind":"Schema"},"transaction":tx,"mappings":[],"derivations":[],"replacements":[],"elected_at":"1970-01-01T00:00:00Z"}),
    );
    let attempt = typed(
        json!({"transaction_id":tx["id"],"transaction":tx,"step_election_id":step.step_election_id,"elected_at":"1970-01-01T00:00:00Z"}),
    );
    (election, step, attempt, review)
}
fn elect(
    store: &dyn Store,
    e: &w::EkrIntegrateRetainedApplicationElection,
    s: &w::EkrIntegrateRetainedApplicationStep,
    a: &w::EkrIntegrateRetainedApplicationAttempt,
) {
    store
        .elect_application(&typed(json!({"election":e,"objects":{}})), Timestamp::EPOCH)
        .unwrap();
    store
        .elect_application_step(&typed(json!({"step":s,"objects":{}})), Timestamp::EPOCH)
        .unwrap();
    store
        .elect_application_attempt(&typed(json!({"attempt":a})), Timestamp::EPOCH)
        .unwrap();
}
fn publication(
    e: &w::EkrIntegrateRetainedApplicationElection,
    s: &w::EkrIntegrateRetainedApplicationStep,
    a: &w::EkrIntegrateRetainedApplicationAttempt,
    r: &w::EkrIntegrateRetainedProposalReview,
    cursor: u64,
) -> Publication {
    let guard: w::EkrIntegrateApplicationPublicationGuard = typed(
        json!({"application_id":e.application_id,"step_election_id":s.step_election_id,"proposal_id":e.proposal_id,"proposal_digest":e.proposal_digest,"review_id":r.review.review_id,"human_proof_digest":r.decision.proof_digest,"review_stream_version":cursor,"step":s.step,"attempt_transaction":a.transaction_id}),
    );
    let body = format!("synthetic ordinary record {}", EventId::mint()).into_bytes();
    let hash = ContentHash::of_bytes(&body);
    Publication {
        event: RevisionEvent {
            format: RevisionEvent::APPLICATION_FORMAT.into(),
            event_id: EventId::mint(),
            record_hash: hash,
            payload: RevisionPayload::TransactionProposed {
                transaction_id: a.transaction_id.0.parse().unwrap(),
                proposer: a.transaction.proposer.0.parse().unwrap(),
                operations_hash: None,
            },
            application: Some(ApplicationGuard::try_from(guard).unwrap()),
        },
        objects: BTreeMap::from([(
            hash,
            PublicationObject {
                storage_class: StorageClass::Canonical,
                stored_at: Timestamp::EPOCH,
                bytes: body,
            },
        )]),
        expected_version: 0,
    }
}
#[test]
fn immutable_election_step_attempt_retry_and_reopen_preserve_winners() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let store = open(&path, sqlite);
        let (e, s, a, _) = fixture(&*store);
        elect(&*store, &e, &s, &a);
        assert_eq!(
            store
                .elect_application(&typed(json!({"election":e,"objects":{}})), Timestamp::EPOCH)
                .unwrap(),
            (e.clone(), false)
        );
        let mut rival = e.clone();
        rival.application_id.0 = AgentId::mint().to_string();
        rival.schema_transaction.id.0 = TransactionId::mint().to_string();
        assert_eq!(
            store
                .elect_application(
                    &typed(json!({"election":rival,"objects":{}})),
                    Timestamp::EPOCH
                )
                .unwrap(),
            (e.clone(), false)
        );
        let mut changed = e.clone();
        changed.selected_items.push(Box::new(typed(json!({"source":{"interpretation_id":AgentId::mint(),"version":1,"document_digest":e.proposal_digest},"item":"facts[0]","mapping_digest":e.proposal_digest}))));
        assert!(store
            .elect_application(
                &typed(json!({"election":changed,"objects":{}})),
                Timestamp::EPOCH
            )
            .is_err());
        drop(store);
        let store = open(&path, sqlite);
        assert_eq!(
            store.retained_application_elections().unwrap(),
            vec![e.clone()]
        );
        assert_eq!(
            store.retained_application_steps(&e.application_id).unwrap(),
            vec![s.clone()]
        );
        assert_eq!(
            store
                .retained_application_attempts(&s.step_election_id)
                .unwrap(),
            vec![a.clone()]
        );
        let captured: ekr_store::ApplicationHistory = store.history().unwrap().applications;
        assert_eq!(captured.attempts(), &[Box::new(a)]);
    }
}
#[test]
fn guarded_publication_has_exact_marker_and_marker_does_not_change_human_predecessor() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let store = open(&path, sqlite);
        let (e, s, a, r) = fixture(&*store);
        elect(&*store, &e, &s, &a);
        let p = publication(&e, &s, &a, &r, 1);
        assert_eq!(store.publish(&p).unwrap(), ekr_store::Appended::Written);
        let coordination = store.proposal_coordination(&e.proposal_id).unwrap();
        assert_eq!(coordination.stream_version.as_u64(), Some(2));
        assert_eq!(coordination.entries.len(), 2);
        assert_eq!(
            store.retained_proposal_reviews(&e.proposal_id).unwrap(),
            vec![r.clone()]
        );
        assert_eq!(store.history().unwrap().occurrences.len(), 1);
        drop(store);
        let store = open(&path, sqlite);
        assert_eq!(store.history().unwrap().occurrences[0].event, p.event);
        let mut rejection = r.clone();
        rejection.review.review_id.0 = AgentId::mint().to_string();
        rejection.decision.decision_id = AgentId::mint().to_string();
        rejection.decision.proof_digest.0 = ContentHash::of_bytes(b"another protocol").to_hex();
        rejection.review.human_proof_digest = rejection.decision.proof_digest.clone();
        *rejection.review.decision = w::EkrIntegrateReviewDecision::V1;
        store
            .retain_proposal_review(
                &rejection,
                Some(r.decision.proof_digest.0.parse().unwrap()),
                Timestamp::EPOCH,
            )
            .unwrap();
        assert_eq!(
            store
                .proposal_coordination(&e.proposal_id)
                .unwrap()
                .stream_version
                .as_u64(),
            Some(3)
        );
    }
}
#[test]
fn prepared_marker_loses_to_rejection_without_orphan_ordinary_event() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let store = open(&path, sqlite);
        let (e, s, a, r) = fixture(&*store);
        elect(&*store, &e, &s, &a);
        let p = publication(&e, &s, &a, &r, 1);
        let key = PublicationCommandKey {
            kind: PublicationCommandKind::Propose,
            transaction_id: Some(a.transaction_id.0.parse().unwrap()),
            predecessor_event_id: None,
            predecessor_record_hash: None,
            answer_id: None,
        };
        let prepared = store
            .prepare(&key, ContentHash::of_bytes(b"input"), &p, None)
            .unwrap();
        assert_eq!(
            prepared.format,
            ekr_store::PublicationPreparationV1::FORMAT_V7
        );
        for legacy in [
            ekr_store::PublicationPreparationV1::FORMAT_V4,
            ekr_store::PublicationPreparationV1::FORMAT_V5,
        ] {
            let mut downgraded = prepared.clone();
            downgraded.format = legacy.into();
            assert!(store.resume(&downgraded).is_err());
            assert!(store.history().unwrap().occurrences.is_empty());
        }
        let mut rejection = r.clone();
        rejection.review.review_id.0 = AgentId::mint().to_string();
        rejection.decision.decision_id = AgentId::mint().to_string();
        rejection.decision.proof_digest.0 = ContentHash::of_bytes(b"rejected protocol").to_hex();
        rejection.review.human_proof_digest = rejection.decision.proof_digest.clone();
        *rejection.review.decision = w::EkrIntegrateReviewDecision::V1;
        store
            .retain_proposal_review(
                &rejection,
                Some(r.decision.proof_digest.0.parse().unwrap()),
                Timestamp::EPOCH,
            )
            .unwrap();
        assert!(store.resume(&prepared).is_err());
        assert!(store.history().unwrap().occurrences.is_empty());
        assert_eq!(
            store
                .proposal_coordination(&e.proposal_id)
                .unwrap()
                .stream_version
                .as_u64(),
            Some(2)
        );
        assert!(store.get(&p.event.record_hash).unwrap().is_none());
    }
}

fn inject(
    path: &Path,
    sqlite: bool,
    stream: (&str, &str),
    expected: u64,
    name: &str,
    version: u32,
    data: serde_json::Value,
) {
    let (stream_type, stream_id) = stream;
    use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};
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
            subject: "synthetic-tamper".into(),
            actor: "synthetic-test".into(),
            request_id: key.clone(),
            trace_id: key,
            causation_id: None,
            causation_depth: 0,
            occurred_at: time::OffsetDateTime::UNIX_EPOCH,
            claim: None,
        };
        provider
            .append(
                &StreamId::new(TenantId::new("test").unwrap(), stream_type, stream_id).unwrap(),
                if expected == 0 {
                    Expected::NoStream
                } else {
                    Expected::Exact(expected)
                },
                &[NewEvent::new(name, version, data).unwrap()],
                &meta,
            )
            .await
            .unwrap();
    });
}
fn marker(p: &Publication) -> serde_json::Value {
    json!({"guard":p.event.application.as_ref().unwrap().as_data(),"event_id":p.event.event_id,"record_hash":p.event.record_hash,"transaction_id":p.event.transaction_id(),"command":"Propose"})
}
#[test]
fn cold_replay_refuses_missing_extra_mismatched_and_zero_cursor_markers() {
    for sqlite in [false, true] {
        for mode in [
            "missing",
            "unknown-proposal",
            "changed-record",
            "zero-cursor",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("store");
            let store = open(&path, sqlite);
            let (e, s, a, r) = fixture(&*store);
            let p = publication(&e, &s, &a, &r, 1);
            if mode != "unknown-proposal" {
                elect(&*store, &e, &s, &a);
                for object in p.objects.values() {
                    store
                        .put(object.storage_class, &object.bytes, object.stored_at)
                        .unwrap();
                }
            }
            drop(store);
            if mode != "unknown-proposal" {
                inject(
                    &path,
                    sqlite,
                    ("ekr.revision", "canonical"),
                    0,
                    p.event.name(),
                    6,
                    serde_json::to_value(&p.event).unwrap(),
                );
            }
            if mode != "missing" {
                let mut record = marker(&p);
                if mode == "changed-record" {
                    record["record_hash"] = json!(ContentHash::of_bytes(b"wrong-record"));
                }
                if mode == "zero-cursor" {
                    record["guard"]["review_stream_version"] = json!(0);
                }
                let stream_id = if mode == "unknown-proposal" {
                    AgentId::mint().to_string()
                } else {
                    e.proposal_id.0.clone()
                };
                inject(
                    &path,
                    sqlite,
                    ("ekr.integrate.proposal-reviews", &stream_id),
                    if mode == "unknown-proposal" { 0 } else { 1 },
                    "ekr.integrate.ApplicationPublicationRecorded",
                    1,
                    record,
                );
            }
            assert!(open(&path, sqlite).history().is_err(), "{sqlite}/{mode}");
        }
    }
}
#[test]
fn missing_independent_source_bytes_refuse_election_before_any_record() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store = open(&dir.path().join("store"), sqlite);
        let (mut e, _, _, _) = fixture(&*store);
        e.selected_items.push(Box::new(typed(json!({"source":{"interpretation_id":AgentId::mint(),"version":1,"document_digest":ContentHash::of_bytes(b"absent-source")},"item":"facts[0]","mapping_digest":ContentHash::of_bytes(b"mapping")}))));
        let result =
            store.elect_application(&typed(json!({"election":e,"objects":{}})), Timestamp::EPOCH);
        assert!(
            result.is_err(),
            "{sqlite}: source bytes absent, yet election was retained"
        );
        assert!(store.retained_application_elections().unwrap().is_empty());
    }
}
#[test]
fn renewed_review_reprepares_same_ordinary_input_without_mutating_prior_attempt() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let store = open(&path, sqlite);
        let (e, s, a, r) = fixture(&*store);
        elect(&*store, &e, &s, &a);
        let p = publication(&e, &s, &a, &r, 1);
        let key = PublicationCommandKey {
            kind: PublicationCommandKind::Propose,
            transaction_id: Some(a.transaction_id.0.parse().unwrap()),
            predecessor_event_id: None,
            predecessor_record_hash: None,
            answer_id: None,
        };
        let input = ContentHash::of_bytes(b"unchanged command input");
        let prior = store.prepare(&key, input, &p, None).unwrap();
        let mut next_review = r.clone();
        next_review.review.review_id.0 = AgentId::mint().to_string();
        next_review.decision.decision_id = AgentId::mint().to_string();
        next_review.decision.proof_digest.0 = ContentHash::of_bytes(b"renewed approval").to_hex();
        next_review.review.human_proof_digest = next_review.decision.proof_digest.clone();
        store
            .retain_proposal_review(
                &next_review,
                Some(r.decision.proof_digest.0.parse().unwrap()),
                Timestamp::EPOCH,
            )
            .unwrap();
        assert!(
            matches!(store.resume(&prior), Err(StoreError::Conflict)),
            "{sqlite}"
        );
        let mut refreshed = p.clone();
        let mut guard = refreshed
            .event
            .application
            .as_ref()
            .unwrap()
            .as_data()
            .clone();
        guard.review_id = next_review.review.review_id.clone();
        guard.human_proof_digest = next_review.decision.proof_digest.clone();
        guard.review_stream_version = 2.into();
        refreshed.event.application = Some(ApplicationGuard::try_from(guard).unwrap());
        let next = store
            .prepare(&key, input, &refreshed, Some(&prior))
            .unwrap();
        assert_eq!(next.attempt_number, prior.attempt_number + 1);
        assert_eq!(
            next.previous_attempt_hash,
            Some(ContentHash::of_bytes(&serde_json::to_vec(&prior).unwrap()))
        );
        let _ = store.resume(&next).unwrap();
        drop(store);
        let reopened = open(&path, sqlite);
        assert_eq!(reopened.history().unwrap().occurrences.len(), 1);
        assert_eq!(
            reopened
                .proposal_coordination(&e.proposal_id)
                .unwrap()
                .stream_version
                .as_u64(),
            Some(3)
        );
        assert_eq!(reopened.prepare(&key, input, &p, None).unwrap(), next);
        assert_eq!(
            reopened.resume(&next).unwrap(),
            ekr_store::Appended::AlreadyRecorded
        );
    }
}

#[test]
fn independent_source_is_pinned_and_replayed_without_an_incubation_root() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let store = open(&path, sqlite);
        let (mut election, _, _, _) = fixture(&*store);
        let interpretation = AgentId::mint();
        let document = json!({"version":{"interpretation_id":interpretation,"version":1},"root_id":AgentId::mint(),"observations":[],"local_schema":{"node_types":[],"edge_types":[]},"entities":[],"facts":[],"evidence":[]});
        let payload = serde_json::to_vec(&document).unwrap();
        let address = store
            .put(StorageClass::Incubating, &payload, Timestamp::EPOCH)
            .unwrap()
            .content_hash;
        election.selected_items.push(Box::new(typed(json!({"source":{"interpretation_id":interpretation,"version":1,"document_digest":address},"item":"facts[0]","mapping_digest":ContentHash::of_bytes(b"mapping")}))));
        store
            .elect_application(
                &typed(json!({"election":election,"objects":{}})),
                Timestamp::EPOCH,
            )
            .unwrap();
        drop(store);
        let reopened = open(&path, sqlite);
        let history = reopened.history().unwrap();
        assert_eq!(
            history.content(address, StorageClass::Provenance).unwrap(),
            payload
        );
        assert_eq!(history.applications.elections().len(), 1);
    }
}

#[test]
fn concurrent_elections_preserve_one_winner_on_both_providers() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let store = open(&path, sqlite);
        let (first, _, _, _) = fixture(&*store);
        let mut second = first.clone();
        second.application_id.0 = AgentId::mint().to_string();
        second.schema_transaction.id.0 = TransactionId::mint().to_string();
        drop(store);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let results = std::thread::scope(|scope| {
            let handles = [first, second]
                .into_iter()
                .map(|election| {
                    let barrier = barrier.clone();
                    let path = &path;
                    scope.spawn(move || {
                        let store = open(path, sqlite);
                        barrier.wait();
                        store.elect_application(
                            &typed(json!({"election":election,"objects":{}})),
                            Timestamp::EPOCH,
                        )
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|h| h.join().unwrap())
                .collect::<Vec<_>>()
        });
        assert!(results.iter().all(Result::is_ok), "{sqlite}: {results:?}");
        let rows = results.into_iter().map(Result::unwrap).collect::<Vec<_>>();
        assert_eq!(rows[0].0, rows[1].0);
        assert_eq!(rows.iter().filter(|r| r.1).count(), 1);
        assert_eq!(
            open(&path, sqlite)
                .retained_application_elections()
                .unwrap(),
            vec![rows[0].0.clone()]
        );
    }
}

#[test]
fn processing_receipt_requires_retained_source_and_mapping_bytes() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let store = open(&path, sqlite);
        let document = b"independent processing source";
        let mapping = b"immutable mapping";
        let receipt: w::EkrIntegrateProcessingReceiptSnapshot = typed(
            json!({"receipt_id":AgentId::mint(),"basis_digest":ContentHash::of_bytes(b"basis"),"document_digest":ContentHash::of_bytes(document),"mapping_digest":ContentHash::of_bytes(mapping),"item":"facts[0]","assertions":[],"disposition":"Parked"}),
        );
        assert!(
            store
                .retain_processing_receipt(&receipt, Timestamp::EPOCH)
                .is_err(),
            "{sqlite}: absent bytes admitted"
        );
        assert!(store.retained_processing_receipts().unwrap().is_empty());
        store
            .put(StorageClass::Incubating, document, Timestamp::EPOCH)
            .unwrap();
        assert!(
            store
                .retain_processing_receipt(&receipt, Timestamp::EPOCH)
                .is_err(),
            "{sqlite}: absent mapping admitted"
        );
        store
            .put(StorageClass::Incubating, mapping, Timestamp::EPOCH)
            .unwrap();
        assert!(
            store
                .retain_processing_receipt(&receipt, Timestamp::EPOCH)
                .unwrap()
                .1
        );
        drop(store);
        let reopened = open(&path, sqlite);
        assert_eq!(
            reopened.retained_processing_receipts().unwrap(),
            vec![receipt.clone()]
        );
        assert_eq!(
            reopened
                .history()
                .unwrap()
                .applications
                .processing_receipts(),
            &[Box::new(receipt.clone())]
        );
        assert!(
            !reopened
                .retain_processing_receipt(&receipt, Timestamp::EPOCH)
                .unwrap()
                .1
        );
        for payload in [document.as_slice(), mapping.as_slice()] {
            assert!(
                reopened
                    .put(StorageClass::Cache, payload, Timestamp::EPOCH)
                    .unwrap()
                    .storage_class
                    .retention_rank()
                    >= StorageClass::Provenance.retention_rank()
            );
        }
    }
}

#[test]
fn default_semantic_authority_refuses_retained_application_history_and_new_attempts() {
    struct DefaultAuthority;
    impl CommitAuthority for DefaultAuthority {
        fn required_objects(
            &self,
            _: &RetainedHistory,
        ) -> Result<BTreeSet<ContentHash>, StoreError> {
            Ok(BTreeSet::new())
        }
        fn replay(
            &self,
            _: &RetainedHistory,
            _: Option<&ekr_ontology::Ontology>,
            _: Option<RevisionNumber>,
        ) -> Result<Option<AdmittedRevision>, StoreError> {
            Ok(None)
        }
    }
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let store = open(&path, sqlite);
        let (e, s, a, _) = fixture(&*store);
        store
            .elect_application(&typed(json!({"election":e,"objects":{}})), Timestamp::EPOCH)
            .unwrap();
        store
            .elect_application_step(&typed(json!({"step":s,"objects":{}})), Timestamp::EPOCH)
            .unwrap();
        drop(store);
        let closed: Box<dyn Store> = if sqlite {
            Box::new(
                ekr_store::SqliteStore::sqlite(&path, "test", None)
                    .unwrap()
                    .under(DefaultAuthority),
            )
        } else {
            Box::new(
                ekr_store::FileStore::file(&path, "test", None)
                    .unwrap()
                    .under(DefaultAuthority),
            )
        };
        assert!(closed.history().is_err());
        assert!(closed.head().is_err());
        assert!(closed
            .elect_application_attempt(&typed(json!({"attempt":a})), Timestamp::EPOCH)
            .is_err());
        assert!(closed
            .retained_application_attempts(&s.step_election_id)
            .unwrap()
            .is_empty());
    }
}

#[test]
fn closed_application_retention_feed_refuses_unknown_tables_and_versions() {
    for sqlite in [false, true] {
        for (table, name, version) in [
            ("future", "ekr.integrate.Future", 1),
            ("elections", "ekr.integrate.ApplicationElected", 99),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("store");
            drop(open(&path, sqlite));
            inject(
                &path,
                sqlite,
                ("ekr.integrate.application-retention", table),
                0,
                name,
                version,
                json!({}),
            );
            assert!(
                open(&path, sqlite).history().is_err(),
                "{sqlite}/{table}/{version}: unknown physical envelope admitted"
            );
        }
    }
}

#[test]
fn selected_history_closes_exact_marker_without_loading_future_application_inputs() {
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let store = open(&path, sqlite);
        let (first, step, attempt, review) = fixture(&*store);
        elect(&*store, &first, &step, &attempt);
        let mut commit = publication(&first, &step, &attempt, &review, 1);
        commit.event.payload = RevisionPayload::RevisionCommitted {
            transaction_id: attempt.transaction_id.0.parse().unwrap(),
            revision_id: ekr_core::RevisionId::mint(),
            number: RevisionNumber::new(1),
            knowledge_root: ContentHash::of_bytes(b"synthetic committed state"),
        };
        let _ = store.publish(&commit).unwrap();
        let (future, future_step, future_attempt, _) = fixture(&*store);
        elect(&*store, &future, &future_step, &future_attempt);
        drop(store);
        let reopened = open(&path, sqlite);
        let history = reopened.history_at(RevisionNumber::new(1)).unwrap();
        assert_eq!(history.applications.canonical_through(), Some(1));
        assert_eq!(
            history.applications.elections().len(),
            1,
            "{sqlite}: future election crossed historical boundary"
        );
        assert!(
            !history
                .objects
                .contains_key(&future.proposal_digest.0.parse().unwrap()),
            "{sqlite}: future proposal bytes loaded"
        );
        assert_eq!(
            history.applications.coordination()[0].entries.len(),
            2,
            "marker after canonical event must stay in atomic closure"
        );
        assert_eq!(
            reopened.history().unwrap().applications.canonical_through(),
            None
        );
        assert_eq!(
            reopened.history().unwrap().applications.elections().len(),
            2
        );
        drop(reopened);
        // Corrupt only a future pinned object through the native test port. A historical
        // capture must not fetch it, while a complete capture must still refuse its absence.
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            use eventlog_core::{EventStore, TenantId};
            let provider: Box<dyn EventStore> = if sqlite {
                Box::new(
                    eventlog_sqlite::SqliteEventStore::open(&path.to_string_lossy(), "ekr")
                        .await
                        .unwrap(),
                )
            } else {
                Box::new(eventlog_file::FileEventStore::open(&path).await.unwrap())
            };
            provider
                .delete_blob(&TenantId::new("test").unwrap(), &future.proposal_digest.0)
                .await
                .unwrap();
        });
        assert!(open(&path, sqlite)
            .history_at(RevisionNumber::new(1))
            .is_ok());
        assert!(open(&path, sqlite).history().is_err());
    }
}

#[test]
fn old_preparation_captures_only_its_original_prefix_and_exact_candidate() {
    struct CaptureAuthority {
        allowed: String,
    }
    impl CommitAuthority for CaptureAuthority {
        fn required_objects(
            &self,
            _: &RetainedHistory,
        ) -> Result<BTreeSet<ContentHash>, StoreError> {
            Ok(BTreeSet::new())
        }
        fn replay(
            &self,
            _: &RetainedHistory,
            _: Option<&ekr_ontology::Ontology>,
            _: Option<RevisionNumber>,
        ) -> Result<Option<AdmittedRevision>, StoreError> {
            Ok(None)
        }
        fn verify_application_history(&self, h: &RetainedHistory) -> Result<(), StoreError> {
            if h.applications.canonical_through().is_none()
                || h.applications.elections().len() != 1
                || h.applications.elections()[0].application_id.0 != self.allowed
                || !h.applications.receipts().is_empty()
            {
                return Err(StoreError::Document(
                    "preparation loaded unrelated current state".into(),
                ));
            }
            Ok(())
        }
    }
    for sqlite in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("store");
        let store = open(&path, sqlite);
        let (e, step, attempt, review) = fixture(&*store);
        elect(&*store, &e, &step, &attempt);
        let p = publication(&e, &step, &attempt, &review, 1);
        let key = PublicationCommandKey {
            kind: PublicationCommandKind::Propose,
            transaction_id: Some(attempt.transaction_id.0.parse().unwrap()),
            predecessor_event_id: None,
            predecessor_record_hash: None,
            answer_id: None,
        };
        let input = ContentHash::of_bytes(b"original exact preparation");
        let prepared = store.prepare(&key, input, &p, None).unwrap();
        let (future, future_step, future_attempt, _) = fixture(&*store);
        elect(&*store, &future, &future_step, &future_attempt);
        drop(store);
        let authority = CaptureAuthority {
            allowed: e.application_id.0.clone(),
        };
        let reopened: Box<dyn Store> = if sqlite {
            Box::new(
                ekr_store::SqliteStore::sqlite(&path, "test", None)
                    .unwrap()
                    .under(authority),
            )
        } else {
            Box::new(
                ekr_store::FileStore::file(&path, "test", None)
                    .unwrap()
                    .under(authority),
            )
        };
        assert_eq!(
            reopened.prepare(&key, input, &p, None).unwrap(),
            prepared,
            "{sqlite}: immutable original preparation"
        );
    }
}
