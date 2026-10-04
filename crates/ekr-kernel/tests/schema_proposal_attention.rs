//! Completed proposals leave the current inbox while their approval and application remain.
#![allow(unused_imports, dead_code)]
include!("support/schema_proposal_fixture.rs");
include!("support/knowledge_learning_fixtures.rs");

#[test]
fn proposal_attention_tracks_completion_and_keeps_unresolved_corrections_and_history() {
    for (sqlite, unresolved) in [(false, false), (true, false), (false, true), (true, true)] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (store, human) = learning_runtime(&path, sqlite, seed());
        let mut proposal = proposal_input(&store);
        if unresolved {
            let question = store
                .read(None)
                .unwrap()
                .dispute_attention()
                .unwrap()
                .remove(0);
            proposal.proposal.corrections.push(Box::new(
                serde_json::from_value(
                    serde_json::json!({"kind":"Unresolved","assertion_id":question.claims[0],
                    "reason":"The available evidence does not decide this ownership question."}),
                )
                .unwrap(),
            ));
            proposal.payload =
                ekr_core::bytes::encode(&serde_json::to_vec(&proposal.proposal).unwrap());
        }
        let (shown, approval) =
            approve_learning(&store, &human, &proposal, b"Approve the exact vocabulary.");
        let approved_question = store.attention().unwrap().into_iter().find(|item| {
            matches!(&item.subject.proposal_id, w::EssPresence::Present(id) if id == &shown.proposal.proposal_id)
        }).unwrap();
        let mut other = proposal_input(&store);
        let mut document = serde_json::to_value(&other.proposal).unwrap();
        document["additions"][0]["value"]["name"] = "AwaitingReview".into();
        other.proposal = serde_json::from_value(document).unwrap();
        other.payload = ekr_core::bytes::encode(&serde_json::to_vec(&other.proposal).unwrap());
        let pending = store
            .submit_schema_proposal(&other, Timestamp::from_millis(4))
            .unwrap();
        let report = store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approval.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(5),
            )
            .unwrap();
        assert_eq!(
            *report.progress,
            if unresolved {
                w::EkrIntegrateApplicationProgress::V3
            } else {
                w::EkrIntegrateApplicationProgress::V0
            }
        );
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding)
            .unwrap();
        reopened.set_full_replay(true);
        let events = reopened.published_events().unwrap();
        let proposals: BTreeSet<_> = reopened
            .attention()
            .unwrap()
            .into_iter()
            .filter_map(|item| match item.subject.proposal_id {
                w::EssPresence::Present(id) => Some(id.0),
                w::EssPresence::Absent => None,
            })
            .collect();
        let mut expected = BTreeSet::from([pending.proposal.proposal_id.0.clone()]);
        if unresolved {
            expected.insert(shown.proposal.proposal_id.0.clone());
        }
        assert_eq!(proposals, expected);
        assert_eq!(
            reopened.attention_item(&approved_question.subject).is_ok(),
            unresolved
        );
        let history = reopened
            .schema_proposal(&shown.proposal.proposal_id)
            .unwrap();
        assert_eq!(history.reviews, [Box::new(approval)]);
        assert!(matches!(history.application, w::EssPresence::Present(_)));
        assert!(!history.receipts.is_empty());
        assert_eq!(reopened.published_events().unwrap(), events);
        assert!(approved_question
            .question
            .contains("integration or renewed review"));
    }
}
