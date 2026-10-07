//! Native knowledge adapter. Fixtures establish inputs and trust; only Runtime answers commands.
use super::super::{error, event_ref, outcome_ref, unavailable, Provider};
use crate::host::CliHostConfigurationV1;
use ekr_core::{
    bytes, canonical::Canonical, contract_data as w, contracts::kernel as m, ContentHash, Timestamp,
};
use ekr_kernel::{human_review as review, PersistenceError, Runtime, SeedDocument};
use ess_conformance::target::*;
use ess_primitives::{facts::Number, node::Node};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::PathBuf,
};

type Signer = Box<dyn Fn(&[u8]) -> Vec<u8>>;
struct Reviewer {
    public_key: Vec<u8>,
    sign: Signer,
}
struct State {
    scenario: String,
    path: PathBuf,
    binding: Option<m::TrustedReviewHostBinding>,
    values: BTreeMap<String, Node>,
}
/// A/E may retain metadata, but must leave both graph state and canonical occurrences intact.
struct CanonicalObservation {
    head: Option<ekr_graph::Root>,
    occurrences: Vec<ekr_kernel::runtime::PublishedEvent>,
}
impl CanonicalObservation {
    fn capture(runtime: &Runtime) -> Result<Self, TargetError> {
        let head = runtime
            .head()
            .map_err(|e| unavailable("verifying canonical knowledge before/after command", e))?;
        let occurrences = runtime
            .published_events()
            .map_err(|e| unavailable("reading actual canonical occurrences", e))?
            .into_iter()
            .filter(|event| event.stream_type == "ekr.revision" && event.stream_id == "canonical")
            .collect();
        Ok(Self { head, occurrences })
    }
    fn require_unchanged(&self, after: &Self) -> Result<(), TargetError> {
        if self.head != after.head || self.occurrences != after.occurrences {
            return Err(unavailable(
                "checking knowledge command canonical side effects",
                "A/E command changed canonical head or canonical occurrences",
            ));
        }
        Ok(())
    }
}
pub(super) struct KnowledgeAdapter {
    provider: Provider,
    host: CliHostConfigurationV1,
    seed: SeedDocument,
    literal_import: Vec<u8>,
    literal_proposal: Vec<u8>,
    work: PathBuf,
    sequence: Cell<u64>,
    reviewer: Option<Reviewer>,
    state: RefCell<Option<State>>,
}
fn value<T: Serialize>(value: T) -> Result<Value, TargetError> {
    serde_json::to_value(value).map_err(|e| unavailable("serializing runtime result", e))
}
fn decode<T: DeserializeOwned>(value: Value) -> Result<T, TargetError> {
    serde_json::from_value(value).map_err(|e| unavailable("decoding typed fixture/input", e))
}
fn node(value: Value) -> Result<Node, TargetError> {
    Ok(match value {
        Value::Null => Node::Null,
        Value::Bool(v) => Node::Bool(v),
        Value::String(v) => Node::Text(v),
        Value::Number(v) => {
            Node::Number(Number::from(v.as_i64().ok_or_else(|| {
                unavailable("converting result number", "expected i64")
            })?))
        }
        Value::Array(v) => Node::Seq(v.into_iter().map(node).collect::<Result<_, _>>()?),
        Value::Object(v) => Node::Map(
            v.into_iter()
                .map(|(k, v)| Ok((k, node(v)?)))
                .collect::<Result<_, TargetError>>()?,
        ),
    })
}
fn json_input(node: &Node) -> Result<Value, TargetError> {
    Ok(match node {
        Node::Null => Value::Null,
        Node::Bool(v) => (*v).into(),
        Node::Text(v) => v.clone().into(),
        Node::Number(v) => v
            .as_i64()
            .ok_or_else(|| unavailable("input number", "expected i64"))?
            .into(),
        Node::Seq(v) => Value::Array(v.iter().map(json_input).collect::<Result<_, _>>()?),
        Node::Map(v) => Value::Object(
            v.iter()
                .map(|(k, v)| Ok((k.clone(), json_input(v)?)))
                .collect::<Result<_, TargetError>>()?,
        ),
    })
}
fn hash(bytes: &[u8]) -> m::ContentHash {
    m::ContentHash(review::digest(bytes).to_string())
}
impl KnowledgeAdapter {
    pub(super) fn new(
        provider: Provider,
        host: CliHostConfigurationV1,
        seed: SeedDocument,
        work: PathBuf,
        literal_import: Vec<u8>,
        literal_proposal: Vec<u8>,
    ) -> Self {
        Self {
            provider,
            host,
            seed,
            literal_import,
            literal_proposal,
            work,
            sequence: Cell::new(0),
            reviewer: None,
            state: RefCell::new(None),
        }
    }
    pub(super) fn reviewer(
        &mut self,
        public_key: Vec<u8>,
        sign: impl Fn(&[u8]) -> Vec<u8> + 'static,
    ) {
        self.reviewer = Some(Reviewer {
            public_key,
            sign: Box::new(sign),
        });
    }
    fn open(&self, state: &State) -> Result<Runtime, TargetError> {
        let runtime = match self.provider {
            Provider::File => Runtime::file(
                &state.path,
                &self.host.tenant,
                self.host.context,
                self.host.authority.clone(),
            ),
            Provider::Sqlite => Runtime::sqlite(
                &state.path,
                &self.host.tenant,
                self.host.context,
                self.host.authority.clone(),
            ),
        }
        .map_err(|e| unavailable("opening native knowledge store", e))?;
        let mut runtime = if let Some(binding) = &state.binding {
            runtime
                .with_review_authority(binding.clone())
                .map_err(|e| unavailable("binding fixture reviewer", e))?
        } else {
            runtime
        };
        runtime.set_full_replay(true);
        Ok(runtime)
    }
    fn observation(&self) -> Result<w::EkrObserveObservationImport, TargetError> {
        let payload = b"Synthetic project health is amber.";
        let address = ContentHash::of_bytes(payload);
        let key = ekr_core::ObservationIdempotencyKey {
            source: "conformance".into(),
            source_native_id: Some("health-1".into()),
            content_hash: address,
        };
        decode(
            json!({"observation":{"observation_id":key.observation_id(),"source":key.source,"source_native_id":key.source_native_id,"content_hash":address,"captured_at":"1970-01-01T00:00:00Z","kind":"FeedItem"},"key":{"source":key.source,"source_native_id":key.source_native_id,"content_hash":address},"payload":bytes::encode(payload)}),
        )
    }
    fn interpretation(
        &self,
        source: &w::EkrObserveObservationImport,
    ) -> Result<w::EkrIntegrateInterpretationImport, TargetError> {
        use ekr_core::generated_identity::{Identity, InterpretationId};
        let evidence = ekr_core::EvidenceId::mint();
        let reference = json!({"node_type":"ProjectHealthFixture","aliases":["Synthetic project"]});
        let document = json!({"version":{"interpretation_id":InterpretationId::mint(),"version":1},"root_id":ekr_core::GraphRootId::mint(),"observations":[source.observation.observation_id],
            "local_schema":{"node_types":[{"name":"ProjectHealthFixture","parents":[],"abstract_type":false,"properties":[{"name":"health","value":{"value_kind":"String"},"required":false,"cardinality":"One"}]}],"edge_types":[]},
            "entities":[reference.clone()],"facts":[{"kind":"Property","value":{"subject":reference,"property":"health","value":{"kind":"String","canonical_bytes":bytes::encode(&ekr_graph::CanonicalValue::String("amber".into()).canonical_bytes())},"evidence":[evidence]}}],
            "evidence":[{"evidence":{"id":evidence,"source":{"kind":"Observation","observation":source.observation.observation_id},"content_hash":source.observation.content_hash,"observed_at":"1970-01-01T00:00:00Z","extracted_by":self.host.context.operator,"confidence_bp":10000},"payload":source.payload}]});
        let raw = serde_json::to_vec(&document)
            .map_err(|e| unavailable("encoding interpretation fixture", e))?;
        decode(json!({"document":document,"payload":bytes::encode(&raw)}))
    }
    fn enroll(
        &self,
        state: &mut State,
        seed_hash: ContentHash,
    ) -> Result<(m::ReviewerTrustPolicy, m::TrustedReviewHostBinding), TargetError> {
        use ekr_core::contracts::primitives::Uuid;
        let signer = self.reviewer.as_ref().ok_or_else(|| {
            TargetError::unsupported(
                "signed schema review fixture",
                "no independently supplied public key and signer",
            )
        })?;
        let audience = m::HumanDecisionAudience {
            tenant: self.host.tenant.clone(),
            seed_anchor: m::ContentHash(seed_hash.to_string()),
        };
        let policy = m::ReviewerTrustPolicy {
            format: m::ReviewerTrustFormat::ReviewerTrust1,
            audience: audience.clone(),
            keys: vec![m::ReviewerVerificationKey {
                key_digest: hash(&signer.public_key),
                algorithm: m::ReviewSignatureAlgorithm::Ed25519,
                public_key: signer.public_key.clone(),
                operator: m::TrustedOperatorIdentity {
                    actor: m::AgentId(Uuid(self.host.context.operator.to_string())),
                    authentication_subject: "independent-conformance-reviewer".into(),
                },
                scopes: vec![
                    m::HumanDecisionScope::ApproveSchemaProposal,
                    m::HumanDecisionScope::RejectSchemaProposal,
                    m::HumanDecisionScope::UpgradeAuthority,
                ],
            }],
        };
        let binding = m::TrustedReviewHostBinding {
            audience,
            reviewer_policy_digest: hash(&review::policy_bytes(&policy).map_err(|e| {
                unavailable(
                    "encoding fixture policy",
                    format!("{}: {}", e.code, e.reason),
                )
            })?),
        };
        state.binding = Some(binding.clone());
        let runtime = self.open(state)?;
        let preview = runtime
            .preview_upgrade(&policy)
            .map_err(|e| unavailable("previewing fixture enrollment", e))?;
        let statement = b"Synthetic reviewer approves this exact fixture enrollment.";
        let intent = m::HumanDecisionIntent {
            format: m::HumanDecisionFormat::HumanDecision1,
            decision_id: Uuid(ekr_core::EventId::mint().to_string()),
            audience: binding.audience.clone(),
            reviewer_policy_digest: binding.reviewer_policy_digest.clone(),
            signer_key_digest: hash(&signer.public_key),
            target: m::HumanDecisionTarget::UpgradeAuthority(m::AuthorityUpgradeTarget {
                preview_digest: m::ContentHash(preview.preview_digest.0.clone()),
                reviewer_policy_digest: binding.reviewer_policy_digest.clone(),
            }),
            statement_digest: hash(statement),
            expected_previous_decision: None,
        };
        let message = review::signing_bytes(&intent).map_err(|e| {
            unavailable(
                "encoding exact enrollment signing input",
                format!("{}: {}", e.code, e.reason),
            )
        })?;
        let proof = m::SignedHumanDecision {
            intent,
            algorithm: m::ReviewSignatureAlgorithm::Ed25519,
            signature: (signer.sign)(&message),
        };
        runtime
            .apply_upgrade(&preview, &policy, &proof, statement, || {
                Timestamp::from_millis(1)
            })
            .map_err(|e| unavailable("applying signed fixture enrollment", e))?;
        Ok((policy, binding))
    }
    pub(super) fn prepare(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        let name = scenario.scenario.to_string();
        if self
            .state
            .borrow()
            .as_ref()
            .is_some_and(|s| s.scenario == name)
        {
            return Ok(());
        }
        let command = name.split('/').next().unwrap_or_default();
        if !Self::supports(command) {
            return Ok(());
        }
        let refused = name.ends_with("/refused");
        let sequence = self.sequence.get();
        self.sequence.set(sequence + 1);
        let directory = self.work.join(sequence.to_string());
        std::fs::create_dir_all(&directory)
            .map_err(|e| unavailable("creating fixture namespace", e))?;
        let mut state = State {
            scenario: name.clone(),
            path: directory.join("store"),
            binding: None,
            values: BTreeMap::new(),
        };
        let runtime = self.open(&state)?;
        // Discover's external refusal is an actual unseeded namespace.
        if command == "ekr.integrate.DiscoverSchemaGaps" && refused {
            *self.state.borrow_mut() = Some(state);
            return Ok(());
        }
        let seeded = runtime
            .seed(self.seed.clone(), || Timestamp::EPOCH)
            .map_err(|e| unavailable("seeding knowledge fixture", e))?;
        let source = self.observation()?;
        runtime
            .import_observation(&source, Timestamp::EPOCH)
            .map_err(|e| unavailable("retaining fixture observation", e))?;
        let interpretation = self.interpretation(&source)?;
        let mut input = value(&interpretation)?;
        if command == "ekr.integrate.ImportInterpretation" {
            // The generated finite literal remains untouched. This independent fixture supplies
            // only its exact payload; drift causes a real command refusal rather than substitution.
            let document: w::EkrIntegrateInterpretationDocument =
                serde_json::from_slice(&self.literal_import)
                    .map_err(|e| unavailable("decoding finite interpretation fixture", e))?;
            input = json!({"document":document,"payload":bytes::encode(if refused {b"{}"} else {&self.literal_import})});
        }
        state.values.insert(
            "interpretation-document".into(),
            node(input["document"].clone())?,
        );
        state.values.insert(
            "interpretation-payload".into(),
            node(input["payload"].clone())?,
        );
        if command == "ekr.integrate.ImportInterpretation" {
            *self.state.borrow_mut() = Some(state);
            return Ok(());
        }
        let held = runtime
            .import_interpretation(&interpretation, Timestamp::EPOCH)
            .map_err(|e| unavailable("retaining fixture interpretation", e))?;
        let mut version = value(&held.version)?;
        if command == "ekr.integrate.ShowInterpretation" && refused {
            version["document_digest"] = ContentHash::of_bytes(b"not the retained document")
                .to_string()
                .into();
        }
        state
            .values
            .insert("interpretation-version".into(), node(version)?);
        if !command.contains("SchemaProposal") {
            if command == "ekr.integrate.DiscoverSchemaGaps" {
                let other = self.interpretation(&source)?;
                runtime
                    .import_interpretation(&other, Timestamp::EPOCH)
                    .map_err(|e| unavailable("retaining recurring gap", e))?;
            }
            *self.state.borrow_mut() = Some(state);
            return Ok(());
        }
        let review_command =
            command.ends_with("ApproveSchemaProposal") || command.ends_with("RejectSchemaProposal");
        drop(runtime);
        let enrollment = if review_command {
            Some(self.enroll(&mut state, seeded.seed_hash)?)
        } else {
            None
        };
        let runtime = self.open(&state)?;
        let base_schema = runtime
            .read(None)
            .map_err(|e| unavailable("reading schema fixture", e))?
            .graph
            .ontology
            .version()
            .id;
        let proposal = json!({"proposal_id":ekr_core::NodeId::mint(),"base_schema":base_schema,"sources":[{"version":held.version,"items":["facts[0]"]}],"observations":interpretation.document.observations,
            "evidence":[interpretation.document.evidence[0].evidence.id],"additions":[{"kind":"DefineType","value":interpretation.document.local_schema.node_types[0]}],
            "mappings":[{"source":held.version,"source_item":"facts[0]","source_type":"ProjectHealthFixture","target_type":"ProjectHealthFixture","target_member":"health","value":{"kind":"CopyField","value":{"declaration":"ProjectHealthFixture","field":"health"}}}],"corrections":[],"explanation":"Synthetic evidence supports optional health vocabulary."});
        let payload = bytes::encode(
            &serde_json::to_vec(&proposal)
                .map_err(|e| unavailable("encoding proposal fixture", e))?,
        );
        state
            .values
            .insert("schema-proposal-document".into(), node(proposal.clone())?);
        state.values.insert(
            "schema-proposal-payload".into(),
            node(if command.ends_with("SubmitSchemaProposal") && refused {
                json!(bytes::encode(b"{}"))
            } else {
                json!(payload)
            })?,
        );
        state.values.insert(
            "schema-proposal-id".into(),
            node(proposal["proposal_id"].clone())?,
        );
        if command.ends_with("SubmitSchemaProposal") {
            // Preserve the generated proposal literal exactly. This witness currently lacks
            // admissible support, so its answered scenario must stay failed, never fabricated.
            let document: w::EkrIntegrateSchemaProposalDocument =
                serde_json::from_slice(&self.literal_proposal)
                    .map_err(|e| unavailable("decoding finite proposal fixture", e))?;
            state
                .values
                .insert("schema-proposal-document".into(), node(value(document)?)?);
            state.values.insert(
                "schema-proposal-payload".into(),
                node(json!(bytes::encode(&self.literal_proposal)))?,
            );
        }
        if command.ends_with("SubmitSchemaProposal")
            || (command.ends_with("ShowSchemaProposal") && refused)
        {
            *self.state.borrow_mut() = Some(state);
            return Ok(());
        }
        let import = decode(json!({"proposal":proposal,"payload":payload}))?;
        let shown = runtime
            .submit_schema_proposal(&import, Timestamp::from_millis(2))
            .map_err(|e| unavailable("retaining fixture proposal", e))?;
        if let Some((policy, binding)) = enrollment {
            let statement = b"Synthetic human reviewed the exact additions, mappings and evidence.";
            let kind = if command.ends_with("ApproveSchemaProposal") {
                "ApproveSchemaProposal"
            } else {
                "RejectSchemaProposal"
            };
            let mut proof: w::EkrKernelSignedHumanDecision = decode(
                json!({"algorithm":"Ed25519","signature":bytes::encode(&[0;64]),"intent":{"format":"ekr.human-decision/1","decision_id":ekr_core::EventId::mint(),"audience":{"tenant":binding.audience.tenant,"seed_anchor":binding.audience.seed_anchor.0},"reviewer_policy_digest":binding.reviewer_policy_digest.0,"signer_key_digest":policy.keys[0].key_digest.0,"statement_digest":review::digest(statement).to_string(),"target":{"kind":kind,"value":{"proposal_id":shown.proposal.proposal_id,"proposal_digest":shown.proposal_digest,"basis":shown.basis}}}}),
            )?;
            let semantic = review::proof_from_document(&proof).map_err(|e| {
                unavailable(
                    "decoding exact review proof",
                    format!("{}: {}", e.code, e.reason),
                )
            })?;
            let message = review::signing_bytes(&semantic.intent).map_err(|e| {
                unavailable(
                    "encoding exact review signing input",
                    format!("{}: {}", e.code, e.reason),
                )
            })?;
            let signer = self
                .reviewer
                .as_ref()
                .ok_or_else(|| unavailable("signing fixture", "reviewer disappeared"))?;
            proof.signature = bytes::encode(&(signer.sign)(&message));
            if refused {
                proof.signature = bytes::encode(&[0; 64]);
            }
            // Conformance uses nominal enum variants; JSON uses their declared wire labels.
            let mut proof = value(proof)?;
            proof["intent"]["format"] = json!("HumanDecision1");
            for (key, v) in [
                ("schema-review-proof", value(proof)?),
                ("schema-review-basis", value(&shown.basis)?),
                (
                    "schema-review-proposal-digest",
                    value(&shown.proposal_digest)?,
                ),
                ("schema-review-statement", json!(bytes::encode(statement))),
            ] {
                state.values.insert(key.into(), node(v)?);
            }
        }
        *self.state.borrow_mut() = Some(state);
        Ok(())
    }
    pub(super) fn supports(command: &str) -> bool {
        matches!(
            command,
            "ekr.integrate.ImportInterpretation"
                | "ekr.integrate.ListInterpretations"
                | "ekr.integrate.ShowInterpretation"
                | "ekr.integrate.DiscoverSchemaGaps"
                | "ekr.integrate.SubmitSchemaProposal"
                | "ekr.integrate.ShowSchemaProposal"
                | "ekr.integrate.ApproveSchemaProposal"
                | "ekr.integrate.RejectSchemaProposal"
        )
    }
    pub(super) fn fixtures(
        &self,
        scenario: &ScenarioContext,
        contract: &ess_conformance::fixtures::Contract,
    ) -> Result<BTreeMap<String, Node>, TargetError> {
        self.prepare(scenario)?;
        let state = self.state.borrow();
        let state = state
            .as_ref()
            .ok_or_else(|| unavailable("supplying knowledge fixtures", "scenario not prepared"))?;
        Ok(state
            .values
            .iter()
            .filter(|(key, _)| {
                contract
                    .fields
                    .iter()
                    .any(|f| f.name.as_str() == key.as_str())
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect())
    }
    pub(super) fn clear(&self) {
        self.state.borrow_mut().take();
    }
    pub(super) fn execute(
        &self,
        request: &SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let command = request.command.to_string();
        let expected_actor = if command.ends_with("ApproveSchemaProposal")
            || command.ends_with("RejectSchemaProposal")
        {
            "ekr.integrate.HumanReviewer"
        } else {
            "ekr.integrate.KnowledgeProposer"
        };
        if request
            .actor
            .as_ref()
            .is_some_and(|a| a.to_string() != expected_actor)
        {
            return Err(TargetError::unsupported(
                command,
                format!("fixture host binds this operation to {expected_actor}"),
            ));
        }
        let state = self.state.borrow();
        let state = state
            .as_ref()
            .ok_or_else(|| unavailable("running knowledge command", "scenario not prepared"))?;
        let runtime = self.open(state)?;
        let mut input: Value = Value::Object(
            request
                .input
                .iter()
                .map(|(k, v)| Ok((k.clone(), json_input(v)?)))
                .collect::<Result<_, TargetError>>()?,
        );
        let operation = command.rsplit('.').next().unwrap_or_default();
        if matches!(operation, "ApproveSchemaProposal" | "RejectSchemaProposal") {
            if input["human_proof"]["intent"]["format"] != "HumanDecision1" {
                return Err(unavailable(
                    "decoding review format",
                    "expected semantic HumanDecision1 variant",
                ));
            }
            input["human_proof"]["intent"]["format"] = json!("ekr.human-decision/1");
        }
        let review_proposal = input["proposal_id"].clone();
        let before = CanonicalObservation::capture(&runtime)?;
        let answer:Result<Value,PersistenceError>=match operation {
            "ImportInterpretation"=>runtime.import_interpretation(&decode(input)?,Timestamp::from_millis(3)).map(|receipt|json!({"receipt":receipt})),
            "ListInterpretations"=>runtime.interpretations().map(|rows|json!({"interpretations":rows})),
            "ShowInterpretation"=>runtime.interpretation(&decode(input["version"].clone())?).map(|shown|json!(shown)),
            "DiscoverSchemaGaps"=>runtime.discover_schema_gaps().map(|request|json!({"request":request})),
            "SubmitSchemaProposal"=>runtime.submit_schema_proposal(&decode(input)?,Timestamp::from_millis(3)).map(|shown|json!({"proposal_id":shown.proposal.proposal_id,"proposal_digest":shown.proposal_digest,"preview":shown.preview})),
            "ShowSchemaProposal"=>runtime.schema_proposal(&decode(input["proposal_id"].clone())?).map(|shown|json!(shown)),
            "ApproveSchemaProposal"=>runtime.approve_schema_proposal(&decode(input.clone())?,Timestamp::from_millis(3)).map(|review|json!({"review_id":review.review_id})),
            "RejectSchemaProposal"=>runtime.reject_schema_proposal(&decode(input.clone())?,Timestamp::from_millis(3)).map(|review|json!({"review_id":review.review_id})),
            _=>return Err(TargetError::unsupported(command,"knowledge adapter does not implement this operation")),
        };
        drop(runtime);
        // Reopen with full replay even for refusals, and observe only the canonical stream:
        // observation/incubation/proposal/review retention is allowed to append metadata.
        let reopened = self.open(state)?;
        before.require_unchanged(&CanonicalObservation::capture(&reopened)?)?;
        if matches!(operation, "ApproveSchemaProposal" | "RejectSchemaProposal") {
            // The generated event only checks an id shape. Verify that this id is a durable,
            // authenticated decision after reopening and full replay, before reporting it.
            let held = reopened
                .schema_proposal_reviews(&decode(review_proposal)?)
                .map_err(|e| unavailable("replaying retained schema review", e))?;
            match &answer {
                Ok(returned) => {
                    let wanted = if operation == "ApproveSchemaProposal" {
                        "Approved"
                    } else {
                        "Rejected"
                    };
                    if !held.iter().any(|record| {
                        value(&record.review).is_ok_and(|review| {
                            review["review_id"] == returned["review_id"]
                                && review["decision"] == wanted
                        })
                    }) {
                        return Err(unavailable(
                            "checking persisted schema decision",
                            "returned review id and decision are absent after reopen/full replay",
                        ));
                    }
                }
                Err(_) if !held.is_empty() => {
                    return Err(unavailable(
                        "checking refused schema review",
                        "a refused review left a retained decision",
                    ))
                }
                Err(_) => {}
            }
        }
        let mut result = SemanticCommandResult::undeclared();
        match answer {
            Ok(value) => {
                let payload = node(value)?.as_map().cloned().ok_or_else(|| {
                    unavailable("reading actual command result", "expected record")
                })?;
                let mut event = ObservedEvent::new(event_ref(&format!("{command}Result"))?)
                    .in_activity(request.correlation.clone());
                event.payload = payload.clone();
                result.outcome = Some(outcome_ref(&command, "answered")?);
                result.response = Some(payload);
                result.direct_events = vec![event];
            }
            Err(
                fault @ (PersistenceError::Document(_)
                | PersistenceError::PublicationInputConflict
                | PersistenceError::Conflict),
            ) => {
                result.outcome = Some(outcome_ref(&command, "refused")?);
                result.error = Some(
                    error("ekr.integrate.KnowledgeRefused")?
                        .with("code", Node::Text("knowledge-refused".into()))
                        .with("reason", Node::Text(fault.to_string())),
                );
            }
            Err(fault) => return Err(unavailable("executing native knowledge command", fault)),
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_guard_rejects_real_proposal_with_unchanged_head_on_both_providers() {
        let fixtures = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("tests/fixtures/conformance");
        let mut missed = Vec::new();
        for provider in [Provider::File, Provider::Sqlite] {
            let work = tempfile::tempdir().unwrap();
            let target =
                super::super::IntegrateTarget::new(provider, &fixtures, work.path()).unwrap();
            let adapter = &target.knowledge;
            let scenario = ScenarioContext::new(
                "ekr.integrate.ListInterpretations/outcome/answered"
                    .parse()
                    .unwrap(),
                ess_primitives::ids::CorrelationId::new("canonical-write-control").unwrap(),
            );
            adapter.prepare(&scenario).unwrap();
            let state = adapter.state.borrow();
            let state = state.as_ref().unwrap();
            let runtime = adapter.open(state).unwrap();
            let before = CanonicalObservation::capture(&runtime).unwrap();
            before
                .require_unchanged(&CanonicalObservation::capture(&runtime).unwrap())
                .unwrap();

            // A real ordinary proposal appends canonical history without moving graph head.
            // This is the same comparison used before the adapter reports success or refusal.
            runtime
                .propose(
                    &std::fs::read(fixtures.join("propose-node.yaml")).unwrap(),
                    adapter.host.context.operator,
                    || Timestamp::from_millis(3),
                )
                .unwrap();
            drop(runtime);
            let reopened = adapter.open(state).unwrap();
            let after = CanonicalObservation::capture(&reopened).unwrap();
            assert_eq!(before.head, after.head, "{provider:?}");
            assert_eq!(
                after.occurrences.len(),
                before.occurrences.len() + 1,
                "{provider:?}: the ordinary proposal must really append a canonical occurrence"
            );
            if before.require_unchanged(&after).is_ok() {
                missed.push(provider);
            }
        }
        assert!(
            missed.is_empty(),
            "canonical writes escaped the guard: {missed:?}"
        );
    }
}
