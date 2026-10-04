//! Reviewed schema application must use ordinary canonical transactions and survive replay.
#![allow(unused_imports, dead_code)]
include!("support/schema_proposal_fixture.rs");

fn captured_application_history(
    path: &std::path::Path,
    sqlite: bool,
    human: &Human,
) -> (ekr_kernel::KernelAuthority, ekr_store::RetainedHistory) {
    use ekr_store::RevisionLog;
    let mut captured = None;
    if sqlite {
        let _kernel = ekr_kernel::Commit::over_with_review_authority(
            context(),
            anchor(),
            human.binding.clone(),
            |authority| {
                let store = ekr_store::SqliteStore::sqlite(path, "upgrade-fixture", None)?
                    .under(authority.clone());
                captured = Some((authority, store.history()?));
                Ok(store)
            },
        )
        .unwrap();
    } else {
        let _kernel = ekr_kernel::Commit::over_with_review_authority(
            context(),
            anchor(),
            human.binding.clone(),
            |authority| {
                let store = ekr_store::FileStore::file(path, "upgrade-fixture", None)?
                    .under(authority.clone());
                captured = Some((authority, store.history()?));
                Ok(store)
            },
        )
        .unwrap();
    }
    captured.unwrap()
}

#[test]
fn kernel_replay_refuses_removed_or_rebound_application_guards_after_a_warm_read() {
    use ekr_store::CommitAuthority;
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (store, human) = upgraded(&path, sqlite);
        let shown = store
            .submit_schema_proposal(&proposal_input(&store), Timestamp::from_millis(2))
            .unwrap();
        let approved = store
            .approve_schema_proposal(
                &signed_review(
                    &human,
                    &shown,
                    true,
                    ekr_core::EventId::mint(),
                    b"Approve the exact additions.",
                ),
                Timestamp::from_millis(3),
            )
            .unwrap();
        store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approved.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(4),
            )
            .unwrap();
        let (authority, history) = captured_application_history(&path, sqlite, &human);
        authority.verify(&history, None, None).unwrap();
        let guarded: Vec<_> = history
            .occurrences
            .iter()
            .enumerate()
            .filter_map(|(i, o)| o.event.application.is_some().then_some(i))
            .collect();
        assert_eq!(
            guarded.len(),
            3,
            "Propose, Validate and Commit each have a marker"
        );
        for index in guarded {
            let mut removed = history.clone();
            let event = &mut removed.occurrences[index].event;
            event.application = None;
            event.format = event.payload.format().into();
            assert!(
                authority.verify(&removed, None, None).is_err(),
                "unguarded elected event admitted"
            );
            let mut rebound = history.clone();
            let mut guard = rebound.occurrences[index]
                .event
                .application
                .as_ref()
                .unwrap()
                .as_data()
                .clone();
            guard.review_id = Box::new(w::EkrIntegrateProposalReviewId(
                ekr_core::EventId::mint().to_string(),
            ));
            rebound.occurrences[index].event.application =
                Some(ekr_graph::events::ApplicationGuard::try_from(guard).unwrap());
            assert!(
                authority.verify(&rebound, None, None).is_err(),
                "marker/review mismatch admitted"
            );
            authority.verify(&history, None, None).unwrap();
        }
    }
}

fn upgraded(path: &std::path::Path, sqlite: bool) -> (Runtime, Human) {
    let store = runtime(path, sqlite);
    let seeded = store.seed(seed(), || Timestamp::EPOCH).unwrap();
    let human = schema_human(seeded.seed_hash);
    let store = store.with_review_authority(human.binding.clone()).unwrap();
    let preview = store.preview_upgrade(&human.policy).unwrap();
    store
        .apply_upgrade(
            &preview,
            &human.policy,
            &human.proof(&preview),
            b"reviewed contradictions and pending validations",
            || Timestamp::from_millis(1),
        )
        .unwrap();
    (store, human)
}

fn approved_schema(
    store: &Runtime,
    human: &Human,
) -> (
    w::EkrIntegrateSchemaProposalRead,
    w::EkrIntegrateProposalReviewSnapshot,
) {
    let shown = store
        .submit_schema_proposal(&proposal_input(store), Timestamp::from_millis(2))
        .unwrap();
    let approval = store
        .approve_schema_proposal(
            &signed_review(
                human,
                &shown,
                true,
                ekr_core::EventId::mint(),
                b"Approve the exact additions.",
            ),
            Timestamp::from_millis(3),
        )
        .unwrap();
    (shown, approval)
}

#[test]
fn application_time_before_approval_refuses_without_retaining_an_invalid_election() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (store, human) = upgraded(&path, sqlite);
        let (shown, approval) = approved_schema(&store, &human);
        let before = store.published_events().unwrap();
        assert!(store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approval.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(2)
            )
            .is_err());
        assert_eq!(
            store.published_events().unwrap(),
            before,
            "{sqlite}: refusal persisted invalid application metadata"
        );
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding)
            .unwrap();
        reopened.set_full_replay(true);
        reopened.read(None).unwrap();
    }
}

// Copies a closed test provider, retaining the exact original approval and seed identities.
fn copy_closed_provider(from: &std::path::Path, to: &std::path::Path) {
    if from.is_dir() {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            copy_closed_provider(&entry.path(), &to.join(entry.file_name()));
        }
    } else {
        std::fs::copy(from, to).unwrap();
    }
}

#[test]
fn a_reserved_election_or_step_transaction_cannot_be_proposed_without_an_attempt() {
    use ekr_store::ApplicationRetention;
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("source");
        let (store, human) = upgraded(&path, sqlite);
        let (shown, approval) = approved_schema(&store, &human);
        drop(store);
        let snapshots = [
            directory.path().join("election"),
            directory.path().join("step"),
        ];
        for snapshot in &snapshots {
            copy_closed_provider(&path, snapshot);
        }
        let store = runtime(&path, sqlite)
            .with_review_authority(human.binding.clone())
            .unwrap();
        store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approval.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(4),
            )
            .unwrap();
        let (_, complete) = captured_application_history(&path, sqlite, &human);
        let election = complete.applications.elections()[0].clone();
        let step = complete.applications.steps()[0].clone();
        let elected_id: TransactionId = election.schema_transaction.id.0.parse().unwrap();
        let document = store.transactions().unwrap()[&elected_id]
            .proposal
            .document_bytes
            .clone();
        for (include_step, snapshot) in snapshots.iter().enumerate() {
            // Reconstruct the genuine retained prefix at either interruption boundary using
            // the exact records elected by the real application, never a fabricated template.
            let (authority, _) = captured_application_history(snapshot, sqlite, &human);
            let retained: Box<dyn ApplicationRetention> = if sqlite {
                Box::new(
                    ekr_store::SqliteStore::sqlite(snapshot, "upgrade-fixture", None)
                        .unwrap()
                        .under(authority),
                )
            } else {
                Box::new(
                    ekr_store::FileStore::file(snapshot, "upgrade-fixture", None)
                        .unwrap()
                        .under(authority),
                )
            };
            retained
                .elect_application(
                    &w::EkrStoreApplicationElectionRetention {
                        election: election.clone(),
                        objects: w::EkrStoreApplicationElectionRetentionObjects {
                            ess_extra: Default::default(),
                        },
                    },
                    Timestamp::from_millis(4),
                )
                .unwrap();
            if include_step == 1 {
                retained
                    .elect_application_step(
                        &w::EkrStoreApplicationStepRetention {
                            step: step.clone(),
                            objects: w::EkrStoreApplicationStepRetentionObjects {
                                ess_extra: Default::default(),
                            },
                        },
                        Timestamp::from_millis(4),
                    )
                    .unwrap();
            }
            drop(retained);
            let store = runtime(snapshot, sqlite)
                .with_review_authority(human.binding.clone())
                .unwrap();
            store.read(None).unwrap();
            let before = store.published_events().unwrap();
            assert!(
                store
                    .propose(&document, context().operator, || {
                        Timestamp::from_millis(5)
                    })
                    .is_err(),
                "{sqlite}/{include_step}: reserved ID admitted without attempt"
            );
            assert_eq!(
                store.published_events().unwrap(),
                before,
                "{sqlite}/{include_step}: refusal wrote an occurrence or preparation"
            );
            drop(store);
            let mut reopened = runtime(snapshot, sqlite)
                .with_review_authority(human.binding.clone())
                .unwrap();
            reopened.set_full_replay(true);
            reopened.read(None).unwrap();
            let recovered = reopened
                .apply_schema_proposal(
                    &shown.proposal.proposal_id,
                    &approval.review_id,
                    &shown.proposal_digest,
                    Timestamp::from_millis(6),
                )
                .unwrap();
            assert_eq!(recovered.schema_transaction, election.schema_transaction.id);
            assert_eq!(*recovered.progress, w::EkrIntegrateApplicationProgress::V0);
        }
    }
}

fn schema_only_application(sqlite: bool) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("store");
    let (store, human) = upgraded(&path, sqlite);
    let proposal = proposal_input(&store);
    let shown = store
        .submit_schema_proposal(&proposal, Timestamp::from_millis(2))
        .unwrap();
    let approval = store
        .approve_schema_proposal(
            &signed_review(
                &human,
                &shown,
                true,
                ekr_core::EventId::mint(),
                b"Approve exact schema additions and retained support.",
            ),
            Timestamp::from_millis(3),
        )
        .unwrap();
    let before = store.read(None).unwrap();
    let report = store
        .apply_schema_proposal(
            &shown.proposal.proposal_id,
            &approval.review_id,
            &shown.proposal_digest,
            Timestamp::from_millis(4),
        )
        .unwrap();
    assert_eq!(*report.progress, w::EkrIntegrateApplicationProgress::V0);
    assert!(!report.already_complete);
    assert!(report.remaining_items.is_empty());
    assert!(!report.corrections_pending);
    let w::EssPresence::Present(revision) = report.schema_revision else {
        panic!("completed schema application has no committed revision");
    };
    assert_eq!(revision.0.as_u64().unwrap(), before.root.revision.get() + 1);
    assert!(matches!(report.receipt_id, w::EssPresence::Present(_)));
    let after = store.read(None).unwrap();
    assert_eq!(after.graph.assertions, before.graph.assertions);
    assert!(after
        .graph
        .ontology
        .to_document()
        .node_types
        .iter()
        .any(|node| node.name == "ReviewVocabulary"));
    let transaction_id: TransactionId = report.schema_transaction.0.parse().unwrap();
    assert_eq!(
        after.transactions[&transaction_id].state(),
        TransactionState::Committed
    );
    let history = store.schema_history(after.root.revision).unwrap();
    let support = &history.supporting_evidence[&after.root.revision];
    for id in &proposal.proposal.evidence {
        assert!(support.contains(&id.0.parse().unwrap()));
    }
    assert!(support.contains(&approval.evidence_id.0.parse().unwrap()));
    let events = store.published_events().unwrap();
    let repeated = store
        .apply_schema_proposal(
            &shown.proposal.proposal_id,
            &approval.review_id,
            &shown.proposal_digest,
            Timestamp::from_millis(5),
        )
        .unwrap();
    assert!(repeated.already_complete);
    assert_eq!(repeated.application_id, report.application_id);
    assert_eq!(repeated.schema_transaction, report.schema_transaction);
    assert_eq!(store.published_events().unwrap(), events);
    drop(store);
    let mut reopened = runtime(&path, sqlite)
        .with_review_authority(human.binding)
        .unwrap();
    reopened.set_full_replay(true);
    assert_eq!(reopened.read(None).unwrap().root, after.root);
    let retained = reopened
        .schema_proposal(&shown.proposal.proposal_id)
        .unwrap();
    let w::EssPresence::Present(application) = retained.application else {
        panic!("reopened proposal lost its application history");
    };
    assert_eq!(application.election.application_id, report.application_id);
    assert!(!application.receipts.is_empty());
    assert!(application.remaining_items.is_empty());
    let repeated = reopened
        .apply_schema_proposal(
            &shown.proposal.proposal_id,
            &approval.review_id,
            &shown.proposal_digest,
            Timestamp::from_millis(6),
        )
        .unwrap();
    assert!(repeated.already_complete);
    assert_eq!(reopened.published_events().unwrap(), events);
}

#[test]
fn schema_only_application_is_durable_and_idempotent_on_file() {
    schema_only_application(false);
}

#[test]
fn schema_only_application_is_durable_and_idempotent_on_sqlite() {
    schema_only_application(true);
}

#[test]
fn schema_application_requires_current_exact_human_approval_before_publication() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (store, human) = upgraded(&directory.path().join("store"), sqlite);
        let proposal = proposal_input(&store);
        let shown = store
            .submit_schema_proposal(&proposal, Timestamp::from_millis(2))
            .unwrap();
        let absent_review = w::EkrIntegrateProposalReviewId(ekr_core::EventId::mint().to_string());
        let events = store.published_events().unwrap();
        assert!(store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &absent_review,
                &shown.proposal_digest,
                Timestamp::from_millis(3)
            )
            .is_err());
        assert_eq!(store.published_events().unwrap(), events);
        let approval = store
            .approve_schema_proposal(
                &signed_review(
                    &human,
                    &shown,
                    true,
                    ekr_core::EventId::mint(),
                    b"Approve exact proposal.",
                ),
                Timestamp::from_millis(3),
            )
            .unwrap();
        let wrong_digest =
            w::EkrKernelContentHash(ContentHash::of_bytes(b"different proposal").to_string());
        let events = store.published_events().unwrap();
        assert!(store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approval.review_id,
                &wrong_digest,
                Timestamp::from_millis(4)
            )
            .is_err());
        assert_eq!(store.published_events().unwrap(), events);
        let shown = store.schema_proposal(&shown.proposal.proposal_id).unwrap();
        store
            .reject_schema_proposal(
                &signed_review(
                    &human,
                    &shown,
                    false,
                    ekr_core::EventId::mint(),
                    b"Reject further application.",
                ),
                Timestamp::from_millis(4),
            )
            .unwrap();
        let events = store.published_events().unwrap();
        assert!(store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approval.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(5)
            )
            .is_err());
        assert_eq!(store.published_events().unwrap(), events);
    }
}

#[test]
fn schema_application_recovers_stale_attempts_and_a_missing_completion_receipt() {
    use ekr_store::ApplicationRetention;
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let (store, human) = upgraded(&source, sqlite);
        let (shown, approval) = approved_schema(&store, &human);
        drop(store);
        let snapshots: Vec<_> = (0..5)
            .map(|i| directory.path().join(format!("prefix-{i}")))
            .collect();
        for path in &snapshots {
            copy_closed_provider(&source, path);
        }
        let store = runtime(&source, sqlite)
            .with_review_authority(human.binding.clone())
            .unwrap();
        store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approval.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(4),
            )
            .unwrap();
        let (_, complete) = captured_application_history(&source, sqlite, &human);
        let election = complete.applications.elections()[0].clone();
        let step = complete.applications.steps()[0].clone();
        let attempt = complete.applications.attempts()[0].clone();
        let original_id: TransactionId = attempt.transaction_id.0.parse().unwrap();
        let document = store.transactions().unwrap()[&original_id]
            .proposal
            .document_bytes
            .clone();
        for (boundary, path) in snapshots.iter().enumerate() {
            let (authority, _) = captured_application_history(path, sqlite, &human);
            let retained: Box<dyn ApplicationRetention> = if sqlite {
                Box::new(
                    ekr_store::SqliteStore::sqlite(path, "upgrade-fixture", None)
                        .unwrap()
                        .under(authority),
                )
            } else {
                Box::new(
                    ekr_store::FileStore::file(path, "upgrade-fixture", None)
                        .unwrap()
                        .under(authority),
                )
            };
            retained
                .elect_application(
                    &w::EkrStoreApplicationElectionRetention {
                        election: election.clone(),
                        objects: w::EkrStoreApplicationElectionRetentionObjects {
                            ess_extra: Default::default(),
                        },
                    },
                    Timestamp::from_millis(4),
                )
                .unwrap();
            retained
                .elect_application_step(
                    &w::EkrStoreApplicationStepRetention {
                        step: step.clone(),
                        objects: w::EkrStoreApplicationStepRetentionObjects {
                            ess_extra: Default::default(),
                        },
                    },
                    Timestamp::from_millis(4),
                )
                .unwrap();
            retained
                .elect_application_attempt(
                    &w::EkrStoreApplicationAttemptRetention {
                        attempt: attempt.clone(),
                    },
                    Timestamp::from_millis(4),
                )
                .unwrap();
            drop(retained);
            let store = runtime(path, sqlite)
                .with_review_authority(human.binding.clone())
                .unwrap();
            store
                .propose(&document, context().operator, || Timestamp::from_millis(5))
                .unwrap();
            let revision = store.read(None).unwrap().root.revision;
            let store = if boundary >= 3 {
                leave_validation_prepared(
                    store,
                    path,
                    &directory
                        .path()
                        .join(format!("validation-probe-{boundary}")),
                    sqlite,
                    &human,
                    original_id,
                    revision,
                )
            } else {
                assert!(matches!(
                    store
                        .validate(original_id, revision, || Timestamp::from_millis(5))
                        .unwrap(),
                    ValidationCommandResult::Validated(_)
                ));
                store
            };
            if boundary < 2 || boundary == 3 {
                let (other, document) = proposal(&store.read(None).unwrap().seed_input);
                store
                    .propose(&document, context().operator, || Timestamp::from_millis(6))
                    .unwrap();
                assert!(matches!(
                    store
                        .validate(other, revision, || Timestamp::from_millis(6))
                        .unwrap(),
                    ValidationCommandResult::Validated(_)
                ));
                assert!(matches!(
                    store
                        .commit(other, context().operator, || Timestamp::from_millis(6))
                        .unwrap(),
                    CommitCommandResult::Committed(_)
                ));
            }
            if boundary == 1 {
                assert!(matches!(
                    store
                        .commit(original_id, context().operator, || Timestamp::from_millis(
                            7
                        ))
                        .unwrap(),
                    CommitCommandResult::Stale(_)
                ));
            } else if boundary == 2 {
                assert!(matches!(
                    store
                        .commit(original_id, context().operator, || Timestamp::from_millis(
                            7
                        ))
                        .unwrap(),
                    CommitCommandResult::Committed(_)
                ));
            }
            let before = store.read(None).unwrap().root;
            drop(store);
            let mut reopened = runtime(path, sqlite)
                .with_review_authority(human.binding.clone())
                .unwrap();
            reopened.set_full_replay(true);
            if boundary == 1 {
                let events = reopened.published_events().unwrap();
                assert!(reopened
                    .apply_schema_proposal(
                        &shown.proposal.proposal_id,
                        &approval.review_id,
                        &shown.proposal_digest,
                        Timestamp::from_millis(6),
                    )
                    .is_err());
                assert_eq!(
                    reopened.published_events().unwrap(),
                    events,
                    "premature successor retained state before its terminal predecessor"
                );
            }
            let report = reopened
                .apply_schema_proposal(
                    &shown.proposal.proposal_id,
                    &approval.review_id,
                    &shown.proposal_digest,
                    Timestamp::from_millis(8),
                )
                .unwrap();
            assert_eq!(*report.progress, w::EkrIntegrateApplicationProgress::V0);
            assert_eq!(report.application_id, election.application_id);
            assert_eq!(
                reopened.read(None).unwrap().root.revision.get(),
                before.revision.get() + u64::from(boundary != 2)
            );
            let (_, captured) = captured_application_history(path, sqlite, &human);
            assert_eq!(captured.applications.steps().len(), 1);
            assert_eq!(captured.applications.receipts().len(), 1);
            if boundary < 2 || boundary == 3 {
                assert_ne!(report.schema_transaction, attempt.transaction_id);
                assert_eq!(
                    reopened.transactions().unwrap()[&original_id].state(),
                    TransactionState::Stale
                );
                let attempts = captured.applications.attempts();
                assert_eq!(attempts.len(), 2);
                let mut successor = *attempts[1].transaction.clone();
                successor.id = attempt.transaction_id.clone();
                assert_eq!(successor, *attempt.transaction);
            } else {
                assert_eq!(report.schema_transaction, attempt.transaction_id);
                assert_eq!(captured.applications.attempts().len(), 1);
            }
            let events = reopened.published_events().unwrap();
            assert!(
                reopened
                    .apply_schema_proposal(
                        &shown.proposal.proposal_id,
                        &approval.review_id,
                        &shown.proposal_digest,
                        Timestamp::from_millis(9)
                    )
                    .unwrap()
                    .already_complete
            );
            assert_eq!(reopened.published_events().unwrap(), events);
        }
    }
}

// Construct the exact pre-publication boundary using a genuine validated decision from a
// closed snapshot. The provider elects its immutable preparation, but never resumes it.
fn leave_validation_prepared(
    store: Runtime,
    path: &std::path::Path,
    probe: &std::path::Path,
    sqlite: bool,
    human: &Human,
    transaction: TransactionId,
    against: RevisionNumber,
) -> Runtime {
    drop(store);
    copy_closed_provider(path, probe);
    let source = runtime(probe, sqlite)
        .with_review_authority(human.binding.clone())
        .unwrap();
    assert!(matches!(
        source
            .validate(transaction, against, || Timestamp::from_millis(5))
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    drop(source);
    let (authority, history) = captured_application_history(probe, sqlite, human);
    let transition = history
        .occurrences
        .iter()
        .rev()
        .find(|o| {
            matches!(
                o.event.payload,
                ekr_graph::RevisionPayload::AuthorityUpgraded { .. }
            )
        })
        .unwrap();
    let key = ekr_store::PublicationCommandKey {
        kind: ekr_store::PublicationCommandKind::Validate,
        transaction_id: Some(transaction),
        answer_id: None,
        predecessor_event_id: Some(transition.event.event_id),
        predecessor_record_hash: Some(transition.event.record_hash),
    };
    let source: Box<dyn RevisionLog> = if sqlite {
        Box::new(
            ekr_store::SqliteStore::sqlite(probe, "upgrade-fixture", None)
                .unwrap()
                .under(authority),
        )
    } else {
        Box::new(
            ekr_store::FileStore::file(probe, "upgrade-fixture", None)
                .unwrap()
                .under(authority),
        )
    };
    let prepared = source.preparation(&key).unwrap().unwrap();
    drop(source);
    let (authority, _) = captured_application_history(path, sqlite, human);
    let target: Box<dyn RevisionLog> = if sqlite {
        Box::new(
            ekr_store::SqliteStore::sqlite(path, "upgrade-fixture", None)
                .unwrap()
                .under(authority),
        )
    } else {
        Box::new(
            ekr_store::FileStore::file(path, "upgrade-fixture", None)
                .unwrap()
                .under(authority),
        )
    };
    assert_eq!(
        target
            .prepare(&key, prepared.input_hash, &prepared.decision, None)
            .unwrap(),
        prepared
    );
    drop(target);
    let store = runtime(path, sqlite)
        .with_review_authority(human.binding.clone())
        .unwrap();
    assert_eq!(
        store.transactions().unwrap()[&transaction].state(),
        TransactionState::Proposed
    );
    store
}

fn application_observation() -> w::EkrObserveObservationImport {
    let payload = b"Synthetic retained schema vocabulary evidence.";
    let hash = ContentHash::of_bytes(payload);
    let key = ekr_core::ObservationIdempotencyKey {
        source: "manual".into(),
        source_native_id: Some("schema-support".into()),
        content_hash: hash,
    };
    serde_json::from_value(serde_json::json!({
        "observation":{"observation_id":key.observation_id(),"source":key.source,"source_native_id":key.source_native_id,
            "content_hash":hash,"captured_at":"1970-01-01T00:00:00.002Z","kind":"FeedItem"},
        "key":{"source":key.source,"source_native_id":key.source_native_id,"content_hash":hash},"payload":ekr_core::bytes::encode(payload)
    })).unwrap()
}

#[test]
fn approved_observation_support_requires_knowledge_three() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (mut store, human) = upgraded(&path, sqlite);
        let source = application_observation();
        store
            .import_observation(&source, Timestamp::from_millis(2))
            .unwrap();
        let mut proposal = proposal_input(&store);
        proposal.proposal.observations = vec![source.observation.observation_id.clone()];
        proposal.proposal.evidence.clear();
        proposal.payload =
            ekr_core::bytes::encode(&serde_json::to_vec(&proposal.proposal).unwrap());
        let shown = store
            .submit_schema_proposal(&proposal, Timestamp::from_millis(3))
            .unwrap();
        let approved = store
            .approve_schema_proposal(
                &signed_review(
                    &human,
                    &shown,
                    true,
                    ekr_core::EventId::mint(),
                    b"Approve the exact retained observation support.",
                ),
                Timestamp::from_millis(4),
            )
            .unwrap();
        let report = store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approved.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(5),
            )
            .expect("exact reviewed Observation support must commit");
        let read = store.read(None).unwrap();
        let sources: Vec<_> = read
            .graph
            .evidence
            .values()
            .filter(|e| matches!(e.source, EvidenceSource::Observation(_)))
            .collect();
        assert_eq!(sources.len(), 1);
        let evidence = sources[0].clone();
        assert_eq!(
            evidence.source,
            EvidenceSource::Observation(source.observation.observation_id.0.parse().unwrap())
        );
        assert_eq!(
            evidence.content_hash.to_string(),
            source.observation.content_hash.0
        );
        assert_eq!(evidence.extracted_by, context().operator);
        assert_eq!(evidence.observed_at, Timestamp::from_millis(2));
        assert_eq!(evidence.confidence.basis_points(), 0);
        assert_eq!(
            read.content(&evidence.content_hash).unwrap(),
            ekr_core::bytes::decode(&source.payload).unwrap()
        );
        let schema_history = store.schema_history(read.root.revision).unwrap();
        assert!(schema_history.supporting_evidence[&read.root.revision].contains(&evidence.id));
        let before = store.published_events().unwrap();
        store.set_full_replay(true);
        assert_eq!(store.read(None).unwrap().root, read.root);
        assert!(
            store
                .apply_schema_proposal(
                    &shown.proposal.proposal_id,
                    &approved.review_id,
                    &shown.proposal_digest,
                    Timestamp::from_millis(6)
                )
                .unwrap()
                .already_complete
        );
        assert_eq!(store.published_events().unwrap(), before);
        assert!(!report.already_complete);
        // Merely running under knowledge/3 with a retained observation grants no ordinary authority.
        let mut forged = evidence.clone();
        forged.id = EvidenceId::mint();
        let unsigned = GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            schema_version: None,
            evidence: BTreeSet::new(),
            operations: vec![GraphOperation::AddEvidence(Box::new(
                ekr_kernel::EvidenceAddition {
                    evidence: forged,
                    payload: ekr_core::bytes::decode(&source.payload).unwrap(),
                },
            ))],
        };
        store
            .propose(&encode(&unsigned), context().operator, || {
                Timestamp::from_millis(7)
            })
            .unwrap();
        let result = store
            .validate(unsigned.id, read.root.revision, || {
                Timestamp::from_millis(8)
            })
            .unwrap();
        assert!(
            matches!(result, ValidationCommandResult::Rejected(_)),
            "unsigned observation was admitted"
        );
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding)
            .unwrap();
        reopened.set_full_replay(true);
        assert_eq!(reopened.read(None).unwrap().root, read.root);
    }
}

fn application_interpretation(
    source: &w::EkrObserveObservationImport,
    observation: bool,
    actor: ekr_core::AgentId,
) -> w::EkrIntegrateInterpretationImport {
    use ekr_core::Canonical;
    let reference = serde_json::json!({"node_type":"Project","aliases":["Maple"]});
    let records: Vec<_> = [7500, 1200].into_iter().map(|confidence| serde_json::json!({
        "evidence":{"id":EvidenceId::mint(),"source":if observation { serde_json::json!({"kind":"Observation","observation":source.observation.observation_id}) } else { serde_json::json!({"kind":"HumanStatement"}) },
            "content_hash":source.observation.content_hash,"observed_at":"1970-01-01T00:00:00.002Z","extracted_by":actor,"confidence_bp":confidence},"payload":source.payload
    })).collect();
    let facts: Vec<_> = records.iter().enumerate().map(|(i,e)| serde_json::json!({"kind":"Property","value":{
        "subject":reference,"property":"health","value":{"kind":"String","canonical_bytes":ekr_core::bytes::encode(&ekr_graph::CanonicalValue::String(format!("health-{i}")).canonical_bytes())},"evidence":[e["evidence"]["id"]]
    }})).collect();
    let document = serde_json::json!({
        "version":{"interpretation_id":ekr_core::NodeId::mint(),"version":1},"root_id":ekr_core::GraphRootId::mint(),
        "observations":[source.observation.observation_id],
        "local_schema":{"node_types":[{"name":"Project","parents":[],"abstract_type":false,"properties":[{"name":"health","value":{"value_kind":"String"},"required":false,"cardinality":"One"}]}],"edge_types":[]},
        "entities":[reference],"facts":facts,"evidence":records
    });
    serde_json::from_value(serde_json::json!({"payload":ekr_core::bytes::encode(&serde_json::to_vec_pretty(&document).unwrap()),"document":document})).unwrap()
}

fn supported_assertion(store: &Runtime, evidence: &Evidence) -> AssertionId {
    let mut claim = store
        .read(None)
        .unwrap()
        .seed_input
        .graph
        .assertions
        .values()
        .next()
        .unwrap()
        .clone();
    claim.id = AssertionId::mint();
    claim.assessment = Assessment::Proposed;
    claim.lifecycle = AssertionLifecycle::Active;
    claim.proposed_by = context().operator;
    claim.transaction_time = TransactionTime::since(Timestamp::from_millis(10));
    claim.evidence = BTreeSet::from([evidence.id]);
    let id = claim.id;
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        schema_version: None,
        evidence: claim.evidence.clone(),
        operations: vec![GraphOperation::AddAssertion(Box::new(claim))],
    };
    store
        .propose(&encode(&tx), context().operator, || {
            Timestamp::from_millis(10)
        })
        .unwrap();
    assert!(matches!(
        store
            .validate(tx.id, store.head().unwrap().unwrap().revision, || {
                Timestamp::from_millis(11)
            })
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    store
        .commit(tx.id, context().operator, || Timestamp::from_millis(12))
        .unwrap();
    let explanation = store.read(None).unwrap().explain(id).unwrap();
    assert!(explanation.links.iter().any(
        |link| matches!(link, ekr_kernel::ExplanationLink::Evidence(actual) if actual == evidence)
    ));
    id
}

#[test]
fn application_wrappers_preserve_source_and_attribution() {
    for sqlite in [false, true] {
        for mode in 0..3 {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("store");
            let (mut store, human) = upgraded(&path, sqlite);
            let observation = application_observation();
            store
                .import_observation(&observation, Timestamp::from_millis(2))
                .unwrap();
            let original = application_interpretation(
                &observation,
                mode == 0,
                if mode == 0 {
                    context().validator
                } else {
                    context().operator
                },
            );
            let imported = store
                .import_interpretation(&original, Timestamp::from_millis(2))
                .unwrap();
            let mut proposal = proposal_input(&store);
            proposal.proposal.evidence.clear();
            proposal.proposal.sources = vec![Box::new(
                serde_json::from_value(
                    serde_json::json!({"version":imported.version,"items":["facts[0]"]}),
                )
                .unwrap(),
            )];
            proposal.payload =
                ekr_core::bytes::encode(&serde_json::to_vec(&proposal.proposal).unwrap());
            let shown = store
                .submit_schema_proposal(&proposal, Timestamp::from_millis(3))
                .unwrap();
            let approved = store
                .approve_schema_proposal(
                    &signed_review(
                        &human,
                        &shown,
                        true,
                        ekr_core::EventId::mint(),
                        b"Approve only the selected retained source fact.",
                    ),
                    Timestamp::from_millis(4),
                )
                .unwrap();
            let source_id: EvidenceId =
                original.document.evidence[0].evidence.id.0.parse().unwrap();
            if mode != 0 {
                let tx = GraphTransaction {
                    id: TransactionId::mint(),
                    proposer: context().operator,
                    schema_version: None,
                    evidence: BTreeSet::new(),
                    operations: vec![GraphOperation::AddEvidence(Box::new(
                        ekr_kernel::EvidenceAddition {
                            evidence: Evidence {
                                id: source_id,
                                source: EvidenceSource::HumanStatement { identity: None },
                                content_hash: ContentHash::of_bytes(
                                    &ekr_core::bytes::decode(&observation.payload).unwrap(),
                                ),
                                extracted_by: context().operator,
                                observed_at: Timestamp::from_millis(2),
                                confidence: Confidence::from_basis_points(if mode == 1 {
                                    7500
                                } else {
                                    7400
                                })
                                .unwrap(),
                            },
                            payload: ekr_core::bytes::decode(&observation.payload).unwrap(),
                        },
                    ))],
                };
                store
                    .propose(&encode(&tx), context().operator, || {
                        Timestamp::from_millis(5)
                    })
                    .unwrap();
                assert!(matches!(
                    store
                        .validate(tx.id, store.head().unwrap().unwrap().revision, || {
                            Timestamp::from_millis(6)
                        })
                        .unwrap(),
                    ValidationCommandResult::Validated(_)
                ));
                store
                    .commit(tx.id, context().operator, || Timestamp::from_millis(7))
                    .unwrap();
            }
            let before = store.published_events().unwrap();
            let result = store.apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approved.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(8),
            );
            if mode == 2 {
                assert!(result
                    .unwrap_err()
                    .to_string()
                    .contains("canonical and source evidence identity disagree"));
                assert_eq!(store.published_events().unwrap(), before);
                assert!(matches!(
                    store
                        .schema_proposal(&shown.proposal.proposal_id)
                        .unwrap()
                        .application,
                    w::EssPresence::Absent
                ));
                store.set_full_replay(true);
                store.read(None).unwrap();
                continue;
            }
            result.expect("same source evidence arriving after approval must not change the elected wrapper plan");
            let read = store.read(None).unwrap();
            let supports = &store
                .schema_history(read.root.revision)
                .unwrap()
                .supporting_evidence[&read.root.revision];
            assert_eq!(
                supports.len(),
                2,
                "only one selected fact plus the human statement support the schema"
            );
            let wrapper = read
                .graph
                .evidence
                .values()
                .find(|e| supports.contains(&e.id) && e.confidence.basis_points() == 7500)
                .unwrap()
                .clone();
            assert_ne!(wrapper.id, source_id);
            assert_eq!(wrapper.extracted_by, context().operator);
            assert_eq!(wrapper.observed_at, Timestamp::from_millis(2));
            assert_eq!(
                store.interpretation(&imported.version).unwrap().document,
                original.document
            );
            let claim = supported_assertion(&store, &wrapper);
            let head = store.head().unwrap();
            drop(store);
            let mut reopened = runtime(&path, sqlite)
                .with_review_authority(human.binding)
                .unwrap();
            reopened.set_full_replay(true);
            assert_eq!(reopened.head().unwrap(), head);
            reopened.read(None).unwrap().explain(claim).unwrap();
        }
    }
}

#[test]
fn application_observation_explanation_survives_source_root_removal() {
    use ekr_store::CommitAuthority;
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (store, human) = upgraded(&path, sqlite);
        let observation = application_observation();
        store
            .import_observation(&observation, Timestamp::from_millis(2))
            .unwrap();
        let source = application_interpretation(&observation, true, context().validator);
        let imported = store
            .import_interpretation(&source, Timestamp::from_millis(2))
            .unwrap();
        let mut proposal = proposal_input(&store);
        proposal.proposal.evidence.clear();
        proposal.proposal.sources = vec![Box::new(
            serde_json::from_value(
                serde_json::json!({"version":imported.version,"items":["facts[0]"]}),
            )
            .unwrap(),
        )];
        proposal.payload =
            ekr_core::bytes::encode(&serde_json::to_vec(&proposal.proposal).unwrap());
        let shown = store
            .submit_schema_proposal(&proposal, Timestamp::from_millis(3))
            .unwrap();
        let approved = store
            .approve_schema_proposal(
                &signed_review(
                    &human,
                    &shown,
                    true,
                    ekr_core::EventId::mint(),
                    b"Retain the selected source independently.",
                ),
                Timestamp::from_millis(4),
            )
            .unwrap();
        store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approved.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(5),
            )
            .unwrap();
        let wrapper = store
            .read(None)
            .unwrap()
            .graph
            .evidence
            .values()
            .find(|e| matches!(e.source, EvidenceSource::Observation(_)))
            .unwrap()
            .clone();
        let claim = supported_assertion(&store, &wrapper);
        let captured = store.read(None).unwrap();
        let expected = captured.explain(claim).unwrap();
        let (authority, history) = captured_application_history(&path, sqlite, &human);
        authority.verify(&history, None, None).unwrap();
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding.clone())
            .unwrap();
        reopened.set_full_replay(true);
        let mut cold_capture = reopened.read(None).unwrap();
        assert_eq!(cold_capture.explain(claim).unwrap(), expected);
        let (cold_authority, cold_history) = captured_application_history(&path, sqlite, &human);
        drop(reopened);
        // Delete only this test's temporary provider. The separately retained capture carries
        // immutable evidence, not a live incubation API; no product GC behavior is claimed.
        directory.close().unwrap();
        assert!(!path.exists());
        for (authority, history) in [(&authority, &history), (&cold_authority, &cold_history)] {
            authority.verify(history, None, None).unwrap();
            for hash in [
                wrapper.content_hash,
                imported.version.document_digest.0.parse().unwrap(),
            ] {
                let mut missing = history.clone();
                assert!(missing.objects.remove(&hash).is_some());
                assert!(
                    authority.verify(&missing, None, None).is_err(),
                    "missing retained support admitted"
                );
                let mut changed = history.clone();
                let object = changed.objects.get_mut(&hash).unwrap();
                std::sync::Arc::make_mut(&mut object.bytes)[0] ^= 1;
                assert!(
                    authority.verify(&changed, None, None).is_err(),
                    "changed retained support admitted after a warm replay"
                );
                authority.verify(history, None, None).unwrap();
            }
        }
        assert_eq!(captured.explain(claim).unwrap(), expected);
        assert_eq!(cold_capture.explain(claim).unwrap(), expected);
        assert_eq!(
            captured.content(&wrapper.content_hash).unwrap(),
            ekr_core::bytes::decode(&observation.payload).unwrap()
        );
        std::sync::Arc::make_mut(&mut cold_capture.graph)
            .evidence
            .get_mut(&wrapper.id)
            .unwrap()
            .source = EvidenceSource::Observation(ekr_core::ObservationId::mint());
        assert!(
            cold_capture.explain(claim).is_err(),
            "caller-owned source enum granted explanation authority"
        );
    }
}
