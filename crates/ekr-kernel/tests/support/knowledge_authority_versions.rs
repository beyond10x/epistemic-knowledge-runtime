//! Genuine signed /2 publications retain their bytes and rules across a reviewed /3 transition.
use crate as ekr_kernel;
include!("authority_review_fixture.rs");

fn exercise<
    S: RevisionLog + ObjectStore + Initialize + ObservationRetention + IncubationRetention,
>(
    open: impl Fn(Option<m::TrustedReviewHostBinding>, bool) -> Commit<S>,
) {
    let old = open(None, false);
    let seeded = old.seed(seed(), || Timestamp::EPOCH).unwrap();
    let human = Human::new(seeded.seed_hash);
    drop(old);
    let kernel = open(Some(human.binding.clone()), false);
    let profile = ValidationProfileV1::knowledge_evidence(context().validator);
    let (history, state) = kernel.replayed_state().unwrap();
    let preview = kernel
        .authority
        .preview_for(&history, &state, &human.policy, &profile)
        .unwrap();
    kernel
        .apply_upgrade_to(
            &preview,
            &human.policy,
            &human.proof(&preview),
            b"reviewed contradictions and pending validations",
            || Timestamp::from_millis(1),
            &profile,
        )
        .unwrap();
    assert_eq!(
        kernel
            .read(None)
            .unwrap()
            .authority_at(kernel.head().unwrap().unwrap().revision)
            .validation_profile,
        profile
    );
    let evidence = *kernel
        .read(None)
        .unwrap()
        .graph
        .evidence
        .keys()
        .next()
        .unwrap();
    let schema = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            TypeId::mint(),
            "LegacyEvidenceVocabulary",
        )))],
        evidence: BTreeSet::from([evidence]),
        schema_version: Some(ekr_core::SchemaVersionId::mint()),
    };
    kernel
        .propose(&encode(&schema), context().operator, || {
            Timestamp::from_millis(2)
        })
        .unwrap();
    assert!(matches!(
        kernel
            .validate(schema.id, kernel.head().unwrap().unwrap().revision, || {
                Timestamp::from_millis(3)
            })
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    kernel
        .commit(schema.id, context().operator, || Timestamp::from_millis(4))
        .unwrap();
    let schema_root = kernel.head().unwrap().unwrap();
    let mut claim = kernel
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
    claim.transaction_time = ekr_graph::TransactionTime::since(Timestamp::from_millis(5));
    let assertion_id = claim.id;
    let assertion = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        schema_version: None,
        evidence: claim.evidence.clone(),
        operations: vec![GraphOperation::AddAssertion(Box::new(claim))],
    };
    kernel
        .propose(&encode(&assertion), context().operator, || {
            Timestamp::from_millis(5)
        })
        .unwrap();
    assert!(matches!(
        kernel
            .validate(assertion.id, schema_root.revision, || {
                Timestamp::from_millis(6)
            })
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    kernel
        .commit(assertion.id, context().operator, || {
            Timestamp::from_millis(7)
        })
        .unwrap();
    let body = b"Synthetic versioned provenance source".to_vec();
    let addition = |source| GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        schema_version: None,
        evidence: BTreeSet::new(),
        operations: vec![GraphOperation::AddEvidence(Box::new(
            crate::EvidenceAddition {
                evidence: Evidence {
                    id: EvidenceId::mint(),
                    source,
                    content_hash: ContentHash::of_bytes(&body),
                    extracted_by: context().operator,
                    observed_at: Timestamp::EPOCH,
                    confidence: Confidence::CERTAIN,
                },
                payload: body.clone(),
            },
        ))],
    };
    let rejected = addition(EvidenceSource::Observation(ekr_core::ObservationId::mint()));
    kernel
        .propose(&encode(&rejected), context().operator, || {
            Timestamp::from_millis(8)
        })
        .unwrap();
    let ValidationCommandResult::Rejected(refusal) = kernel
        .validate(
            rejected.id,
            kernel.head().unwrap().unwrap().revision,
            || Timestamp::from_millis(9),
        )
        .unwrap()
    else {
        panic!("knowledge/2 admitted an Observation");
    };
    assert!(refusal
        .issues
        .iter()
        .any(|issue| issue.code == "evidence-unsupported-source"));
    let pending = addition(EvidenceSource::HumanStatement { identity: None });
    kernel
        .propose(&encode(&pending), context().operator, || {
            Timestamp::from_millis(10)
        })
        .unwrap();
    assert!(matches!(
        kernel
            .validate(pending.id, kernel.head().unwrap().unwrap().revision, || {
                Timestamp::from_millis(11)
            })
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    let old_events: Vec<_> = kernel
        .store
        .history()
        .unwrap()
        .occurrences
        .into_iter()
        .map(|o| o.event)
        .collect();
    let old_bytes: Vec<_> = old_events
        .iter()
        .map(|event| {
            (
                event.record_hash,
                kernel.content(&event.record_hash).unwrap().unwrap(),
            )
        })
        .collect();
    let explanation = kernel.read(None).unwrap().explain(assertion_id).unwrap();
    assert!(explanation.links.iter().any(|link| matches!(link, crate::ExplanationLink::Validation(v) if v.validation_profile == profile)));
    let preview = kernel.preview_upgrade(&human.policy).unwrap();
    assert_eq!(preview.from.ruleset.0, "ekr.knowledge-deterministic/2");
    assert_eq!(preview.to.ruleset.0, "ekr.knowledge-deterministic/3");
    assert_eq!(preview.pending_revalidation.len(), 1);
    kernel
        .apply_upgrade(
            &preview,
            &human.policy,
            &human.proof(&preview),
            b"reviewed contradictions and pending validations",
            || Timestamp::from_millis(12),
        )
        .unwrap();
    assert_eq!(
        kernel.transaction_states([pending.id]).unwrap()[&pending.id],
        TransactionState::Proposed
    );
    assert!(kernel
        .commit(pending.id, context().operator, || Timestamp::from_millis(
            13
        ))
        .is_err());
    assert!(matches!(
        kernel
            .validate(pending.id, kernel.head().unwrap().unwrap().revision, || {
                Timestamp::from_millis(14)
            })
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    kernel
        .commit(pending.id, context().operator, || {
            Timestamp::from_millis(15)
        })
        .unwrap();
    let final_root = kernel.head().unwrap();
    drop(kernel);
    for full in [false, true] {
        let reopened = open(Some(human.binding.clone()), full);
        assert_eq!(reopened.head().unwrap(), final_root);
        assert_eq!(
            reopened
                .store
                .history()
                .unwrap()
                .occurrences
                .into_iter()
                .map(|o| o.event)
                .take(old_events.len())
                .collect::<Vec<_>>(),
            old_events
        );
        for (hash, bytes) in &old_bytes {
            assert_eq!(&reopened.content(hash).unwrap().unwrap(), bytes);
        }
        assert_eq!(
            reopened.transaction_states([rejected.id]).unwrap()[&rejected.id],
            TransactionState::Rejected
        );
        let now = reopened.read(None).unwrap().explain(assertion_id).unwrap();
        assert_eq!(now.links, explanation.links);
        assert_eq!(
            reopened
                .schema_history(schema_root.revision)
                .unwrap()
                .supporting_evidence[&schema_root.revision],
            schema.evidence
        );
    }
}

#[test]
fn knowledge_two_history_survives_the_reviewed_third_profile() {
    let file = tempfile::tempdir().unwrap();
    exercise(|binding, full| {
        match binding {
            Some(binding) => {
                Commit::over_with_review_authority(context(), anchor(), binding, |authority| {
                    FileStore::file(file.path(), "upgrade-fixture", None).map(|mut store| {
                        store.set_full_replay(full);
                        store.under(authority)
                    })
                })
            }
            None => Commit::over_with_authority(context(), anchor(), |authority| {
                FileStore::file(file.path(), "upgrade-fixture", None).map(|mut store| {
                    store.set_full_replay(full);
                    store.under(authority)
                })
            }),
        }
        .unwrap()
    });
    let sqlite = tempfile::tempdir().unwrap();
    exercise(|binding, full| {
        match binding {
            Some(binding) => {
                Commit::over_with_review_authority(context(), anchor(), binding, |authority| {
                    SqliteStore::sqlite(&sqlite.path().join("store.db"), "upgrade-fixture", None)
                        .map(|mut store| {
                            store.set_full_replay(full);
                            store.under(authority)
                        })
                })
            }
            None => Commit::over_with_authority(context(), anchor(), |authority| {
                SqliteStore::sqlite(&sqlite.path().join("store.db"), "upgrade-fixture", None).map(
                    |mut store| {
                        store.set_full_replay(full);
                        store.under(authority)
                    },
                )
            }),
        }
        .unwrap()
    });
}
