//! Public SDK requests traverse session parsing, CLI dispatch and the actual kernel/providers.
use super::*;
use crate::cli::session::{fixture, InProcess};
use ekr_core::contracts::kernel as m;
use ekr_core::{AssertionId, ContentHash, EvidenceId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Object,
    Predicate, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{EvidenceAddition, GraphOperation, GraphTransaction, SeedDocument};
use ekr_ontology::Value;
use ekr_sdk::knowledge::Knowledge;
use ring::signature::{Ed25519KeyPair, KeyPair};

fn signed(key: &Ed25519KeyPair, intent: serde_json::Value) -> EkrKernelSignedHumanDecision {
    let mut proof: EkrKernelSignedHumanDecision = serde_json::from_value(serde_json::json!({
        "algorithm": "Ed25519", "signature": ekr_core::bytes::encode(&[0;64]), "intent": intent
    }))
    .unwrap();
    let decoded = human_review::proof_from_document(&proof).unwrap();
    proof.signature = ekr_core::bytes::encode(
        key.sign(&human_review::signing_bytes(&decoded.intent).unwrap())
            .as_ref(),
    );
    proof
}

fn claims(runtime: &Runtime, actor: ekr_core::AgentId) {
    let mut transaction = GraphTransaction {
        id: TransactionId::mint(),
        proposer: actor,
        operations: vec![],
        evidence: Default::default(),
        schema_version: None,
    };
    for value in ["First", "Second"] {
        let payload = format!("Fixture legal name is {value}").into_bytes();
        let evidence = Evidence {
            id: EvidenceId::mint(),
            source: EvidenceSource::HumanStatement { identity: None },
            content_hash: ContentHash::of_bytes(&payload),
            extracted_by: actor,
            observed_at: Timestamp::EPOCH,
            confidence: Confidence::CERTAIN,
        };
        let assertion = Assertion {
            id: AssertionId::mint(),
            root_id: "00000000-0000-4000-8000-000000000002".parse().unwrap(),
            subject: Subject::Node("00000000-0000-4000-8000-000000000303".parse().unwrap()),
            predicate: Predicate::Property("00000000-0000-4000-8000-000000000801".parse().unwrap()),
            object: Object::Value(Value::String(value.into())),
            evidence: [evidence.id].into_iter().collect(),
            proposed_by: actor,
            assessment: Assessment::Proposed,
            lifecycle: AssertionLifecycle::Active,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        };
        transaction.evidence.insert(evidence.id);
        transaction
            .operations
            .push(GraphOperation::AddEvidence(Box::new(EvidenceAddition {
                evidence,
                payload,
            })));
        transaction
            .operations
            .push(GraphOperation::AddAssertion(Box::new(assertion)));
    }
    #[derive(serde::Serialize)]
    struct Document<'a> {
        format: &'a str,
        transaction: &'a GraphTransaction,
    }
    let bytes = serde_yaml_ng::to_string(&Document {
        format: "ekr.transaction-document/2",
        transaction: &transaction,
    })
    .unwrap();
    runtime
        .propose(bytes.as_bytes(), actor, || Timestamp::EPOCH)
        .unwrap();
    let result = runtime
        .validate(transaction.id, RevisionNumber::SEED, || Timestamp::EPOCH)
        .unwrap();
    assert!(
        matches!(result, ekr_kernel::ValidationCommandResult::Validated(_)),
        "{result:?}"
    );
    let result = runtime
        .commit(transaction.id, actor, || Timestamp::EPOCH)
        .unwrap();
    assert!(
        matches!(result, ekr_kernel::CommitCommandResult::Committed(_)),
        "{result:?}"
    );
}

#[test]
fn sdk_answer_and_history_use_real_session_signature_validation_and_replay() {
    for backend in fixture::BACKENDS {
        let directory = tempfile::tempdir().unwrap();
        let store = fixture::seeded(directory.path(), backend, "answer");
        let runtime = store.open().unwrap();
        let seed = SeedDocument::from_yaml(
            &std::fs::read_to_string(directory.path().join("seed.yaml")).unwrap(),
        )
        .unwrap();
        let seeded = runtime.seed(seed, || panic!("seed retry")).unwrap();
        claims(&runtime, store.host.context.operator);
        let key = Ed25519KeyPair::from_seed_unchecked(&[29; 32]).unwrap();
        let audience = serde_json::json!({"tenant": store.host.tenant, "seed_anchor": seeded.seed_hash.to_string()});
        let key_digest = human_review::digest(key.public_key().as_ref()).to_string();
        let wire_policy: EkrKernelReviewerTrustPolicy = serde_json::from_value(serde_json::json!({
            "format": "ekr.reviewer-trust/1", "audience": audience,
            "keys": [{"key_digest": key_digest, "algorithm": "Ed25519", "public_key": ekr_core::bytes::encode(key.public_key().as_ref()),
                "operator": {"actor": store.host.context.operator.to_string(), "authentication_subject": "fixture-human"},
                "scopes": ["AnswerAttention", "UpgradeAuthority"]}]
        })).unwrap();
        let policy = human_review::policy_from_document(&wire_policy).unwrap();
        let digest =
            human_review::digest(&human_review::policy_bytes(&policy).unwrap()).to_string();
        let binding = m::TrustedReviewHostBinding {
            audience: policy.audience.clone(),
            reviewer_policy_digest: m::ContentHash(digest.clone()),
        };
        let runtime = runtime.with_review_authority(binding.clone()).unwrap();
        let retry = std::cell::Cell::new(false);
        let now = || {
            assert!(!retry.get(), "completed retries must not call the clock");
            Timestamp::from_millis(2)
        };
        let mut transport = InProcess::new(store.clone(), runtime, &now);
        let mut knowledge = Knowledge::new(&mut transport);
        let preview = knowledge.preview_upgrade(&wire_policy).unwrap();
        let statement = b"reviewed the ownership of this fixture's claims";
        let intent = |target| {
            serde_json::json!({
                "format": "ekr.human-decision/1", "decision_id": ekr_core::EventId::mint().to_string(),
                "audience": audience, "reviewer_policy_digest": digest, "signer_key_digest": key_digest,
                "statement_digest": human_review::digest(statement).to_string(), "target": target
            })
        };
        let proof = signed(
            &key,
            intent(serde_json::json!({"kind": "UpgradeAuthority", "value": {
                "preview_digest": preview.preview_digest, "reviewer_policy_digest": digest
            }})),
        );
        knowledge
            .apply_upgrade(&EkrKernelAuthorityUpgradeApplication {
                preview: Box::new(preview),
                policy: Box::new(wire_policy),
                human_proof: Box::new(proof),
                statement: ekr_core::bytes::encode(statement),
            })
            .unwrap();
        let questions = knowledge.attention().unwrap();
        assert_eq!(questions.len(), 1);
        let question = &questions[0];
        let EssPresence::Present(dispute_id) = &question.subject.dispute_id else {
            panic!("dispute identity")
        };
        let correction: EkrKernelClaimCorrection = serde_json::from_value(serde_json::json!({
            "kind": "Choose", "assertion_id": question.claims[0], "reason": "reviewed both original source statements"
        })).unwrap();
        let semantic_correction = m::ClaimCorrection {
            kind: m::ClaimCorrectionKind::Choose,
            assertion_id: ekr_core::contracts::graph::AssertionId(
                ekr_core::contracts::primitives::Uuid(question.claims[0].0.clone()),
            ),
            reason: correction.reason.clone(),
            valid_from: None,
            valid_to: None,
        };
        let proof = signed(
            &key,
            intent(serde_json::json!({"kind": "AnswerAttention", "value": {
                "dispute_id": dispute_id, "basis": question.basis,
                "corrections_digest": human_review::digest(&human_review::corrections_bytes(&[semantic_correction]).unwrap()).to_string()
            }})),
        );
        let input = EkrKernelAttentionAnswerApplication {
            human_proof: Box::new(proof),
            dispute_id: dispute_id.clone(),
            basis: question.basis.clone(),
            corrections: vec![Box::new(correction)],
            statement: ekr_core::bytes::encode(statement),
        };
        human_review::answer_from_document(&input).unwrap();
        let mut malformed = input.clone();
        malformed.statement = "not base64!".into();
        assert!(matches!(
            knowledge.answer_attention(&malformed),
            Err(ekr_sdk::read::ReadError::Refused { .. })
        ));
        malformed = input.clone();
        malformed.corrections[0].reason.clear();
        assert!(matches!(
            knowledge.answer_attention(&malformed),
            Err(ekr_sdk::read::ReadError::Refused { .. })
        ));
        assert!(knowledge.answer_history(None).unwrap().is_empty());
        let mut forged = input.clone();
        forged.human_proof.signature = ekr_core::bytes::encode(&[0; 64]);
        assert!(matches!(
            knowledge.answer_attention(&forged),
            Err(ekr_sdk::read::ReadError::Refused { .. })
        ));
        assert!(knowledge.answer_history(None).unwrap().is_empty());
        assert_eq!(knowledge.attention().unwrap(), questions);
        let receipt = knowledge.answer_attention(&input).unwrap();
        assert_eq!(*receipt.outcome, EkrKernelAnswerOutcome::V2);
        assert!(receipt.remaining.is_empty());
        retry.set(true);
        assert_eq!(knowledge.answer_attention(&input).unwrap(), receipt);
        assert!(knowledge.attention().unwrap().is_empty());
        let history = knowledge.answer_history(None).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(*history[0].receipt, receipt);
        assert_eq!(knowledge.answer_history(Some(dispute_id)).unwrap(), history);
        let unknown = EkrKernelDisputeId(AssertionId::mint().to_string());
        assert!(knowledge.answer_history(Some(&unknown)).unwrap().is_empty());
        let mut changed = input.clone();
        changed.statement = ekr_core::bytes::encode(b"unreviewed replacement statement");
        assert!(matches!(
            knowledge.answer_attention(&changed),
            Err(ekr_sdk::read::ReadError::Refused { .. })
        ));
        transport.close();
        let mut reopened = store
            .open()
            .unwrap()
            .with_review_authority(binding)
            .unwrap();
        reopened.set_full_replay(true);
        let head = reopened.head().unwrap();
        let html = String::from_utf8(crate::cli::inbox::render(&reopened).unwrap()).unwrap();
        assert!(html.contains("No unresolved knowledge questions."));
        assert!(html.contains("Answer history"));
        assert!(html.contains("fixture-human"));
        assert!(html.contains(&format!("answer-{}", receipt.answer_id.0)));
        assert!(html.contains("Human statement evidence"));
        assert!(!html.contains("<form"));
        assert_eq!(reopened.head().unwrap(), head);
        if let Ok(directory) = std::env::var("EKR_INBOX_CAPTURE_DIR") {
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                std::path::Path::new(&directory).join(format!("answer-{backend:?}.html")),
                &html,
            )
            .unwrap();
        }
        let mut transport = InProcess::new(store, reopened, &now);
        let mut knowledge = Knowledge::new(&mut transport);
        assert_eq!(knowledge.answer_history(None).unwrap(), history);
        assert_eq!(knowledge.answer_attention(&input).unwrap(), receipt);
        assert!(knowledge.attention().unwrap().is_empty());
        let explained = ekr_sdk::read::Reader::new(&mut transport)
            .explain_documents(question.claims[0].0.parse().unwrap())
            .unwrap();
        assert!(explained.links.iter().any(|link| matches!(link, ekr_sdk::read::ExplanationLink::HumanAnswer(answer) if *answer.record == history[0])));
        assert!(explained.links.iter().any(|link| matches!(link, ekr_sdk::read::ExplanationLink::Evidence(evidence) if evidence.payload.as_deref() == Some(&input.statement))));
        transport.close();
    }
}

#[test]
fn answer_json_is_bounded_and_null_bounds_are_refused_before_kernel_admission() {
    use ekr_sdk::transport::{Request, Transport};
    for backend in fixture::BACKENDS {
        let directory = tempfile::tempdir().unwrap();
        let store = fixture::seeded(directory.path(), backend, "bounded-answer");
        let runtime = store.open().unwrap();
        let now = || panic!("invalid JSON must not reach publication");
        let mut transport = InProcess::new(store, runtime, &now);
        for document in ["{".to_owned(), " ".repeat(8 * 1024 * 1024 + 1)] {
            let reply = transport
                .request(&Request::new(["attention", "answer", "-"]).with_stdin(document))
                .unwrap();
            assert_eq!(reply.exit, 2, "{}", reply.stderr);
            assert!(reply.stderr.contains("ekr.kernel.KnowledgeRefused"));
        }
        let null_bound = serde_json::json!({"kind": "CorrectTime", "assertion_id": AssertionId::mint().to_string(), "reason": "reviewed time", "valid_from": null});
        assert!(serde_json::from_value::<EkrKernelClaimCorrection>(null_bound).is_err());
        assert!(Knowledge::new(&mut transport)
            .answer_history(None)
            .unwrap()
            .is_empty());
        transport.close();
    }
}
