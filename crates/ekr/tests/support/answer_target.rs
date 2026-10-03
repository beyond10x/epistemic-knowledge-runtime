//! Actual signed-answer commands and retained projections for authored ESS scenarios.
use super::*;
use ekr_kernel::{CommitCommandResult, GraphOperation, GraphTransaction, ValidationCommandResult};

const STATEMENT: &[u8] = b"reviewed fixture answer";
const REASON: &str = "human reviewed the retained source evidence";

pub(super) fn signed(
    runtime: &Runtime,
    human: &fixture::Human,
    decision: u64,
) -> Result<Value, TargetError> {
    let item = runtime
        .attention()
        .map_err(|e| unavailable("reading review basis", e))?
        .into_iter()
        .next()
        .ok_or_else(|| unavailable("preparing answer fixture", "no dispute"))?;
    let mut application: wire::EkrKernelAttentionAnswerApplication = serde_json::from_value(json!({
        "dispute_id":item.subject.dispute_id,"basis":item.basis,
        "corrections":[{"kind":"Choose","assertion_id":fixture::id::<ekr_core::AssertionId>(401).to_string(),"reason":REASON}],
        "statement":ekr_core::bytes::encode(STATEMENT),
        "human_proof":{"algorithm":"Ed25519","signature":ekr_core::bytes::encode(&[0;64]),"intent":{
            "format":"ekr.human-decision/1","decision_id":fixture::id::<ekr_core::EventId>(decision).to_string(),
            "audience":{"tenant":fixture::TENANT,"seed_anchor":human.binding.audience.seed_anchor.0},
            "reviewer_policy_digest":human.binding.reviewer_policy_digest.0,
            "signer_key_digest":human.key_digest(),"statement_digest":human_review::digest(STATEMENT).to_string(),
            "target":{"kind":"AnswerAttention","value":{"dispute_id":item.subject.dispute_id,"basis":item.basis,
                "corrections_digest":ContentHash::of_bytes(b"placeholder").to_string()}}}}
    })).map_err(|e| unavailable("decoding generated answer fixture", e))?;
    let semantic = human_review::answer_from_document(&application)
        .map_err(|e| unavailable("encoding corrections", e.reason))?;
    let corrections_digest = human_review::digest(
        &human_review::corrections_bytes(&semantic.corrections)
            .map_err(|e| unavailable("encoding corrections", e.reason))?,
    )
    .to_string();
    let mut proof = value(&application.human_proof)?;
    proof["intent"]["target"]["value"]["corrections_digest"] = json!(corrections_digest);
    application.human_proof =
        Box::new(serde_json::from_value(proof).map_err(|e| unavailable("binding corrections", e))?);
    human.sign(&mut application.human_proof);
    let mut output = value(application)?;
    output["human_proof"]["intent"]["format"] = json!("HumanDecision1");
    Ok(output)
}

/// Review before an ordinary intervening transaction; sign a second, renewed review afterward.
/// Fixture setup chooses inputs only. Scenario assertions decide the required answer/outcome.
pub(super) fn prepare(
    runtime: &Runtime,
    human: &fixture::Human,
    scenario: &str,
) -> Result<BTreeMap<String, Node>, TargetError> {
    let preview = runtime
        .preview_upgrade(&human.policy)
        .map_err(|e| unavailable("previewing fixture upgrade", e))?;
    let proof = human_review::proof_from_document(&human.proof(&preview))
        .map_err(|e| unavailable("decoding fixture proof", e.reason))?;
    runtime
        .apply_upgrade(&preview, &human.policy, &proof, fixture::STATEMENT, || {
            Timestamp::from_millis(1)
        })
        .map_err(|e| unavailable("upgrading answer fixture", e))?;
    let old = signed(runtime, human, 801)?;
    let changed = scenario.ends_with("changed-answer-basis-requires-review");
    advance(runtime, changed)?;
    let fresh = signed(runtime, human, 802)?;
    let mut supplied = BTreeMap::new();
    for (prefix, input) in [("reviewed", old), ("renewed", fresh)] {
        for (name, value) in input.as_object().unwrap() {
            supplied.insert(
                format!("{prefix}-{}", name.replace('_', "-")),
                node(value.clone())?,
            );
        }
    }
    Ok(supplied)
}

pub(super) fn advance(runtime: &Runtime, changed: bool) -> Result<(), TargetError> {
    let read = runtime
        .read(None)
        .map_err(|e| unavailable("reading intervening basis", e))?;
    let operations = if changed {
        let mut assertion = read.seed_input.graph.assertions[&fixture::id(401)].clone();
        assertion.id = fixture::id(405);
        assertion.object = ekr_graph::Object::Value(ekr_ontology::Value::String("amber".into()));
        assertion.assessment = Assessment::Proposed;
        assertion.evidence = [fixture::id(600)].into_iter().collect();
        let mut evidence = read.graph.evidence[&fixture::id(500)].clone();
        evidence.id = fixture::id(600);
        let payload = b"New independent health observation".to_vec();
        evidence.content_hash = ContentHash::of_bytes(&payload);
        vec![
            GraphOperation::AddEvidence(Box::new(ekr_kernel::EvidenceAddition {
                evidence,
                payload,
            })),
            GraphOperation::AddAssertion(Box::new(assertion)),
        ]
    } else {
        vec![GraphOperation::CreateNode(ekr_kernel::NodeDraft {
            id: fixture::id(210),
            root_id: read.graph.root.id,
            type_id: fixture::id(100),
            canonical_name: "Unrelated project".into(),
            properties: BTreeMap::new(),
            aliases: vec![],
        })]
    };
    let tx = GraphTransaction {
        id: fixture::id(900),
        proposer: fixture::context().operator,
        operations,
        evidence: if changed {
            [fixture::id(600)].into_iter().collect()
        } else {
            Default::default()
        },
        schema_version: None,
    };
    #[derive(serde::Serialize)]
    struct Document<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let bytes = serde_yaml_ng::to_string(&Document {
        format: "ekr.transaction-document/1",
        transaction: &tx,
    })
    .unwrap();
    runtime
        .propose(bytes.as_bytes(), fixture::context().operator, || {
            Timestamp::from_millis(2)
        })
        .map_err(|e| unavailable("proposing intervening facts", e))?;
    let validated = runtime
        .validate(tx.id, read.root.revision, || Timestamp::from_millis(3))
        .map_err(|e| unavailable("validating intervening facts", e))?;
    if !matches!(validated, ValidationCommandResult::Validated(_)) {
        return Err(unavailable(
            "validating intervening facts",
            format!("{validated:?}"),
        ));
    }
    let committed = runtime
        .commit(tx.id, fixture::context().operator, || {
            Timestamp::from_millis(4)
        })
        .map_err(|e| unavailable("committing intervening facts", e))?;
    if !matches!(committed, CommitCommandResult::Committed(_)) {
        return Err(unavailable(
            "committing intervening facts",
            "unexpected outcome",
        ));
    }
    Ok(())
}

pub(super) fn execute(
    target: &UpgradeTarget,
    runtime: &Runtime,
    request: &SemanticCommandRequest,
) -> Result<SemanticCommandResult, TargetError> {
    let mut input = json_input(&Node::Map(request.input.clone()))?;
    if input["human_proof"]["intent"]["format"] != "HumanDecision1" {
        return Err(unavailable(
            "decoding reviewed answer",
            "unknown semantic format",
        ));
    }
    input["human_proof"]["intent"]["format"] = json!("ekr.human-decision/1");
    let document: wire::EkrKernelAttentionAnswerApplication = serde_json::from_value(input)
        .map_err(|e| unavailable("decoding generated answer application", e))?;
    let input = human_review::answer_from_document(&document)
        .map_err(|e| unavailable("decoding answer", e.reason))?;
    let before = runtime
        .published_events()
        .map_err(|e| unavailable("capturing answer publication boundary", e))?;
    let mut result = SemanticCommandResult::undeclared();
    match runtime.answer_attention(&input, || Timestamp::from_millis(5)) {
        Ok(receipt) => {
            result.outcome = Some(
                serde_json::from_value(
                    json!({"command":"ekr.kernel.AnswerAttention","outcome":"answered"}),
                )
                .unwrap(),
            );
            let observed = target.event(
                request,
                "ekr.kernel.AnswerAttentionResult",
                json!({"receipt":receipt}),
            )?;
            result.response = Some(observed.payload.clone());
            result.direct_events.push(observed);
        }
        Err(error) => {
            // The public runtime names reviewed refusals with this prefix. Any other error is
            // an adapter execution failure, never an invented expected refusal.
            let ekr_kernel::CommitError::Store(ekr_kernel::PersistenceError::Document(text)) =
                error
            else {
                return Err(unavailable("answering reviewed question", error));
            };
            let Some(reason) = text.strip_prefix("attention-answer: ") else {
                return Err(unavailable("answering reviewed question", text));
            };
            let Some((code, _)) = reason.split_once(':') else {
                return Err(unavailable("decoding named answer refusal", reason));
            };
            result.outcome = Some(
                serde_json::from_value(
                    json!({"command":"ekr.kernel.AnswerAttention","outcome":"refused"}),
                )
                .unwrap(),
            );
            result.error = Some(
                DeclaredErrorValue::new("ekr.kernel.KnowledgeRefused".parse().unwrap())
                    .with("code", Node::Text(code.into()))
                    .with("reason", Node::Text(reason.into())),
            );
            let after = runtime
                .published_events()
                .map_err(|e| unavailable("reading refused command boundary", e))?;
            if after != before {
                return Err(unavailable(
                    "checking refused answer",
                    "publication changed",
                ));
            }
        }
    }
    result.consistency = Some(consistency(
        &runtime
            .read(None)
            .map_err(|e| unavailable("reading answer boundary", e))?,
    )?);
    Ok(result)
}

pub(super) fn rows(runtime: &Runtime) -> Result<Vec<ViewRow>, TargetError> {
    runtime.answer_history(None).map_err(|e| unavailable("replaying answer records", e))?.into_iter().map(|record| {
        let review = value(&record.review)?;
        map(json!({"answer_id":record.answer_id,"state":"Recorded","dispute_id":record.dispute_id,
            "basis":record.basis,"operator":review["operator"],"corrections":record.corrections,
            "statement_evidence":record.statement_evidence,"transaction_id":record.transaction_id,
            "human_proof_digest":review["proof_digest"]}))
    }).collect()
}
