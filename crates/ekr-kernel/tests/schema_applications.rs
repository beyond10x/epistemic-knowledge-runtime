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
