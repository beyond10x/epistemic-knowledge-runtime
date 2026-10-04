//! A schema approval authorizes its own final ordinary correction, never an attention answer.
#![allow(unused_imports, dead_code)]
include!("support/schema_proposal_fixture.rs");
include!("support/schema_application_history.rs");

fn prepared(
    path: &std::path::Path,
    sqlite: bool,
    correction: &str,
) -> (
    Runtime,
    Human,
    w::EkrIntegrateSchemaProposalRead,
    w::EkrIntegrateProposalReviewSnapshot,
) {
    let store = runtime(path, sqlite);
    let mut initial = seed();
    if correction == "Mixed" {
        let original = initial.graph.assertions.values().next().unwrap().subject;
        let Subject::Node(original) = original else {
            panic!("node subject")
        };
        let mut node = initial.graph.nodes[&original].clone();
        node.id = NodeId::mint();
        node.canonical_name = "Second correction project".into();
        let claims = initial
            .graph
            .assertions
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for mut claim in claims {
            claim.id = AssertionId::mint();
            claim.subject = Subject::Node(node.id);
            initial.graph.assertions.insert(claim.id, claim);
        }
        initial.graph.nodes.insert(node.id, node);
    }
    if correction == "CorrectTime" || correction == "Mixed" {
        for claim in initial.graph.assertions.values_mut() {
            claim.valid_time.to = Some(Timestamp::from_millis(946684800000));
        }
    }
    let seeded = store.seed(initial, || Timestamp::EPOCH).unwrap();
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
    let read = store.read(None).unwrap();
    let questions = read.dispute_attention().unwrap();
    let mut input = proposal_input(&store);
    input.proposal.corrections = questions.iter().enumerate().map(|(i, question)| {
        let kind = if correction == "Mixed" { if i == 0 { "Choose" } else { "CorrectTime" } } else { correction };
        let mut value = serde_json::json!({"kind":kind,"assertion_id":question.claims[0],"reason":"Human-reviewed interpretation of the retained ownership evidence."});
        if kind == "CorrectTime" { value["valid_from"] = "2000-01-01T00:00:00Z".into(); }
        Box::new(serde_json::from_value(value).unwrap())
    }).collect();
    input.payload = ekr_core::bytes::encode(&serde_json::to_vec(&input.proposal).unwrap());
    let shown = store
        .submit_schema_proposal(&input, Timestamp::from_millis(2))
        .unwrap();
    let approved = store
        .approve_schema_proposal(
            &signed_review(
                &human,
                &shown,
                true,
                ekr_core::EventId::mint(),
                b"Approve the vocabulary and this exact ownership correction.",
            ),
            Timestamp::from_millis(3),
        )
        .unwrap();
    (store, human, shown, approved)
}

#[test]
fn temporal_and_multi_dispute_corrections_preserve_both_versions_and_replay() {
    use ekr_store::CommitAuthority;
    for sqlite in [false, true] {
        for kind in ["CorrectTime", "Mixed"] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("store");
            let (store, human, shown, approved) = prepared(&path, sqlite, kind);
            let before = store.read(None).unwrap();
            let report = store
                .apply_schema_proposal(
                    &shown.proposal.proposal_id,
                    &approved.review_id,
                    &shown.proposal_digest,
                    Timestamp::from_millis(4),
                )
                .unwrap();
            assert_eq!(
                *report.progress,
                w::EkrIntegrateApplicationProgress::V0,
                "{report:?}"
            );
            let read = store.read(None).unwrap();
            assert_eq!(read.root.revision.get(), before.root.revision.get() + 2);
            assert!(read.dispute_attention().unwrap().is_empty());
            assert_eq!(
                read.graph.assertions.len(),
                before.graph.assertions.len() + 1
            );
            let (_, history) = captured_application_history(&path, sqlite, &human);
            let step = history
                .applications
                .steps()
                .iter()
                .find(|s| matches!(*s.step.kind, w::EkrIntegrateApplicationStepKind::V0))
                .unwrap();
            assert_eq!(step.replacements.len(), 1);
            let previous: AssertionId = step.replacements[0].previous.0.parse().unwrap();
            let replacement: AssertionId = step.replacements[0].replacement.0.parse().unwrap();
            let old = &read.graph.assertions[&previous];
            let new = &read.graph.assertions[&replacement];
            assert_eq!(old.object, new.object);
            assert!(old.evidence.is_subset(&new.evidence));
            assert_eq!(
                new.valid_time.from,
                Some(Timestamp::from_millis(946684800000))
            );
            assert_eq!(new.valid_time.to, None);
            assert!(matches!(
                old.lifecycle,
                AssertionLifecycle::Retracted { .. }
            ));
            assert!(matches!(new.assessment, Assessment::Accepted { .. }));
            for id in [previous, replacement] {
                let explained = read.explain(id).unwrap();
                for member in [previous, replacement] {
                    assert!(explained.links.iter().any(|link| matches!(link, ekr_kernel::ExplanationLink::Assertion(a) if a.id == member)));
                }
                assert!(explained
                    .links
                    .iter()
                    .any(|link| matches!(link, ekr_kernel::ExplanationLink::SchemaCorrection(_))));
            }
            let (authority, history) = captured_application_history(&path, sqlite, &human);
            let addresses = match &*history.applications.coordination()[0].entries[0].record {
                w::EkrIntegrateProposalCoordinationRecord::V1(review) => [
                    &review.value.decision.proof_object_hash.0,
                    &review.value.decision.policy_object_hash.0,
                    &review.value.decision.statement_object_hash.0,
                ],
                _ => panic!("review first"),
            };
            authority.verify(&history, None, None).unwrap();
            for address in addresses {
                let hash = address.parse().unwrap();
                let mut missing = history.clone();
                assert!(missing.objects.remove(&hash).is_some());
                assert!(authority.verify(&missing, None, None).is_err());
                let mut changed = history.clone();
                std::sync::Arc::make_mut(&mut changed.objects.get_mut(&hash).unwrap().bytes)[0] ^=
                    1;
                assert!(authority.verify(&changed, None, None).is_err());
                let (cold, _) = captured_application_history(&path, sqlite, &human);
                assert!(cold.verify(&missing, None, None).is_err());
                assert!(cold.verify(&changed, None, None).is_err());
            }
            drop(store);
            let mut reopened = runtime(&path, sqlite)
                .with_review_authority(human.binding)
                .unwrap();
            reopened.set_full_replay(true);
            assert_eq!(
                reopened.read(None).unwrap().explain(replacement).unwrap(),
                read.explain(replacement).unwrap()
            );
        }
    }
}

#[test]
fn application_corrections_explain_the_real_schema_review() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (store, human, shown, approved) = prepared(&path, sqlite, "Choose");
        let before = store.read(None).unwrap();
        let chosen: AssertionId = shown.proposal.corrections[0]
            .assertion_id
            .0
            .parse()
            .unwrap();
        let report = store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approved.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(4),
            )
            .unwrap();
        assert_eq!(*report.progress, w::EkrIntegrateApplicationProgress::V0);
        assert!(!report.corrections_pending);
        let read = store.read(None).unwrap();
        assert_eq!(
            read.root.revision.get(),
            before.root.revision.get() + 2,
            "schema and all corrections are separate ordinary commits"
        );
        assert!(read.dispute_attention().unwrap().is_empty());
        assert!(matches!(
            read.graph.assertions[&chosen].assessment,
            Assessment::Accepted { .. }
        ));
        assert_eq!(read.graph.assertions.len(), before.graph.assertions.len());
        assert!(
            store.answer_history(None).unwrap().is_empty(),
            "schema review is not an AttentionAnswer"
        );
        let explained = serde_json::to_value(read.explain(chosen).unwrap()).unwrap();
        let links = explained["links"].as_array().unwrap();
        assert!(links.iter().any(|link| link["kind"] == "SchemaCorrection"));
        assert!(!links.iter().any(|link| link["kind"] == "HumanAnswer"));
        let retained = store.schema_proposal(&shown.proposal.proposal_id).unwrap();
        let w::EssPresence::Present(application) = retained.application else {
            panic!("application missing")
        };
        assert_eq!(application.steps.len(), 2);
        assert_eq!(
            application
                .steps
                .iter()
                .filter(|step| matches!(*step.step.kind, w::EkrIntegrateApplicationStepKind::V0))
                .count(),
            1
        );
        let events = store.published_events().unwrap();
        assert!(
            store
                .apply_schema_proposal(
                    &shown.proposal.proposal_id,
                    &approved.review_id,
                    &shown.proposal_digest,
                    Timestamp::from_millis(5)
                )
                .unwrap()
                .already_complete
        );
        assert_eq!(store.published_events().unwrap(), events);
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding)
            .unwrap();
        reopened.set_full_replay(true);
        let cold = reopened.read(None).unwrap();
        assert_eq!(
            serde_json::to_value(cold.explain(chosen).unwrap()).unwrap(),
            explained
        );
        drop(reopened);
        std::fs::rename(&path, directory.path().join("detached-provider")).unwrap();
        assert!(!path.exists());
        assert_eq!(
            serde_json::to_value(cold.explain(chosen).unwrap()).unwrap(),
            explained
        );
        assert_eq!(
            serde_json::to_value(read.explain(chosen).unwrap()).unwrap(),
            explained
        );
    }
}

#[test]
fn unresolved_application_corrections_remain_pending() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (store, human, shown, approved) = prepared(&path, sqlite, "Unresolved");
        let before = store.read(None).unwrap();
        let report = store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approved.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(4),
            )
            .unwrap();
        assert_eq!(*report.progress, w::EkrIntegrateApplicationProgress::V3);
        assert!(report.corrections_pending);
        assert!(report.remaining_items.is_empty());
        let read = store.read(None).unwrap();
        assert_eq!(read.root.revision.get(), before.root.revision.get() + 1);
        assert_eq!(read.graph.assertions, before.graph.assertions);
        assert!(!read.dispute_attention().unwrap().is_empty());
        let w::EssPresence::Present(application) = store
            .schema_proposal(&shown.proposal.proposal_id)
            .unwrap()
            .application
        else {
            panic!("application missing")
        };
        assert_eq!(
            application.steps.len(),
            1,
            "uncertainty must not mint a correction step"
        );
        let events = store.published_events().unwrap();
        let retry = store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approved.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(5),
            )
            .unwrap();
        assert_eq!(retry.receipt_id, report.receipt_id);
        assert_eq!(store.published_events().unwrap(), events);
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding)
            .unwrap();
        reopened.set_full_replay(true);
        assert_eq!(
            reopened.read(None).unwrap().graph.assertions,
            before.graph.assertions
        );
    }
}

fn retain_final_step_prefix(
    path: &std::path::Path,
    sqlite: bool,
    human: &Human,
    history: &ekr_store::RetainedHistory,
    transactions: &BTreeMap<TransactionId, ekr_kernel::TransactionRecord>,
) -> Box<w::EkrIntegrateRetainedApplicationStep> {
    let retained = application_retention(path, sqlite, human);
    retained
        .elect_application(
            &w::EkrStoreApplicationElectionRetention {
                election: history.applications.elections()[0].clone(),
                objects: w::EkrStoreApplicationElectionRetentionObjects {
                    ess_extra: Default::default(),
                },
            },
            Timestamp::from_millis(4),
        )
        .unwrap();
    drop(retained);
    let store = runtime(path, sqlite)
        .with_review_authority(human.binding.clone())
        .unwrap();
    let mut steps = history.applications.steps().to_vec();
    steps.sort_by_key(|s| {
        transactions[&s.transaction.id.0.parse().unwrap()]
            .committed
            .as_ref()
            .unwrap()
            .result
            .revision
    });
    for step in steps {
        let retained = application_retention(path, sqlite, human);
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
        if matches!(*step.step.kind, w::EkrIntegrateApplicationStepKind::V0) {
            return step;
        }
        let attempt = history
            .applications
            .attempts()
            .iter()
            .find(|a| a.step_election_id == step.step_election_id)
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
        let id = attempt.transaction_id.0.parse().unwrap();
        commit_application_document(&store, id, &transactions[&id].proposal.document_bytes, 4);
    }
    panic!("missing final correction")
}

#[test]
fn renewed_approval_resumes_the_same_frozen_correction() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let target = directory.path().join("interrupted");
        let (store, human, shown, approved) = prepared(&source, sqlite, "Choose");
        drop(store);
        copy_closed_provider(&source, &target);
        let store = runtime(&source, sqlite)
            .with_review_authority(human.binding.clone())
            .unwrap();
        store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approved.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(4),
            )
            .unwrap();
        let (_, history) = captured_application_history(&source, sqlite, &human);
        let frozen = retain_final_step_prefix(
            &target,
            sqlite,
            &human,
            &history,
            &store.transactions().unwrap(),
        );
        let target_store = runtime(&target, sqlite)
            .with_review_authority(human.binding.clone())
            .unwrap();
        let current = target_store
            .schema_proposal(&shown.proposal.proposal_id)
            .unwrap();
        let renewed = target_store
            .approve_schema_proposal(
                &signed_review(
                    &human,
                    &current,
                    true,
                    ekr_core::EventId::mint(),
                    b"Renew approval of the exact remaining ownership correction.",
                ),
                Timestamp::from_millis(5),
            )
            .unwrap();
        let result = target_store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &renewed.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(6),
            )
            .unwrap();
        assert_eq!(
            *result.progress,
            w::EkrIntegrateApplicationProgress::V0,
            "{result:?}"
        );
        let (_, completed) = captured_application_history(&target, sqlite, &human);
        assert!(
            completed.applications.steps().contains(&frozen),
            "renewal changed frozen IDs or operations"
        );
        let id: AssertionId = shown.proposal.corrections[0]
            .assertion_id
            .0
            .parse()
            .unwrap();
        let explanation = target_store.read(None).unwrap().explain(id).unwrap();
        let link = explanation
            .links
            .iter()
            .find_map(|l| match l {
                ekr_kernel::ExplanationLink::SchemaCorrection(c) => Some(c),
                _ => None,
            })
            .unwrap();
        assert_eq!(link.review.review.review_id, renewed.review_id);
        assert_eq!(link.original_review.review.review_id, approved.review_id);
        drop(target_store);
        let mut reopened = runtime(&target, sqlite)
            .with_review_authority(human.binding)
            .unwrap();
        reopened.set_full_replay(true);
        assert_eq!(
            reopened.read(None).unwrap().explain(id).unwrap(),
            explanation
        );
    }
}

#[test]
fn correction_recovery_preserves_allocations_and_requires_current_approval() {
    use ekr_store::ApplicationRetention;
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source");
        let (store, human, shown, approved) = prepared(&source, sqlite, "Mixed");
        drop(store);
        let prefixes = (0..4)
            .map(|i| directory.path().join(format!("prefix-{i}")))
            .collect::<Vec<_>>();
        for prefix in &prefixes {
            copy_closed_provider(&source, prefix);
        }
        let store = runtime(&source, sqlite)
            .with_review_authority(human.binding.clone())
            .unwrap();
        store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approved.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(4),
            )
            .unwrap();
        let (_, history) = captured_application_history(&source, sqlite, &human);
        let transactions = store.transactions().unwrap();
        for (boundary, path) in prefixes.iter().enumerate() {
            let step = retain_final_step_prefix(path, sqlite, &human, &history, &transactions);
            let attempt = history
                .applications
                .attempts()
                .iter()
                .find(|a| a.step_election_id == step.step_election_id)
                .unwrap();
            let id: TransactionId = attempt.transaction_id.0.parse().unwrap();
            let store = runtime(path, sqlite)
                .with_review_authority(human.binding.clone())
                .unwrap();
            let mut approval = approved.review_id.clone();
            if boundary == 3 {
                let mut unsigned = ekr_kernel::TransactionDocument::parse(
                    &transactions[&id].proposal.document_bytes,
                )
                .unwrap()
                .transaction()
                .clone();
                unsigned.id = TransactionId::mint();
                store
                    .propose(&encode(&unsigned), context().operator, || {
                        Timestamp::from_millis(5)
                    })
                    .unwrap();
                assert!(
                    matches!(
                        store
                            .validate(unsigned.id, store.read(None).unwrap().root.revision, || {
                                Timestamp::from_millis(5)
                            })
                            .unwrap(),
                        ValidationCommandResult::Rejected(_)
                    ),
                    "copied unsigned corrections received reviewed withdrawal authority"
                );
            }
            let retained = application_retention(path, sqlite, &human);
            retained
                .elect_application_attempt(
                    &w::EkrStoreApplicationAttemptRetention {
                        attempt: attempt.clone(),
                    },
                    Timestamp::from_millis(4),
                )
                .unwrap();
            drop(retained);
            store
                .propose(
                    &transactions[&id].proposal.document_bytes,
                    context().operator,
                    || Timestamp::from_millis(5),
                )
                .unwrap();
            let basis = store.read(None).unwrap().root.revision;
            assert!(matches!(
                store
                    .validate(id, basis, || Timestamp::from_millis(5))
                    .unwrap(),
                ValidationCommandResult::Validated(_)
            ));
            if boundary == 1 {
                let (other, document) = proposal(&store.read(None).unwrap().seed_input);
                commit_application_document(&store, other, &document, 6);
                assert!(matches!(
                    store
                        .commit(id, context().operator, || Timestamp::from_millis(7))
                        .unwrap(),
                    CommitCommandResult::Stale(_)
                ));
            } else if boundary == 2 {
                let current = store.schema_proposal(&shown.proposal.proposal_id).unwrap();
                store
                    .reject_schema_proposal(
                        &signed_review(
                            &human,
                            &current,
                            false,
                            ekr_core::EventId::mint(),
                            b"Pause the remaining correction.",
                        ),
                        Timestamp::from_millis(6),
                    )
                    .unwrap();
                let events = store.published_events().unwrap();
                assert!(store
                    .commit(id, context().operator, || Timestamp::from_millis(7))
                    .is_err());
                assert_eq!(store.published_events().unwrap(), events);
                assert_eq!(store.read(None).unwrap().root.revision, basis);
                let current = store.schema_proposal(&shown.proposal.proposal_id).unwrap();
                approval = store
                    .approve_schema_proposal(
                        &signed_review(
                            &human,
                            &current,
                            true,
                            ekr_core::EventId::mint(),
                            b"Renew the unchanged correction after the pause.",
                        ),
                        Timestamp::from_millis(8),
                    )
                    .unwrap()
                    .review_id;
            } else if boundary == 0 {
                assert!(matches!(
                    store
                        .commit(id, context().operator, || Timestamp::from_millis(6))
                        .unwrap(),
                    CommitCommandResult::Committed(_)
                ));
            }
            let before = store.published_events().unwrap();
            drop(store);
            let mut reopened = runtime(path, sqlite)
                .with_review_authority(human.binding.clone())
                .unwrap();
            reopened.set_full_replay(true);
            let result = reopened
                .apply_schema_proposal(
                    &shown.proposal.proposal_id,
                    &approval,
                    &shown.proposal_digest,
                    Timestamp::from_millis(9),
                )
                .unwrap();
            assert_eq!(
                *result.progress,
                w::EkrIntegrateApplicationProgress::V0,
                "{sqlite}/{boundary}: {result:?}"
            );
            if boundary == 0 {
                assert_eq!(
                    reopened
                        .published_events()
                        .unwrap()
                        .into_iter()
                        .filter(|e| e.stream_type == "ekr.revision")
                        .collect::<Vec<_>>(),
                    before
                        .into_iter()
                        .filter(|e| e.stream_type == "ekr.revision")
                        .collect::<Vec<_>>(),
                    "receipt recovery wrote canonical history"
                );
            }
            let (_, recovered) = captured_application_history(path, sqlite, &human);
            assert!(recovered.applications.steps().contains(&step));
            let final_read = reopened.read(None).unwrap();
            assert!(final_read.dispute_attention().unwrap().is_empty());
            for replacement in &step.replacements {
                let new: AssertionId = replacement.replacement.0.parse().unwrap();
                assert!(final_read.graph.assertions.contains_key(&new));
                final_read.explain(new).unwrap();
            }
            let events = reopened.published_events().unwrap();
            assert!(
                reopened
                    .apply_schema_proposal(
                        &shown.proposal.proposal_id,
                        &approval,
                        &shown.proposal_digest,
                        Timestamp::from_millis(10)
                    )
                    .unwrap()
                    .already_complete
            );
            assert_eq!(reopened.published_events().unwrap(), events);
        }
    }
}
