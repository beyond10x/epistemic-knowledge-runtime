//! Real signed capabilities exercise the schema-specific deterministic operation boundary.
// Shared fixture helpers also serve unrelated provider review tests.
#![allow(unused_imports, dead_code)]
use crate as ekr_kernel;
include!("../../tests/support/schema_proposal_fixture.rs");

struct Fixture {
    _directory: tempfile::TempDir,
    read: crate::VerifiedRead,
    human: Human,
    components: Vec<Vec<AssertionId>>,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let store = runtime(&directory.path().join("store"), false);
        let mut document = seed();
        let mut second = document.graph.nodes.values().next().unwrap().clone();
        second.id = NodeId::mint();
        second.canonical_name = "Second project".into();
        for mut claim in document
            .graph
            .assertions
            .values()
            .cloned()
            .collect::<Vec<_>>()
        {
            claim.id = AssertionId::mint();
            claim.subject = Subject::Node(second.id);
            document.graph.assertions.insert(claim.id, claim);
        }
        document.graph.nodes.insert(second.id, second);
        let seeded = store.seed(document, || Timestamp::EPOCH).unwrap();
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
        let components: Vec<Vec<AssertionId>> = read
            .dispute_attention()
            .unwrap()
            .into_iter()
            .map(|item| item.claims.iter().map(|id| id.0.parse().unwrap()).collect())
            .collect::<Vec<_>>();
        assert_eq!(components.len(), 2);
        assert!(components.iter().all(|claims| claims.len() == 2));
        Self {
            _directory: directory,
            read,
            human,
            components,
        }
    }
    fn correction(&self, component: usize, kind: &str) -> serde_json::Value {
        let mut correction = serde_json::json!({"kind":kind,"assertion_id":self.components[component][0],"reason":"reviewed exact source and competing claims"});
        if kind == "CorrectTime" {
            correction["valid_from"] = "2000-01-01T00:00:00Z".into();
        }
        correction
    }
    fn proposal(
        &self,
        corrections: Vec<serde_json::Value>,
    ) -> w::EkrIntegrateRetainedSchemaProposal {
        let document: w::EkrIntegrateSchemaProposalDocument =
            serde_json::from_value(serde_json::json!({
                "proposal_id":NodeId::mint(),"base_schema":self.read.graph.ontology.version().id,
                "observations":[],"sources":[],"evidence":[],"additions":[],"mappings":[],
                "corrections":corrections,"explanation":"Exact signed correction operation fixture"
            }))
            .unwrap();
        let bytes = serde_json::to_vec_pretty(&document).unwrap();
        w::EkrIntegrateRetainedSchemaProposal {
            proposal: Box::new(document),
            proposal_digest: Box::new(w::EkrKernelContentHash(
                ContentHash::of_bytes(&bytes).to_string(),
            )),
            payload: ekr_core::bytes::encode(&bytes),
        }
    }
    fn verify(
        &self,
        proposal: &w::EkrIntegrateRetainedSchemaProposal,
        approve: bool,
    ) -> review::VerifiedDecision {
        let basis =
            review::basis_from_document(&self.read.dispute_attention().unwrap()[0].basis).unwrap();
        let target = m::SchemaReviewTarget {
            proposal_id: ekr_core::contracts::integrate::SchemaProposalId(Uuid(
                proposal.proposal.proposal_id.0.clone(),
            )),
            proposal_digest: m::ContentHash(proposal.proposal_digest.0.clone()),
            basis,
        };
        let target = if approve {
            m::HumanDecisionTarget::ApproveSchemaProposal(target)
        } else {
            m::HumanDecisionTarget::RejectSchemaProposal(target)
        };
        let intent = m::HumanDecisionIntent {
            format: m::HumanDecisionFormat::HumanDecision1,
            decision_id: Uuid(ekr_core::EventId::mint().to_string()),
            audience: self.human.binding.audience.clone(),
            reviewer_policy_digest: self.human.binding.reviewer_policy_digest.clone(),
            signer_key_digest: self.human.policy.keys[0].key_digest.clone(),
            target: target.clone(),
            statement_digest: hash(b"reviewed schema correction"),
            expected_previous_decision: None,
        };
        let proof = m::SignedHumanDecision {
            signature: self
                .human
                .key
                .sign(&review::signing_bytes(&intent).unwrap())
                .as_ref()
                .to_vec(),
            intent,
            algorithm: m::ReviewSignatureAlgorithm::Ed25519,
        };
        review::Reviewer::from_host(&self.human.binding, self.human.policy.clone())
            .unwrap()
            .verify(&proof, &target, b"reviewed schema correction", None)
            .unwrap()
    }
}
fn replacement(previous: AssertionId, new: AssertionId) -> m::ClaimReplacement {
    m::ClaimReplacement {
        previous: ekr_core::contracts::graph::AssertionId(Uuid(previous.to_string())),
        replacement: ekr_core::contracts::graph::AssertionId(Uuid(new.to_string())),
    }
}
fn withdrawals(operations: &[GraphOperation]) -> BTreeSet<AssertionId> {
    operations
        .iter()
        .filter_map(|operation| match operation {
            GraphOperation::RetractAssertion(row) => Some(row.assertion),
            _ => None,
        })
        .collect()
}

#[test]
fn schema_choose_and_retract_use_actual_dispute_competitors() {
    let fixture = Fixture::new();
    for (kind, expected) in [
        ("Choose", fixture.components[0][1]),
        ("Retract", fixture.components[0][0]),
    ] {
        let proposal = fixture.proposal(vec![fixture.correction(0, kind)]);
        let operations = fixture
            .verify(&proposal, true)
            .schema_correction_operations(&fixture.read, &proposal, &[], EvidenceId::mint())
            .unwrap();
        assert_eq!(operations.len(), 1);
        assert_eq!(withdrawals(&operations), [expected].into());
    }
}
#[test]
fn schema_time_preserves_source_evidence_and_uses_exact_fresh_replacement() {
    let fixture = Fixture::new();
    let proposal = fixture.proposal(vec![fixture.correction(0, "CorrectTime")]);
    let chosen = fixture.components[0][0];
    let new = AssertionId::mint();
    let statement = EvidenceId::mint();
    let operations = fixture
        .verify(&proposal, true)
        .schema_correction_operations(
            &fixture.read,
            &proposal,
            &[replacement(chosen, new)],
            statement,
        )
        .unwrap();
    assert_eq!(operations.len(), 2);
    assert_eq!(withdrawals(&operations), [chosen].into());
    let GraphOperation::AddAssertion(added) = &operations[1] else {
        panic!("missing replacement")
    };
    let original = &fixture.read.graph.assertions[&chosen];
    assert_eq!(added.id, new);
    assert_eq!(added.predicate, original.predicate);
    assert_eq!(added.root_id, original.root_id);
    assert_eq!(
        added.object,
        Object::Value(match &original.object {
            Object::Value(value) => value.clone().into(),
            _ => panic!("fixture property"),
        })
    );
    let mut evidence: BTreeSet<_> = original.evidence.iter().map(|id| id.id()).collect();
    evidence.insert(statement);
    assert_eq!(added.evidence, evidence);
    assert_eq!(added.proposed_by, context().operator);
    assert_eq!(added.assessment, Assessment::Proposed);
    assert_eq!(added.lifecycle, AssertionLifecycle::Active);
    assert_ne!(added.valid_time, original.valid_time);
}
#[test]
fn schema_multi_dispute_effects_are_complete_and_stably_ordered() {
    let fixture = Fixture::new();
    let corrections = vec![
        fixture.correction(0, "Choose"),
        fixture.correction(1, "CorrectTime"),
    ];
    let proposal = fixture.proposal(corrections.clone());
    let statement = EvidenceId::mint();
    let replacements = vec![replacement(fixture.components[1][0], AssertionId::mint())];
    let operations = fixture
        .verify(&proposal, true)
        .schema_correction_operations(&fixture.read, &proposal, &replacements, statement)
        .unwrap();
    assert_eq!(operations.len(), 3);
    assert_eq!(
        withdrawals(&operations),
        [fixture.components[0][1], fixture.components[1][0]].into()
    );
    let reversed = fixture.proposal(corrections.into_iter().rev().collect());
    let again = fixture
        .verify(&reversed, true)
        .schema_correction_operations(&fixture.read, &reversed, &replacements, statement)
        .unwrap();
    assert_eq!(operations, again);
}
#[test]
fn schema_unresolved_refuses_the_entire_correction_step() {
    let fixture = Fixture::new();
    for corrections in [
        vec![fixture.correction(0, "Unresolved")],
        vec![
            fixture.correction(0, "Choose"),
            fixture.correction(1, "Unresolved"),
        ],
    ] {
        let proposal = fixture.proposal(corrections);
        let refusal = fixture
            .verify(&proposal, true)
            .schema_correction_operations(&fixture.read, &proposal, &[], EvidenceId::mint())
            .unwrap_err();
        assert_eq!(refusal.code, "schema-correction-unresolved");
    }
}
#[test]
fn schema_proof_target_and_immutable_payload_are_exact() {
    let fixture = Fixture::new();
    let proposal = fixture.proposal(vec![fixture.correction(0, "Choose")]);
    let approved = fixture.verify(&proposal, true);
    assert!(fixture
        .verify(&proposal, false)
        .schema_correction_operations(&fixture.read, &proposal, &[], EvidenceId::mint())
        .is_err());
    let answer = answer(
        &fixture.human,
        &fixture.read,
        m::ClaimCorrectionKind::Choose,
    );
    let attention =
        review::Reviewer::from_host(&fixture.human.binding, fixture.human.policy.clone())
            .unwrap()
            .verify(
                &answer.human_proof,
                &answer.human_proof.intent.target,
                &answer.statement,
                None,
            )
            .unwrap();
    assert!(attention
        .schema_correction_operations(&fixture.read, &proposal, &[], EvidenceId::mint())
        .is_err());
    assert!(approved
        .correction_operations(&fixture.read, &answer.corrections, &[], EvidenceId::mint())
        .is_err());
    for case in 0..4 {
        let mut changed = proposal.clone();
        match case {
            0 => changed.proposal_digest.0 = ContentHash::of_bytes(b"different").to_string(),
            1 => changed.proposal.proposal_id.0 = NodeId::mint().to_string(),
            2 => changed.payload = "AA==".into(),
            3 => {
                changed.proposal.corrections[0].reason = "changed exact bytes".into();
                let bytes = serde_json::to_vec(&changed.proposal).unwrap();
                changed.payload = ekr_core::bytes::encode(&bytes);
                changed.proposal_digest.0 = ContentHash::of_bytes(&bytes).to_string();
            }
            _ => unreachable!(),
        }
        assert!(
            approved
                .schema_correction_operations(&fixture.read, &changed, &[], EvidenceId::mint())
                .is_err(),
            "accepted {case}"
        );
    }
}
#[test]
fn schema_replacements_have_global_exact_fresh_distinct_coverage() {
    let fixture = Fixture::new();
    let proposal = fixture.proposal(vec![
        fixture.correction(0, "CorrectTime"),
        fixture.correction(1, "CorrectTime"),
    ]);
    let approved = fixture.verify(&proposal, true);
    let a = fixture.components[0][0];
    let b = fixture.components[1][0];
    let x = AssertionId::mint();
    let y = AssertionId::mint();
    let valid = vec![replacement(a, x), replacement(b, y)];
    assert_eq!(
        approved
            .schema_correction_operations(&fixture.read, &proposal, &valid, EvidenceId::mint())
            .unwrap()
            .len(),
        4
    );
    for replacements in [
        vec![],
        vec![replacement(a, x)],
        vec![replacement(a, x), replacement(b, x)],
        vec![replacement(a, x), replacement(a, y)],
        vec![replacement(a, b), replacement(b, y)],
        vec![
            replacement(a, x),
            replacement(b, y),
            replacement(fixture.components[0][1], AssertionId::mint()),
        ],
    ] {
        assert!(approved
            .schema_correction_operations(
                &fixture.read,
                &proposal,
                &replacements,
                EvidenceId::mint()
            )
            .is_err());
    }
}
#[test]
fn schema_corrections_require_distinct_current_applicable_claims() {
    let fixture = Fixture::new();
    let valid = fixture.correction(0, "Choose");
    for corrections in [
        vec![valid.clone(), valid.clone()],
        vec![
            serde_json::json!({"kind":"Retract","assertion_id":AssertionId::mint(),"reason":"unknown"}),
        ],
        vec![
            serde_json::json!({"kind":"Choose","assertion_id":fixture.components[0][0],"reason":"   "}),
        ],
    ] {
        let proposal = fixture.proposal(corrections);
        assert!(fixture
            .verify(&proposal, true)
            .schema_correction_operations(&fixture.read, &proposal, &[], EvidenceId::mint())
            .is_err());
    }
}
