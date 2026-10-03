//! Reviewed corrections remain explainable at their captured and historical boundaries.
#![allow(dead_code, unused_imports)]
include!("support/attention_answer_fixture.rs");

fn exercise<S: RevisionLog + ObjectStore + Initialize>(
    open: impl Fn(Option<m::TrustedReviewHostBinding>) -> Commit<S>,
    kind: m::ClaimCorrectionKind,
) {
    let document = seed();
    let seeded = open(None)
        .seed(document.clone(), || Timestamp::EPOCH)
        .unwrap();
    let mut human = Human::new(seeded.seed_hash);
    human.policy.keys[0]
        .scopes
        .insert(0, m::HumanDecisionScope::AnswerAttention);
    human.binding.reviewer_policy_digest = hash(&review::policy_bytes(&human.policy).unwrap());
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
    before
        .snapshot(None)
        .expect("an authority transition is a valid snapshot boundary");
    let input = answer(&human, &before, kind);
    let chosen: AssertionId = input.corrections[0].assertion_id.0 .0.parse().unwrap();
    let historical = before.explain(chosen).unwrap();
    let record = kernel
        .answer_attention(&input, || Timestamp::from_millis(2))
        .unwrap();
    let captured = kernel.read(None).unwrap();
    captured
        .snapshot(None)
        .expect("a reviewed answer is a valid snapshot boundary");
    assert_eq!(before.explain(chosen).unwrap(), historical);
    let linked = |id| {
        let explanation = captured.explain(id).unwrap();
        let json = serde_json::to_value(&explanation).unwrap();
        let answers: Vec<_> = json["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["kind"] == "HumanAnswer")
            .collect();
        assert_eq!(answers.len(), 1, "{json}");
        assert_eq!(answers[0]["record"], serde_json::to_value(&record).unwrap());
        let address: ContentHash = answers[0]["record_hash"].as_str().unwrap().parse().unwrap();
        assert_eq!(
            captured.content(&address).unwrap(),
            serde_json::to_vec(&record).unwrap()
        );
        let evidence: BTreeSet<_> = json["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["kind"] == "Evidence")
            .map(|v| v["id"].as_str().unwrap().to_owned())
            .collect();
        assert!(evidence.contains(&record.statement_evidence.0));
        for id in &before.graph.assertions[&id].evidence {
            assert!(evidence.contains(&id.id().to_string()));
        }
        explanation
    };
    let expected = linked(chosen);
    if kind == m::ClaimCorrectionKind::Choose {
        for id in before
            .graph
            .assertions
            .keys()
            .copied()
            .filter(|id| *id != chosen)
        {
            linked(id);
        }
    }
    if kind == m::ClaimCorrectionKind::CorrectTime {
        let replacement: AssertionId = record.replacements[0].replacement.0.parse().unwrap();
        let explanation = captured.explain(replacement).unwrap();
        let assertions: BTreeSet<_> = explanation
            .links
            .iter()
            .filter_map(|link| match link {
                ekr_kernel::ExplanationLink::Assertion(a) => Some(a.id),
                _ => None,
            })
            .collect();
        assert_eq!(assertions, BTreeSet::from([chosen, replacement]));
        assert_eq!(
            captured.graph.assertions[&chosen].valid_time,
            before.graph.assertions[&chosen].valid_time
        );
    }
    // Ordinary publications after upgrade use the new profile; old captures keep their rules.
    let mut ordinary = document.graph.assertions.values().next().unwrap().clone();
    ordinary.id = AssertionId::mint();
    let ordinary_id = ordinary.id;
    let transaction = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        evidence: ordinary.evidence.clone(),
        operations: vec![GraphOperation::AddAssertion(Box::new(ordinary))],
        schema_version: None,
    };
    kernel
        .propose(&encode(&transaction), context().operator, || {
            Timestamp::from_millis(3)
        })
        .unwrap();
    let validation = kernel
        .validate(transaction.id, captured.root.revision, || {
            Timestamp::from_millis(3)
        })
        .unwrap();
    assert!(
        matches!(validation, ValidationCommandResult::Validated(_)),
        "{validation:?}"
    );
    kernel
        .commit(transaction.id, context().operator, || {
            Timestamp::from_millis(3)
        })
        .unwrap();
    let added = kernel.read(None).unwrap().explain(ordinary_id).unwrap();
    assert!(added.links.iter().any(|link| matches!(link, ekr_kernel::ExplanationLink::Validation(v) if v.validation_profile == ValidationProfileV1::knowledge(context().validator))));
    assert_eq!(captured.explain(chosen).unwrap(), expected);
    drop(kernel);
    let reopened = open(Some(human.binding));
    assert_eq!(
        reopened
            .read(Some(captured.root.revision))
            .unwrap()
            .explain(chosen)
            .unwrap(),
        expected
    );
    assert_eq!(
        reopened
            .read(Some(RevisionNumber::new(1)))
            .unwrap()
            .explain(chosen)
            .unwrap(),
        historical
    );
    // Public mutable capture fields are not authority: a forged coordinate cannot explain.
    let mut forged = reopened.read(None).unwrap();
    forged.revisions.last_entry().unwrap().get_mut().record_hash = ContentHash::of_bytes(b"forged");
    assert!(forged.explain(chosen).is_err());
    let mut forged = reopened.read(Some(captured.root.revision)).unwrap();
    forged.revisions.last_entry().unwrap().get_mut().event_id = ekr_core::EventId::mint();
    assert!(
        matches!(forged.explain(chosen), Err(ekr_kernel::ProjectionError::Unverified { code }) if code == "answer-coordinate-disagrees")
    );
}

#[test]
fn all_reviewed_corrections_explain_their_signed_origin_and_evidence_on_both_providers() {
    for kind in [
        m::ClaimCorrectionKind::Choose,
        m::ClaimCorrectionKind::Retract,
        m::ClaimCorrectionKind::CorrectTime,
        m::ClaimCorrectionKind::Unresolved,
    ] {
        let directory = tempfile::tempdir().unwrap();
        exercise(
            |binding| match binding {
                Some(binding) => {
                    Commit::over_with_review_authority(context(), anchor(), binding, |a| {
                        let mut s =
                            FileStore::file(directory.path(), "upgrade-fixture", None)?.under(a);
                        s.set_full_replay(true);
                        Ok(s)
                    })
                    .unwrap()
                }
                None => Commit::over_with_authority(context(), anchor(), |a| {
                    FileStore::file(directory.path(), "upgrade-fixture", None).map(|s| s.under(a))
                })
                .unwrap(),
            },
            kind,
        );
        let directory = tempfile::tempdir().unwrap();
        exercise(
            |binding| match binding {
                Some(binding) => {
                    Commit::over_with_review_authority(context(), anchor(), binding, |a| {
                        let mut s = SqliteStore::sqlite(
                            &directory.path().join("store.db"),
                            "upgrade-fixture",
                            None,
                        )?
                        .under(a);
                        s.set_full_replay(true);
                        Ok(s)
                    })
                    .unwrap()
                }
                None => Commit::over_with_authority(context(), anchor(), |a| {
                    SqliteStore::sqlite(&directory.path().join("store.db"), "upgrade-fixture", None)
                        .map(|s| s.under(a))
                })
                .unwrap(),
            },
            kind,
        );
    }
}
