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
