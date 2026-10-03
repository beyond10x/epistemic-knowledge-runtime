//! Provider-backed admission through retained reviewer enrollment and the actual sealed pipeline.
use crate as ekr_kernel;
include!("../../tests/support/attention_answer_fixture.rs");

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

fn publish_and_reopen<S: RevisionLog + ObjectStore + Initialize>(
    open: impl Fn(Option<m::TrustedReviewHostBinding>) -> Commit<S>,
    kind: m::ClaimCorrectionKind,
) {
    let kernel = open(None);
    let seeded = kernel.seed(seed(), || Timestamp::EPOCH).unwrap();
    let graph = kernel.read(None).unwrap();
    let claim = *graph.graph.assertions.keys().next().unwrap();
    let evidence_id = EvidenceId::mint();
    let payload = b"additional retained support from an ordinary transaction".to_vec();
    let transaction = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![
            GraphOperation::AddEvidence(Box::new(crate::EvidenceAddition {
                evidence: Evidence {
                    id: evidence_id,
                    source: EvidenceSource::HumanStatement { identity: None },
                    content_hash: ContentHash::of_bytes(&payload),
                    extracted_by: context().operator,
                    observed_at: Timestamp::EPOCH,
                    confidence: Confidence::CERTAIN,
                },
                payload,
            })),
            GraphOperation::AttachEvidence(crate::EvidenceAttachment {
                assertion: claim,
                evidence: evidence_id,
            }),
        ],
        evidence: BTreeSet::from([evidence_id]),
        schema_version: None,
    };
    kernel
        .propose(&encode(&transaction), context().operator, || {
            Timestamp::EPOCH
        })
        .unwrap();
    assert!(matches!(
        kernel
            .validate(transaction.id, RevisionNumber::SEED, || Timestamp::EPOCH)
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        kernel
            .commit(transaction.id, context().operator, || Timestamp::EPOCH)
            .unwrap(),
        CommitCommandResult::Committed(_)
    ));
    let mut human = Human::new(seeded.seed_hash);
    human.policy.keys[0]
        .scopes
        .insert(0, m::HumanDecisionScope::AnswerAttention);
    human.binding.reviewer_policy_digest = hash(&review::policy_bytes(&human.policy).unwrap());
    drop(kernel);
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
    let before = kernel.read(None).unwrap();
    let input = answer(&human, &before, kind);
    let chosen: AssertionId = input.corrections[0].assertion_id.0 .0.parse().unwrap();
    let record = kernel
        .answer_attention(&input, || Timestamp::from_millis(2))
        .unwrap();
    let after = kernel.read(None).unwrap();
    assert_eq!(after.graph.revision, before.graph.revision.next().unwrap());
    let statement: EvidenceId = record.statement_evidence.0.parse().unwrap();
    assert_eq!(
        after.graph.evidence[&statement].content_hash,
        ContentHash::of_bytes(&input.statement)
    );
    for (id, evidence) in &before.graph.evidence {
        assert_eq!(&after.graph.evidence[id], evidence);
    }
    let current = after.dispute_attention().unwrap();
    if kind == m::ClaimCorrectionKind::Unresolved {
        assert_eq!(after.graph.assertions, before.graph.assertions);
        assert_eq!(current.len(), 1);
    } else if kind == m::ClaimCorrectionKind::CorrectTime {
        assert_eq!(current.len(), 1);
        assert_eq!(
            *record.receipt.outcome,
            ekr_core::contract_data::EkrKernelAnswerOutcome::V1
        );
    } else {
        assert!(current.is_empty());
    }
    if kind == m::ClaimCorrectionKind::CorrectTime {
        assert_eq!(
            after.graph.assertions[&chosen].valid_time,
            before.graph.assertions[&chosen].valid_time
        );
        let replacement: AssertionId = record.replacements[0].replacement.0.parse().unwrap();
        assert!(after.graph.assertions[&replacement]
            .evidence
            .iter()
            .any(|r| r.id() == statement));
    }
    assert_eq!(
        record,
        kernel
            .answer_attention(&input, || panic!("retry must not allocate"))
            .unwrap()
    );
    let mut changed = input.clone();
    changed.statement.push(b'!');
    assert!(kernel
        .answer_attention(&changed, || panic!("conflict must not allocate"))
        .is_err());
    let (_, state) = kernel.replayed_state().unwrap();
    let root = state.head().root;
    let version = state.version;
    assert_eq!(kernel.answer_history(None).unwrap(), vec![record.clone()]);
    drop(kernel);
    let reopened = open(Some(human.binding.clone()));
    assert_eq!(reopened.answer_history(None).unwrap(), vec![record.clone()]);
    assert_eq!(
        reopened
            .answer_attention(&input, || panic!("reopened retry must not allocate"))
            .unwrap(),
        record
    );
    let history = reopened.store.history().unwrap();
    let full = reopened
        .authority
        .reconstruct_in_full(&history)
        .unwrap()
        .unwrap();
    assert_eq!(full.head().root, root);
    assert_eq!(full.version, version);
    assert_eq!(full.head().graph().unwrap(), after.graph.as_ref());
    assert_eq!(full.answers.values().collect::<Vec<_>>(), vec![&record]);
    if kind == m::ClaimCorrectionKind::CorrectTime {
        legacy_timestamp_replay(&reopened.authority, &history, &record, &root);
    }
    // Re-address the changed object and occurrence, so content hashing alone cannot catch the
    // forgery. Full replay must independently reconstruct the signed input and resulting record.
    for pointer in [
        "/basis/evidence_digest",
        "/validation_hash",
        "/review/policy_digest",
        "/transaction_object_hash",
        "/corrections/0/reason",
        "/result/knowledge_root",
    ] {
        let mut forged = history.clone();
        let mut value = serde_json::to_value(&record).unwrap();
        *value.pointer_mut(pointer).unwrap() = if pointer.ends_with("reason") {
            serde_json::json!("unreviewed reason")
        } else {
            serde_json::json!(ContentHash::of_bytes(b"forged").to_string())
        };
        let data = serde_json::to_vec(&value).unwrap();
        let address = ContentHash::of_bytes(&data);
        let occurrence = forged.occurrences.last_mut().unwrap();
        let mut object = forged.objects[&occurrence.event.record_hash].clone();
        object.bytes = std::sync::Arc::new(data);
        object.metadata.content_hash = address;
        object.metadata.byte_len = object.bytes.len() as u64;
        forged.objects.insert(address, object);
        occurrence.event.record_hash = address;
        assert!(
            reopened.authority.reconstruct_in_full(&forged).is_err(),
            "admitted forged {pointer}"
        );
    }
    assert_eq!(
        reopened.read(Some(before.graph.revision)).unwrap().graph,
        before.graph
    );
    if !current.is_empty() {
        let mut next = answer(&human, &after, m::ClaimCorrectionKind::Choose);
        if next.dispute_id == input.dispute_id {
            next.human_proof.intent.expected_previous_decision =
                Some(m::ContentHash(record.review.proof_digest.0.clone()));
        }
        next.human_proof.signature = human
            .key
            .sign(&review::signing_bytes(&next.human_proof.intent).unwrap())
            .as_ref()
            .to_vec();
        let second = reopened
            .answer_attention(&next, || Timestamp::from_millis(3))
            .unwrap();
        assert!(reopened
            .read(None)
            .unwrap()
            .dispute_attention()
            .unwrap()
            .is_empty());
        assert_eq!(
            reopened.answer_history(None).unwrap(),
            vec![record.clone(), second]
        );
        let history = reopened.store.history().unwrap();
        let full = reopened
            .authority
            .reconstruct_in_full(&history)
            .unwrap()
            .unwrap();
        assert!(full
            .head()
            .graph()
            .unwrap()
            .assertions
            .values()
            .all(|claim| !matches!(claim.assessment, Assessment::Disputed { .. })));
        assert_eq!(
            reopened
                .answer_attention(&input, || panic!("old retry after another answer"))
                .unwrap(),
            record
        );
    }
}
fn legacy_timestamp_replay(
    authority: &crate::KernelAuthority,
    history: &ekr_store::RetainedHistory,
    record: &ekr_core::contract_data::EkrKernelHumanAnswerRecord,
    root: &ekr_graph::Root,
) {
    // These were accepted String spellings in answer format /1 before ESS 0.52. The signed
    // correction bytes and ordinary transaction are unchanged: only the retained JSON differs.
    for (text, accepted) in [
        ("2000-01-01T00:00:00.000Z", true),
        ("2000-01-01T00:00:00+00:00", true),
        ("2000-01-01T01:00:00.000+01:00", true),
        ("2000-01-01T00:00:00.001Z", false),
        ("2000-01-01T00:00:00.000001Z", false),
        ("not-a-time", false),
    ] {
        let mut value = serde_json::to_value(record).unwrap();
        assert_eq!(
            serde_json::to_vec(&value).unwrap(),
            super::durable::bytes(record).unwrap()
        );
        value["corrections"][0]["valid_from"] = text.into();
        let data = serde_json::to_vec(&value).unwrap();
        let address = ContentHash::of_bytes(&data);
        let mut historical = history.clone();
        let occurrence = historical.occurrences.last_mut().unwrap();
        let mut object = historical.objects[&occurrence.event.record_hash].clone();
        object.bytes = std::sync::Arc::new(data.clone());
        object.metadata.content_hash = address;
        object.metadata.byte_len = data.len() as u64;
        historical.objects.insert(address, object);
        occurrence.event.record_hash = address;
        let replayed = authority.reconstruct_in_full(&historical);
        if accepted {
            assert_eq!(replayed.unwrap().unwrap().head().root, *root, "{text}");
            assert_eq!(
                *historical.objects[&address].bytes, data,
                "historical bytes changed"
            );
        } else {
            assert!(replayed.is_err(), "admitted changed correction: {text}");
        }
    }
}
#[test]
fn reviewed_answers_publish_retry_reopen_and_fully_replay_on_both_providers() {
    for kind in [
        m::ClaimCorrectionKind::Choose,
        m::ClaimCorrectionKind::Retract,
        m::ClaimCorrectionKind::CorrectTime,
        m::ClaimCorrectionKind::Unresolved,
    ] {
        let directory = tempfile::tempdir().unwrap();
        publish_and_reopen(
            |binding| {
                let open =
                    |authority| {
                        Ok(FileStore::file(directory.path(), "upgrade-fixture", None)?
                            .under(authority))
                    };
                match binding {
                    Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
                    None => Commit::over_with_authority(context(), anchor(), open),
                }
                .unwrap()
            },
            kind,
        );
        let directory = tempfile::tempdir().unwrap();
        publish_and_reopen(
            |binding| {
                let open = |authority| {
                    Ok(SqliteStore::sqlite(
                        &directory.path().join("store.db"),
                        "upgrade-fixture",
                        None,
                    )?
                    .under(authority))
                };
                match binding {
                    Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
                    None => Commit::over_with_authority(context(), anchor(), open),
                }
                .unwrap()
            },
            kind,
        );
    }
}
