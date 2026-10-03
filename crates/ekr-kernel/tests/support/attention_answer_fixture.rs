include!("authority_review_fixture.rs");

fn answer(
    human: &Human,
    read: &ekr_kernel::VerifiedRead,
    kind: m::ClaimCorrectionKind,
) -> m::AttentionAnswerApplication {
    let current = read.dispute_attention().unwrap().remove(0);
    let ekr_core::contract_data::EssPresence::Present(id) = &current.subject.dispute_id else { panic!("dispute identity") };
    let dispute_id = m::DisputeId(Uuid(id.0.clone()));
    let basis = m::ReviewBasis {
        observed_revision: m::RevisionNumber(current.basis.observed_revision.0.as_i64().unwrap()),
        evidence_digest: m::ContentHash(current.basis.evidence_digest.0.clone()),
        options_digest: m::ContentHash(current.basis.options_digest.0.clone()),
        effects_digest: m::ContentHash(current.basis.effects_digest.0.clone()),
    };
    let corrections = vec![m::ClaimCorrection {
        kind,
        assertion_id: ekr_core::contracts::graph::AssertionId(Uuid(current.claims[0].0.clone())),
        valid_from: (kind == m::ClaimCorrectionKind::CorrectTime)
            .then(|| ekr_core::contracts::primitives::Timestamp("2000-01-01T00:00:00Z".into())),
        valid_to: None,
        reason: "human reviewed the retained source evidence".into(),
    }];
    let target = m::HumanDecisionTarget::AnswerAttention(m::AttentionAnswerTarget {
        dispute_id: dispute_id.clone(),
        basis: basis.clone(),
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
        dispute_id,
        basis,
        corrections,
        statement: b"reviewed answer".to_vec(),
    }
}
