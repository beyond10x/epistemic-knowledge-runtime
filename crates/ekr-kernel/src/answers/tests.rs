//! Provider-backed admission through retained reviewer enrollment and the actual sealed pipeline.
use crate as ekr_kernel;
include!("../../tests/support/authority_review_fixture.rs");

fn answer(
    human: &Human,
    read: &crate::VerifiedRead,
    kind: m::ClaimCorrectionKind,
) -> m::AttentionAnswerApplication {
    let current =
        crate::attention_behavior::item(&read.dispute_attention().unwrap().remove(0)).unwrap();
    let corrections = vec![m::ClaimCorrection {
        kind,
        assertion_id: current.claims[0].clone(),
        valid_from: (kind == m::ClaimCorrectionKind::CorrectTime)
            .then(|| ekr_core::contracts::primitives::Timestamp("2000-01-01T00:00:00Z".into())),
        valid_to: None,
        reason: "human reviewed the retained source evidence".into(),
    }];
    let target = m::HumanDecisionTarget::AnswerAttention(m::AttentionAnswerTarget {
        dispute_id: current.subject.dispute_id.clone().unwrap(),
        basis: current.basis.clone(),
        corrections_digest: hash(&review::corrections_bytes(&corrections).unwrap()),
    });
    let intent = m::HumanDecisionIntent {
        format: m::HumanDecisionFormat::HumanDecision1,
        decision_id: Uuid(ekr_core::EventId::mint().to_string()),
        audience: human.binding.audience.clone(),
        reviewer_policy_digest: human.binding.reviewer_policy_digest.clone(),
        signer_key_digest: hash(human.key.public_key().as_ref()),
        target,
        statement_digest: hash(b"reviewed answer"),
        expected_previous_decision: None,
    };
    let signature = human
        .key
        .sign(&review::signing_bytes(&intent).unwrap())
        .as_ref()
        .to_vec();
    m::AttentionAnswerApplication {
        human_proof: m::SignedHumanDecision {
            intent,
            algorithm: m::ReviewSignatureAlgorithm::Ed25519,
            signature,
        },
        dispute_id: current.subject.dispute_id.unwrap(),
        basis: current.basis,
        corrections,
        statement: b"reviewed answer".to_vec(),
    }
}
fn run<S: RevisionLog + ObjectStore + Initialize>(
    open: impl Fn(Option<m::TrustedReviewHostBinding>) -> Commit<S>,
) {
    let original = open(None);
    let document = seed();
    let seeded = original
        .seed(document.clone(), || Timestamp::EPOCH)
        .unwrap();
    let mut human = Human::new(seeded.seed_hash);
    human.policy.keys[0]
        .scopes
        .insert(0, m::HumanDecisionScope::AnswerAttention);
    human.binding.reviewer_policy_digest = hash(&review::policy_bytes(&human.policy).unwrap());
    drop(original);
    let kernel = open(Some(human.binding.clone()));
    let preview = kernel.preview_upgrade(&human.policy).unwrap();
    kernel
        .apply_upgrade(
            &preview,
            &human.policy,
            &human.proof(&preview),
            b"reviewed contradictions and pending validations",
            || Timestamp::from_millis(1),
        )
        .unwrap();
    let read = kernel.read(None).unwrap();
    let (history, state) = kernel.replayed_state().unwrap();
    let baseline = read.graph.clone();
    let mut hashes = BTreeSet::new();
    for kind in [
        m::ClaimCorrectionKind::Choose,
        m::ClaimCorrectionKind::Retract,
        m::ClaimCorrectionKind::CorrectTime,
        m::ClaimCorrectionKind::Unresolved,
    ] {
        let input = answer(&human, &read, kind);
        let chosen: AssertionId = input.corrections[0].assertion_id.0 .0.parse().unwrap();
        let statement = EvidenceId::mint();
        let tx = TransactionId::mint();
        let allocation = (tx, statement, Timestamp::from_millis(2));
        let replacement_id = AssertionId::mint();
        let replacements = if kind == m::ClaimCorrectionKind::CorrectTime {
            vec![m::ClaimReplacement {
                previous: input.corrections[0].assertion_id.clone(),
                replacement: ekr_core::contracts::graph::AssertionId(Uuid(
                    replacement_id.to_string(),
                )),
            }]
        } else {
            vec![]
        };
        let (validated, proof) = kernel
            .authority
            .validate_answer(&history, &state, &input, &replacements, allocation)
            .unwrap();
        let (again, _) = kernel
            .authority
            .validate_answer(&history, &state, &input, &replacements, allocation)
            .unwrap();
        assert_eq!(validated.validation_hash(), again.validation_hash());
        assert!(hashes.insert(validated.validation_hash()));
        assert_eq!(validated.validated_against(), state.head().root.revision);
        let proposed: crate::GraphTransaction =
            serde_json::from_value(serde_json::to_value(validated.transaction()).unwrap()).unwrap();
        let ordinary = crate::Pipeline::identity_keeping(
            context().validator,
            BTreeSet::new(),
            state.held.clone(),
        )
        .validate(&ekr_graph::GraphSnapshot::of(&read.graph), &proposed);
        if kind != m::ClaimCorrectionKind::Unresolved {
            assert!(ordinary
                .unwrap_err()
                .iter()
                .any(|i| i.code == "assertion-lifecycle-state"));
        } else {
            assert!(ordinary.is_ok());
        }
        if kind == m::ClaimCorrectionKind::Choose {
            let withdrawals: BTreeSet<_> = proposed
                .operations
                .iter()
                .filter_map(|op| match op {
                    GraphOperation::RetractAssertion(r) => Some(r.assertion),
                    _ => None,
                })
                .collect();
            let pipeline = crate::Pipeline::reviewed_answer(
                context().validator,
                BTreeSet::new(),
                state.held.clone(),
                withdrawals.clone(),
            );
            let snapshot = ekr_graph::GraphSnapshot::of(&read.graph);
            let mut unreviewed = proposed.clone();
            unreviewed
                .operations
                .push(GraphOperation::RetractAssertion(crate::Retraction {
                    assertion: chosen,
                    reason: ekr_graph::RetractionReason::new("outside reviewed withdrawal set"),
                }));
            assert!(pipeline
                .validate(&snapshot, &unreviewed)
                .unwrap_err()
                .iter()
                .any(|i| i.code == "assertion-lifecycle-state"));
            let mut self_validated = proposed.clone();
            self_validated.proposer = context().validator;
            assert!(pipeline
                .validate(&snapshot, &self_validated)
                .unwrap_err()
                .iter()
                .any(|i| i.code == "proposer-is-validator"));
            let withdrawn = *withdrawals.first().unwrap();
            let mut superseded = proposed.clone();
            for op in &mut superseded.operations {
                if matches!(op, GraphOperation::RetractAssertion(_)) {
                    *op = GraphOperation::SupersedeAssertion(crate::Supersession {
                        assertion: withdrawn,
                        by: chosen,
                        effective_from: Timestamp::EPOCH,
                    });
                }
            }
            assert!(pipeline
                .validate(&snapshot, &superseded)
                .unwrap_err()
                .iter()
                .any(|i| i.code == "assertion-lifecycle-state"));
            let mut closed = (*read.graph).clone();
            closed
                .assertions
                .get_mut(&withdrawn)
                .unwrap()
                .transaction_time =
                TransactionTime::new(Timestamp::EPOCH, Some(Timestamp::from_millis(1))).unwrap();
            assert!(pipeline
                .validate(&ekr_graph::GraphSnapshot::of(&closed), &proposed)
                .unwrap_err()
                .iter()
                .any(|i| i.code == "assertion-lifecycle-state"));
        }
        let (mut applied, _) = crate::apply::apply(
            state.head(),
            &validated,
            &BTreeSet::from([context().validator]),
            Timestamp::from_millis(2),
        )
        .unwrap();
        crate::disputes::recompute(&mut applied, &mut state.assessment_validators.clone()).unwrap();
        assert_eq!(
            applied.evidence[&statement].content_hash,
            ContentHash::of_bytes(proof.statement())
        );
        assert_eq!(
            applied.evidence[&statement].source,
            EvidenceSource::HumanStatement {
                identity: Some("fixture-human".into())
            }
        );
        match kind {
            m::ClaimCorrectionKind::Choose => {
                assert!(applied.assertions[&chosen].is_current());
                assert_eq!(
                    applied
                        .assertions
                        .values()
                        .filter(|a| a.is_current())
                        .count(),
                    1
                );
            }
            m::ClaimCorrectionKind::Retract => {
                assert!(matches!(
                    applied.assertions[&chosen].lifecycle,
                    AssertionLifecycle::Retracted { .. }
                ));
                assert_eq!(
                    applied
                        .assertions
                        .values()
                        .filter(|a| a.is_current())
                        .count(),
                    1
                );
            }
            m::ClaimCorrectionKind::CorrectTime => {
                assert_eq!(
                    applied.assertions[&chosen].valid_time,
                    baseline.assertions[&chosen].valid_time
                );
                assert!(matches!(
                    applied.assertions[&chosen].lifecycle,
                    AssertionLifecycle::Retracted { .. }
                ));
                assert!(applied.assertions[&replacement_id]
                    .evidence
                    .contains(&ekr_graph::CanonicalRef::new(statement)));
                assert!(applied.assertions.values().all(|a| match &a.assessment {
                    Assessment::Disputed {
                        competing_assertions,
                    } => competing_assertions.iter().all(|id| id.id() != chosen
                        && matches!(
                            applied.assertions[&id.id()].lifecycle,
                            AssertionLifecycle::Active
                        )),
                    _ => true,
                }));
            }
            m::ClaimCorrectionKind::Unresolved => {
                assert_eq!(applied.assertions, baseline.assertions)
            }
        }
        let mut forged = input.clone();
        forged.human_proof.signature[0] ^= 1;
        assert!(kernel
            .authority
            .validate_answer(&history, &state, &forged, &replacements, allocation)
            .is_err());
        let mut changed = input.clone();
        changed.statement.push(b'!');
        assert!(kernel
            .authority
            .validate_answer(&history, &state, &changed, &replacements, allocation)
            .is_err());
        let mut changed = input.clone();
        changed.basis.observed_revision.0 += 1;
        assert!(kernel
            .authority
            .validate_answer(&history, &state, &changed, &replacements, allocation)
            .is_err());
        let mut changed = input.clone();
        changed.corrections[0].reason.push('!');
        assert!(kernel
            .authority
            .validate_answer(&history, &state, &changed, &replacements, allocation)
            .is_err());
        let missing_host = open(None);
        assert!(missing_host
            .authority
            .validate_answer(&history, &state, &input, &replacements, allocation)
            .is_err());
        let mut before_upgrade = (*state).clone();
        before_upgrade.transition = None;
        assert!(kernel
            .authority
            .validate_answer(&history, &before_upgrade, &input, &replacements, allocation)
            .is_err());
        let held_evidence = *read.graph.evidence.keys().next().unwrap();
        assert!(kernel
            .authority
            .validate_answer(
                &history,
                &state,
                &input,
                &replacements,
                (tx, held_evidence, Timestamp::from_millis(2))
            )
            .is_err());
    }
    assert_eq!(
        *kernel.read(None).unwrap().graph,
        *baseline,
        "candidate admission must not publish"
    );
    let input = answer(&human, &read, m::ClaimCorrectionKind::Choose);
    let (id, document) = proposal(&document);
    kernel
        .propose(&document, context().operator, || Timestamp::from_millis(3))
        .unwrap();
    assert!(matches!(
        kernel
            .validate(id, read.root.revision, || Timestamp::from_millis(4))
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        kernel
            .commit(id, context().operator, || Timestamp::from_millis(5))
            .unwrap(),
        CommitCommandResult::Committed(_)
    ));
    let (advanced_history, advanced) = kernel.replayed_state().unwrap();
    assert!(
        kernel
            .authority
            .validate_answer(
                &advanced_history,
                &advanced,
                &input,
                &[],
                (
                    TransactionId::mint(),
                    EvidenceId::mint(),
                    Timestamp::from_millis(6)
                )
            )
            .is_ok(),
        "unrelated commit preserves the reviewed material basis"
    );
    let first = read.graph.assertions.values().next().unwrap();
    let mut extra: Assertion<Value> =
        serde_json::from_value(serde_json::to_value(first).unwrap()).unwrap();
    extra.id = AssertionId::mint();
    extra.assessment = Assessment::Proposed;
    extra.object = Object::Value(Value::String("additional conflicting claim".into()));
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        evidence: extra.evidence.clone(),
        operations: vec![GraphOperation::AddAssertion(Box::new(extra))],
        schema_version: None,
    };
    kernel
        .propose(&encode(&tx), context().operator, || {
            Timestamp::from_millis(7)
        })
        .unwrap();
    assert!(matches!(
        kernel
            .validate(tx.id, advanced.head().root.revision, || {
                Timestamp::from_millis(8)
            })
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(9))
            .unwrap(),
        CommitCommandResult::Committed(_)
    ));
    let (changed_history, changed) = kernel.replayed_state().unwrap();
    let refusal = kernel
        .authority
        .validate_answer(
            &changed_history,
            &changed,
            &input,
            &[],
            (
                TransactionId::mint(),
                EvidenceId::mint(),
                Timestamp::from_millis(10),
            ),
        )
        .err()
        .expect("changed claim options require new review");
    assert!(
        refusal.to_string().contains("answer-review-required"),
        "{refusal}"
    );
}
#[test]
fn signed_answers_validate_through_the_ordinary_pipeline_with_private_reviewed_withdrawals() {
    let file = tempfile::tempdir().unwrap();
    run(|binding| {
        let open =
            |authority| Ok(FileStore::file(file.path(), "upgrade-fixture", None)?.under(authority));
        match binding {
            Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
            None => Commit::over_with_authority(context(), anchor(), open),
        }
        .unwrap()
    });
    let sqlite = tempfile::tempdir().unwrap();
    run(|binding| {
        let open = |authority| {
            Ok(
                SqliteStore::sqlite(&sqlite.path().join("store.db"), "upgrade-fixture", None)?
                    .under(authority),
            )
        };
        match binding {
            Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
            None => Commit::over_with_authority(context(), anchor(), open),
        }
        .unwrap()
    });
}
