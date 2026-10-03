//! Exact external human signatures authorize retained reviews, never canonical publication.
#![allow(unused_imports, dead_code)]
include!("support/schema_proposal_fixture.rs");

#[test]
fn schema_proposal_requires_exact_human_approval() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let store = runtime(&path, sqlite);
        let seeded = store.seed(seed(), || Timestamp::EPOCH).unwrap();
        let human = schema_human(seeded.seed_hash);
        let proposal = proposal_input(&store);
        let shown = store
            .submit_schema_proposal(&proposal, Timestamp::EPOCH)
            .unwrap();
        let decision_id = ekr_core::EventId::mint();
        let before_upgrade = signed_review(
            &human,
            &shown,
            true,
            decision_id,
            b"Approve the exact additions.",
        );
        let events = store.published_events().unwrap();
        assert!(store
            .approve_schema_proposal(&before_upgrade, Timestamp::from_millis(1))
            .is_err());
        assert_eq!(store.published_events().unwrap(), events);
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
        let shown = store
            .schema_proposal(&proposal.proposal.proposal_id)
            .unwrap();
        let approval = signed_review(
            &human,
            &shown,
            true,
            decision_id,
            b"Approve the exact additions.",
        );
        let initial_root = store.read(None).unwrap().root;
        let events = store.published_events().unwrap();
        for field in [
            "signature",
            "statement",
            "proposal_digest",
            "basis",
            "target",
        ] {
            let mut changed = approval.clone();
            match field {
                "signature" => changed.human_proof.signature = ekr_core::bytes::encode(&[0; 64]),
                "statement" => changed.statement = ekr_core::bytes::encode(b"agent-approved"),
                "proposal_digest" => {
                    changed.proposal_digest.0 =
                        ContentHash::of_bytes(b"another proposal").to_string()
                }
                "basis" => {
                    changed.basis.effects_digest.0 =
                        ContentHash::of_bytes(b"another effect").to_string()
                }
                _ => {
                    assert!(store
                        .reject_schema_proposal(&changed, Timestamp::from_millis(2))
                        .is_err());
                    continue;
                }
            }
            assert!(
                store
                    .approve_schema_proposal(&changed, Timestamp::from_millis(2))
                    .is_err(),
                "accepted {field}"
            );
            assert_eq!(store.published_events().unwrap(), events);
        }
        let first = store
            .approve_schema_proposal(&approval, Timestamp::from_millis(2))
            .unwrap();
        assert_eq!(first.operator.authentication_subject, "fixture-human");
        assert_eq!(serde_json::to_value(&first.decision).unwrap(), "Approved");
        assert_eq!(store.read(None).unwrap().root, initial_root);
        let reviewed = store
            .schema_proposal(&proposal.proposal.proposal_id)
            .unwrap();
        assert_eq!(reviewed.reviews, [Box::new(first.clone())]);
        assert_eq!(
            store
                .schema_proposal_reviews(&proposal.proposal.proposal_id)
                .unwrap()
                .into_iter()
                .map(|retained| *retained.review)
                .collect::<Vec<_>>(),
            vec![first.clone()]
        );
        assert_eq!(
            reviewed.expected_previous_decision,
            w::EssPresence::Present(first.human_proof_digest.clone())
        );
        assert_eq!(
            store
                .approve_schema_proposal(&approval, Timestamp::from_millis(3))
                .unwrap(),
            first
        );
        let changed = signed_review(
            &human,
            &shown,
            true,
            decision_id,
            b"Changed input under a reused identity.",
        );
        assert!(store
            .approve_schema_proposal(&changed, Timestamp::from_millis(3))
            .is_err());
        let rejected = signed_review(
            &human,
            &reviewed,
            false,
            ekr_core::EventId::mint(),
            b"Reject future integration.",
        );
        let rejection = store
            .reject_schema_proposal(&rejected, Timestamp::from_millis(4))
            .unwrap();
        assert_eq!(
            serde_json::to_value(&rejection.decision).unwrap(),
            "Rejected"
        );
        assert_eq!(store.read(None).unwrap().root, initial_root);
        let events = store.published_events().unwrap();
        assert_eq!(
            store
                .approve_schema_proposal(&approval, Timestamp::from_millis(5))
                .unwrap(),
            first
        );
        assert_eq!(
            store.published_events().unwrap(),
            events,
            "retry must not reinstate an older approval"
        );
        let stale = signed_review(
            &human,
            &shown,
            true,
            ekr_core::EventId::mint(),
            b"Stale predecessor.",
        );
        assert!(store
            .approve_schema_proposal(&stale, Timestamp::from_millis(5))
            .is_err());
        assert_eq!(store.published_events().unwrap(), events);
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding.clone())
            .unwrap();
        reopened.set_full_replay(true);
        let final_read = reopened
            .schema_proposal(&proposal.proposal.proposal_id)
            .unwrap();
        assert_eq!(final_read.reviews, [Box::new(first), Box::new(rejection)]);
        assert_eq!(reopened.read(None).unwrap().root, initial_root);
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
fn commit_document(store: &Runtime, id: TransactionId, bytes: &[u8], at: i64) {
    let revision = store.read(None).unwrap().root.revision;
    store
        .propose(bytes, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    let validation = store
        .validate(id, revision, || Timestamp::from_millis(at))
        .unwrap();
    assert!(
        matches!(validation, ValidationCommandResult::Validated(_)),
        "{validation:?}"
    );
    assert!(matches!(
        store
            .commit(id, context().operator, || Timestamp::from_millis(at))
            .unwrap(),
        CommitCommandResult::Committed(_)
    ));
}

#[test]
fn schema_review_revalidates_material_and_replays_the_original_review_revision() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (store, human) = upgraded(&path, sqlite);
        let input = proposal_input(&store);
        let shown = store
            .submit_schema_proposal(&input, Timestamp::from_millis(2))
            .unwrap();
        let approval = signed_review(
            &human,
            &shown,
            true,
            ekr_core::EventId::mint(),
            b"Approve reviewed vocabulary.",
        );
        let (id, bytes) = proposal(&store.read(None).unwrap().seed_input);
        commit_document(&store, id, &bytes, 3);
        let first = store
            .approve_schema_proposal(&approval, Timestamp::from_millis(4))
            .unwrap();
        assert_eq!(first.basis, shown.basis);
        let current = store.schema_proposal(&input.proposal.proposal_id).unwrap();
        let pending = signed_review(
            &human,
            &current,
            true,
            ekr_core::EventId::mint(),
            b"A second review before changed effects.",
        );
        let read = store.read(None).unwrap();
        let tx = GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            operations: vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
                TypeId::mint(),
                "ReviewVocabulary",
            )))],
            evidence: read.graph.evidence.keys().copied().collect(),
            schema_version: Some(ekr_core::SchemaVersionId::mint()),
        };
        commit_document(&store, tx.id, &encode(&tx), 5);
        let events = store.published_events().unwrap();
        assert!(store
            .approve_schema_proposal(&pending, Timestamp::from_millis(6))
            .is_err());
        assert_eq!(store.published_events().unwrap(), events);
        let changed = store.schema_proposal(&input.proposal.proposal_id).unwrap();
        assert_ne!(changed.basis.effects_digest, first.basis.effects_digest);
        assert_eq!(changed.reviews, [Box::new(first.clone())]);
        // Signing current material while claiming the old revision must refuse before retention.
        let mut impossible = changed.clone();
        impossible.basis.observed_revision = shown.basis.observed_revision.clone();
        let impossible = signed_review(
            &human,
            &impossible,
            false,
            ekr_core::EventId::mint(),
            b"An invalid historical basis.",
        );
        assert!(store
            .reject_schema_proposal(&impossible, Timestamp::from_millis(6))
            .is_err());
        assert_eq!(store.published_events().unwrap(), events);
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding)
            .unwrap();
        reopened.set_full_replay(true);
        assert_eq!(
            reopened
                .schema_proposal(&input.proposal.proposal_id)
                .unwrap()
                .reviews,
            [Box::new(first)]
        );
    }
}

#[test]
fn schema_review_cannot_reuse_an_authority_upgrade_decision_identity() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let store = runtime(&path, sqlite);
        let seeded = store.seed(seed(), || Timestamp::EPOCH).unwrap();
        let human = schema_human(seeded.seed_hash);
        let store = store.with_review_authority(human.binding.clone()).unwrap();
        let preview = store.preview_upgrade(&human.policy).unwrap();
        let proof = human.proof(&preview);
        let reused = proof.intent.decision_id.0.parse().unwrap();
        store
            .apply_upgrade(
                &preview,
                &human.policy,
                &proof,
                b"reviewed contradictions and pending validations",
                || Timestamp::from_millis(1),
            )
            .unwrap();
        let input = proposal_input(&store);
        let shown = store
            .submit_schema_proposal(&input, Timestamp::from_millis(2))
            .unwrap();
        let approval = signed_review(
            &human,
            &shown,
            true,
            reused,
            b"A different decision using the upgrade identity.",
        );
        let before = store.published_events().unwrap();
        assert!(store
            .approve_schema_proposal(&approval, Timestamp::from_millis(3))
            .is_err());
        assert_eq!(store.published_events().unwrap(), before);
    }
}

#[test]
fn concurrent_identical_schema_reviews_return_one_elected_record() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (store, human) = upgraded(&path, sqlite);
        let input = proposal_input(&store);
        let shown = store
            .submit_schema_proposal(&input, Timestamp::from_millis(2))
            .unwrap();
        let approval = signed_review(
            &human,
            &shown,
            true,
            ekr_core::EventId::mint(),
            b"One exact reviewed decision.",
        );
        let first_binding = human.binding.clone();
        let second_binding = human.binding.clone();
        let barrier = std::sync::Barrier::new(2);
        let results = std::thread::scope(|scope| {
            let barrier = &barrier;
            let approval = &approval;
            let path = &path;
            let a = scope.spawn(move || {
                let first = runtime(path, sqlite)
                    .with_review_authority(first_binding)
                    .unwrap();
                barrier.wait();
                first.approve_schema_proposal(approval, Timestamp::from_millis(3))
            });
            let b = scope.spawn(move || {
                let second = runtime(path, sqlite)
                    .with_review_authority(second_binding)
                    .unwrap();
                barrier.wait();
                second.approve_schema_proposal(approval, Timestamp::from_millis(4))
            });
            (a.join().unwrap(), b.join().unwrap())
        });
        assert!(results.0.is_ok() && results.1.is_ok(), "{results:?}");
        assert_eq!(results.0, results.1);
        let shown = store.schema_proposal(&input.proposal.proposal_id).unwrap();
        assert_eq!(shown.reviews, [Box::new(results.0.unwrap())]);
    }
}

#[test]
fn schema_reviews_and_attention_answers_share_decision_identity_in_both_directions() {
    for sqlite in [false, true] {
        for answer_first in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("store");
            let (store, human) = upgraded(&path, sqlite);
            let input = proposal_input(&store);
            let shown = store
                .submit_schema_proposal(&input, Timestamp::from_millis(2))
                .unwrap();
            let answer = answer(
                &human,
                &store.read(None).unwrap(),
                m::ClaimCorrectionKind::Unresolved,
            );
            let approval = signed_review(
                &human,
                &shown,
                true,
                answer.human_proof.intent.decision_id.0.parse().unwrap(),
                b"Review with an identity also used for an answer.",
            );
            if answer_first {
                let first = store
                    .answer_attention(&answer, || Timestamp::from_millis(3))
                    .unwrap();
                let events = store.published_events().unwrap();
                assert!(store
                    .approve_schema_proposal(&approval, Timestamp::from_millis(4))
                    .is_err());
                assert_eq!(store.published_events().unwrap(), events);
                assert_eq!(
                    store
                        .answer_attention(&answer, || Timestamp::from_millis(5))
                        .unwrap(),
                    first
                );
            } else {
                let first = store
                    .approve_schema_proposal(&approval, Timestamp::from_millis(3))
                    .unwrap();
                let events = store.published_events().unwrap();
                assert!(store
                    .answer_attention(&answer, || Timestamp::from_millis(4))
                    .is_err());
                assert_eq!(store.published_events().unwrap(), events);
                assert_eq!(
                    store
                        .approve_schema_proposal(&approval, Timestamp::from_millis(5))
                        .unwrap(),
                    first
                );
            }
            drop(store);
            let mut reopened = runtime(&path, sqlite)
                .with_review_authority(human.binding)
                .unwrap();
            reopened.set_full_replay(true);
            assert_eq!(
                reopened.answer_history(None).unwrap().len(),
                usize::from(answer_first)
            );
            assert_eq!(
                reopened
                    .schema_proposal(&input.proposal.proposal_id)
                    .unwrap()
                    .reviews
                    .len(),
                usize::from(!answer_first)
            );
        }
    }
}

fn with_corrections(
    mut input: w::EkrIntegrateSchemaProposalImport,
    corrections: serde_json::Value,
) -> w::EkrIntegrateSchemaProposalImport {
    input.proposal.corrections = serde_json::from_value(corrections).unwrap();
    input.payload = ekr_core::bytes::encode(&serde_json::to_vec(&input.proposal).unwrap());
    input
}

#[test]
fn proposal_correction_review_binds_the_full_competing_component() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (store, human) = upgraded(&directory.path().join("store"), sqlite);
        let question = store.attention().unwrap().remove(0);
        let chosen: AssertionId = question.claims[0].0.parse().unwrap();
        let input = with_corrections(
            proposal_input(&store),
            serde_json::json!([
                {"kind":"Choose","assertion_id":chosen,"reason":"The supported ownership claim."}
            ]),
        );
        let shown = store
            .submit_schema_proposal(&input, Timestamp::from_millis(2))
            .unwrap();
        let approval = signed_review(
            &human,
            &shown,
            true,
            ekr_core::EventId::mint(),
            b"Approve additions and selected correction.",
        );
        let read = store.read(None).unwrap();
        let original = read.graph.assertions[&chosen].clone();
        let mut equal = read.seed_input.graph.assertions[&chosen].clone();
        equal.id = AssertionId::mint();
        equal.assessment = Assessment::Proposed;
        let evidence = equal.evidence.clone();
        let tx = GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            schema_version: None,
            evidence,
            operations: vec![GraphOperation::AddAssertion(Box::new(equal))],
        };
        commit_document(&store, tx.id, &encode(&tx), 3);
        let after = store.read(None).unwrap();
        assert_eq!(after.graph.assertions[&chosen], original);
        assert_eq!(after.dispute_attention().unwrap()[0].claims.len(), 3);
        let events = store.published_events().unwrap();
        assert!(
            store
                .approve_schema_proposal(&approval, Timestamp::from_millis(4))
                .is_err(),
            "accepted changed competing component"
        );
        assert_eq!(store.published_events().unwrap(), events);
        let current = store.schema_proposal(&input.proposal.proposal_id).unwrap();
        assert_ne!(shown.basis.options_digest, current.basis.options_digest);
    }
}

#[test]
fn proposal_submission_refuses_inapplicable_corrections() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (store, _) = upgraded(&directory.path().join("store"), sqlite);
        let question = store.attention().unwrap().remove(0);
        let chosen = &question.claims[0];
        let peer = &question.claims[1];
        let cases = [
            serde_json::json!([{"kind":"Choose","assertion_id":chosen,"reason":" "}]),
            serde_json::json!([{"kind":"Choose","assertion_id":chosen,"reason":"chosen","valid_from":"1970-01-01T00:00:00Z"}]),
            serde_json::json!([{"kind":"CorrectTime","assertion_id":chosen,"reason":"unchanged interval"}]),
            serde_json::json!([{"kind":"CorrectTime","assertion_id":chosen,"reason":"inverted","valid_from":"1970-01-01T00:00:02Z","valid_to":"1970-01-01T00:00:01Z"}]),
            serde_json::json!([{"kind":"Choose","assertion_id":chosen,"reason":"first"},{"kind":"Choose","assertion_id":peer,"reason":"competing"}]),
            serde_json::json!([{"kind":"Unresolved","assertion_id":chosen,"reason":"uncertain"},{"kind":"Retract","assertion_id":peer,"reason":"withdrawn"}]),
        ];
        for corrections in cases {
            let input = with_corrections(proposal_input(&store), corrections.clone());
            let events = store.published_events().unwrap();
            assert!(
                store
                    .submit_schema_proposal(&input, Timestamp::from_millis(2))
                    .is_err(),
                "accepted {corrections}"
            );
            assert_eq!(store.published_events().unwrap(), events);
        }
    }
}

#[test]
fn correction_review_survives_unrelated_advancement_and_remains_historical_after_resolution() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (store, human) = upgraded(&path, sqlite);
        let question = store
            .read(None)
            .unwrap()
            .dispute_attention()
            .unwrap()
            .remove(0);
        let input = with_corrections(
            proposal_input(&store),
            serde_json::json!([
                {"kind":"Choose","assertion_id":question.claims[0],"reason":"Reviewed supported ownership."}
            ]),
        );
        let shown = store
            .submit_schema_proposal(&input, Timestamp::from_millis(2))
            .unwrap();
        let approval = signed_review(
            &human,
            &shown,
            true,
            ekr_core::EventId::mint(),
            b"Approve the selected correction and additions.",
        );
        let (id, bytes) = proposal(&store.read(None).unwrap().seed_input);
        commit_document(&store, id, &bytes, 3);
        let retained = store
            .approve_schema_proposal(&approval, Timestamp::from_millis(4))
            .unwrap();
        assert_eq!(retained.basis, shown.basis);
        let correction = answer(
            &human,
            &store.read(None).unwrap(),
            m::ClaimCorrectionKind::Choose,
        );
        store
            .answer_attention(&correction, || Timestamp::from_millis(5))
            .unwrap();
        assert!(store
            .read(None)
            .unwrap()
            .dispute_attention()
            .unwrap()
            .is_empty());
        let current = store.schema_proposal(&input.proposal.proposal_id).unwrap();
        assert_eq!(current.reviews, [Box::new(retained.clone())]);
        assert_ne!(current.basis.options_digest, retained.basis.options_digest);
        // Exact recovery retains history without re-authorizing the now-inapplicable correction.
        assert_eq!(
            store
                .approve_schema_proposal(&approval, Timestamp::from_millis(6))
                .unwrap(),
            retained
        );
        let invalid = signed_review(
            &human,
            &current,
            true,
            ekr_core::EventId::mint(),
            b"The correction is no longer applicable.",
        );
        let events = store.published_events().unwrap();
        assert!(store
            .approve_schema_proposal(&invalid, Timestamp::from_millis(6))
            .is_err());
        assert_eq!(store.published_events().unwrap(), events);
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding)
            .unwrap();
        reopened.set_full_replay(true);
        assert_eq!(
            reopened
                .schema_proposal(&input.proposal.proposal_id)
                .unwrap()
                .reviews,
            [Box::new(retained)]
        );
    }
}
