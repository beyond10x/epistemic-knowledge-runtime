// Shared external-human fixtures for proposal review and application tests.
include!("attention_answer_fixture.rs");
use ekr_core::contract_data as w;
use ekr_kernel::Runtime;

fn runtime(path: &std::path::Path, sqlite: bool) -> Runtime {
    if sqlite {
        Runtime::sqlite(path, "upgrade-fixture", context(), anchor()).unwrap()
    } else {
        Runtime::file(path, "upgrade-fixture", context(), anchor()).unwrap()
    }
}
fn proposal_input(runtime: &Runtime) -> w::EkrIntegrateSchemaProposalImport {
    let read = runtime.read(None).unwrap();
    let evidence = read.graph.evidence.keys().next().unwrap();
    let proposal = serde_json::json!({
        "proposal_id":ekr_core::NodeId::mint(),"base_schema":read.graph.ontology.version().id,
        "observations":[],"sources":[],"evidence":[evidence],
        "additions":[{"kind":"DefineType","value":{"name":"ReviewVocabulary","parents":[],"abstract_type":false,"properties":[]}}],
        "mappings":[],"corrections":[],"explanation":"A supported additive proposal requiring exact external review."
    });
    serde_json::from_value(serde_json::json!({"payload":ekr_core::bytes::encode(&serde_json::to_vec_pretty(&proposal).unwrap()),"proposal":proposal})).unwrap()
}
fn schema_human(seed: ContentHash) -> Human {
    let mut human = Human::new(seed);
    human.policy.keys[0].scopes = vec![
        m::HumanDecisionScope::AnswerAttention,
        m::HumanDecisionScope::ApproveSchemaProposal,
        m::HumanDecisionScope::RejectSchemaProposal,
        m::HumanDecisionScope::UpgradeAuthority,
    ];
    human.binding.reviewer_policy_digest = hash(&review::policy_bytes(&human.policy).unwrap());
    human
}
fn signed_review(
    human: &Human,
    shown: &w::EkrIntegrateSchemaProposalRead,
    approve: bool,
    decision: ekr_core::EventId,
    statement: &[u8],
) -> w::EkrIntegrateSchemaProposalReviewApplication {
    let basis = m::ReviewBasis {
        observed_revision: m::RevisionNumber(shown.basis.observed_revision.0.as_i64().unwrap()),
        evidence_digest: m::ContentHash(shown.basis.evidence_digest.0.clone()),
        options_digest: m::ContentHash(shown.basis.options_digest.0.clone()),
        effects_digest: m::ContentHash(shown.basis.effects_digest.0.clone()),
    };
    let target = m::SchemaReviewTarget {
        proposal_id: ekr_core::contracts::integrate::SchemaProposalId(Uuid(
            shown.proposal.proposal_id.0.clone(),
        )),
        proposal_digest: m::ContentHash(shown.proposal_digest.0.clone()),
        basis,
    };
    let previous = match &shown.expected_previous_decision {
        w::EssPresence::Absent => None,
        w::EssPresence::Present(hash) => Some(m::ContentHash(hash.0.clone())),
    };
    let intent = m::HumanDecisionIntent {
        format: m::HumanDecisionFormat::HumanDecision1,
        decision_id: Uuid(decision.to_string()),
        audience: human.binding.audience.clone(),
        reviewer_policy_digest: human.binding.reviewer_policy_digest.clone(),
        signer_key_digest: human.policy.keys[0].key_digest.clone(),
        target: if approve {
            m::HumanDecisionTarget::ApproveSchemaProposal(target)
        } else {
            m::HumanDecisionTarget::RejectSchemaProposal(target)
        },
        statement_digest: hash(statement),
        expected_previous_decision: previous,
    };
    let signature = human.key.sign(&review::signing_bytes(&intent).unwrap());
    let mut proof = serde_json::json!({"algorithm":"Ed25519","signature":ekr_core::bytes::encode(signature.as_ref()),
        "intent":{"format":"ekr.human-decision/1","decision_id":decision,"audience":{"tenant":intent.audience.tenant,"seed_anchor":intent.audience.seed_anchor.0},
        "reviewer_policy_digest":intent.reviewer_policy_digest.0,"signer_key_digest":intent.signer_key_digest.0,
        "target":{"kind":if approve {"ApproveSchemaProposal"} else {"RejectSchemaProposal"},"value":{"proposal_id":shown.proposal.proposal_id,"proposal_digest":shown.proposal_digest,"basis":shown.basis}},
        "statement_digest":intent.statement_digest.0}});
    if let Some(previous) = intent.expected_previous_decision {
        proof["intent"]["expected_previous_decision"] = previous.0.into();
    }
    serde_json::from_value(serde_json::json!({"human_proof":proof,"proposal_id":shown.proposal.proposal_id,
        "proposal_digest":shown.proposal_digest,"basis":shown.basis,"statement":ekr_core::bytes::encode(statement)})).unwrap()
}
