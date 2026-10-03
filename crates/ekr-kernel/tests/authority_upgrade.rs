//! Explicit upgrades preserve historical bytes and rules on both native providers.
#[path = "recovery/support.rs"]
#[allow(dead_code)]
mod recovery;
use ekr_core::contracts::{kernel as m, primitives::Uuid};
use ekr_core::{
    AssertionId, ContentHash, EvidenceId, NodeId, PropertyId, RevisionNumber, Timestamp,
    TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Node, Object,
    Predicate, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    human_review as review, Agent, AuthorityStateV1, BootstrapContext, Commit, CommitCommandResult,
    GraphOperation, GraphTransaction, NodeDraft, SeedDocument, TransactionState,
    ValidationCommandResult, ValidationProfileV1,
};
use ekr_ontology::{Cardinality, NodeType, PropertyDefinition, Value, ValueType};
use ekr_store::{
    FileStore, IncubationRetention, Initialize, ObjectStore, ObservationRetention, RevisionLog,
    SqliteStore,
};
use ring::signature::{Ed25519KeyPair, KeyPair};
use std::collections::{BTreeMap, BTreeSet};

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}
fn anchor() -> AuthorityStateV1 {
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [context().operator, context().validator]
            .into_iter()
            .map(|id| {
                (
                    id,
                    Agent {
                        id,
                        name: "fixture".into(),
                        capabilities: BTreeSet::new(),
                    },
                )
            })
            .collect(),
        validation_profile: ValidationProfileV1::deterministic(context().validator),
    }
}
fn hash(bytes: &[u8]) -> m::ContentHash {
    m::ContentHash(review::digest(bytes).to_string())
}
struct Human {
    key: Ed25519KeyPair,
    policy: m::ReviewerTrustPolicy,
    binding: m::TrustedReviewHostBinding,
}
impl Human {
    fn new(seed: ContentHash) -> Self {
        let key = Ed25519KeyPair::from_seed_unchecked(&[19; 32]).unwrap();
        let audience = m::HumanDecisionAudience {
            tenant: "upgrade-fixture".into(),
            seed_anchor: m::ContentHash(seed.to_string()),
        };
        let policy = m::ReviewerTrustPolicy {
            format: m::ReviewerTrustFormat::ReviewerTrust1,
            audience: audience.clone(),
            keys: vec![m::ReviewerVerificationKey {
                key_digest: hash(key.public_key().as_ref()),
                algorithm: m::ReviewSignatureAlgorithm::Ed25519,
                public_key: key.public_key().as_ref().to_vec(),
                operator: m::TrustedOperatorIdentity {
                    actor: m::AgentId(Uuid(context().operator.to_string())),
                    authentication_subject: "fixture-human".into(),
                },
                scopes: vec![m::HumanDecisionScope::UpgradeAuthority],
            }],
        };
        let binding = m::TrustedReviewHostBinding {
            audience,
            reviewer_policy_digest: hash(&review::policy_bytes(&policy).unwrap()),
        };
        Self {
            key,
            policy,
            binding,
        }
    }
    fn proof(
        &self,
        preview: &ekr_core::contract_data::EkrKernelUpgradePreview,
    ) -> m::SignedHumanDecision {
        let intent = m::HumanDecisionIntent {
            format: m::HumanDecisionFormat::HumanDecision1,
            decision_id: Uuid(ekr_core::EventId::mint().to_string()),
            audience: self.binding.audience.clone(),
            reviewer_policy_digest: self.binding.reviewer_policy_digest.clone(),
            signer_key_digest: hash(self.key.public_key().as_ref()),
            target: m::HumanDecisionTarget::UpgradeAuthority(m::AuthorityUpgradeTarget {
                preview_digest: m::ContentHash(preview.preview_digest.0.clone()),
                reviewer_policy_digest: self.binding.reviewer_policy_digest.clone(),
            }),
            statement_digest: hash(b"reviewed contradictions and pending validations"),
            expected_previous_decision: None,
        };
        let signature = self
            .key
            .sign(&review::signing_bytes(&intent).unwrap())
            .as_ref()
            .to_vec();
        m::SignedHumanDecision {
            intent,
            algorithm: m::ReviewSignatureAlgorithm::Ed25519,
            signature,
        }
    }
}
fn seed() -> SeedDocument {
    let mut doc = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let ty = TypeId::mint();
    let property = PropertyId::mint();
    let mut definition = NodeType::new(ty, "Project");
    let mut p = PropertyDefinition::new(property, "health", ValueType::String);
    p.cardinality = Cardinality::One;
    definition.properties.insert(property, p);
    doc.ontology.node_types.push(definition);
    let node = Node::new(NodeId::mint(), doc.graph.root.id, ty, "Fixture project");
    let node_id = node.id;
    doc.graph.nodes.insert(node_id, node);
    for value in ["green", "red"] {
        let payload = format!("Fixture health is {value}").into_bytes();
        let address = ContentHash::of_bytes(&payload);
        let evidence = Evidence {
            id: EvidenceId::mint(),
            source: EvidenceSource::HumanStatement { identity: None },
            content_hash: address,
            extracted_by: context().operator,
            observed_at: Timestamp::EPOCH,
            confidence: Confidence::CERTAIN,
        };
        let claim = Assertion {
            id: AssertionId::mint(),
            root_id: doc.graph.root.id,
            subject: Subject::Node(node_id),
            predicate: Predicate::Property(property),
            object: Object::Value(Value::String(value.into())),
            evidence: [evidence.id].into_iter().collect(),
            proposed_by: context().operator,
            assessment: Assessment::Proposed,
            lifecycle: AssertionLifecycle::Active,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        };
        doc.graph.evidence.insert(evidence.id, evidence);
        doc.evidence_payloads.insert(address, payload.into());
        doc.graph.assertions.insert(claim.id, claim);
    }
    doc
}
fn proposal(doc: &SeedDocument) -> (TransactionId, Vec<u8>) {
    let tx: GraphTransaction = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id: doc.graph.root.id,
            type_id: doc.ontology.node_types[0].id,
            canonical_name: "Another fixture".into(),
            properties: BTreeMap::new(),
            aliases: vec![],
        })],
        evidence: BTreeSet::new(),
        schema_version: None,
    };
    (tx.id, encode(&tx))
}
fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(serde::Serialize)]
    struct Wire<'a> {
        format: &'a str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}
fn run<S: RevisionLog + ObjectStore + Initialize + ObservationRetention + IncubationRetention>(
    open: impl Fn(Option<m::TrustedReviewHostBinding>, bool) -> Commit<S>,
) {
    let doc = seed();
    let old = open(None, false);
    let seeded = old.seed(doc.clone(), || Timestamp::EPOCH).unwrap();
    let original_root = old.head().unwrap().unwrap();
    let original_graph = old.snapshot().unwrap();
    assert!(
        old.attention().unwrap().is_empty(),
        "historical authority must not silently activate disputes"
    );
    assert!(original_graph
        .assertions
        .values()
        .all(|a| matches!(a.assessment, Assessment::Accepted { .. })));
    let human = Human::new(seeded.seed_hash);
    assert!(old
        .preview_upgrade(&human.policy)
        .unwrap_err()
        .to_string()
        .contains("review-host-not-provisioned"));
    drop(old);
    let mut wrong_tenant = human.binding.clone();
    wrong_tenant.audience.tenant = "another-tenant".into();
    assert!(open(Some(wrong_tenant), true)
        .head()
        .unwrap_err()
        .to_string()
        .contains("review-tenant-audience"));
    let kernel = open(Some(human.binding.clone()), false);
    let stale = kernel.preview_upgrade(&human.policy).unwrap();
    let stale_proof = human.proof(&stale);
    let (id, document) = proposal(&doc);
    kernel
        .propose(&document, context().operator, || Timestamp::from_millis(1))
        .unwrap();
    assert!(matches!(
        kernel
            .validate(id, RevisionNumber::SEED, || Timestamp::from_millis(2))
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    let old_validation = kernel.transactions().unwrap()[&id]
        .validation_record_hash
        .unwrap();
    let old_validation_bytes = kernel.content(&old_validation).unwrap().unwrap();
    assert!(kernel
        .apply_upgrade(
            &stale,
            &human.policy,
            &stale_proof,
            b"reviewed contradictions and pending validations",
            || panic!("stale preview must refuse before time")
        )
        .unwrap_err()
        .to_string()
        .contains("upgrade-preview-stale"));
    let preview = kernel.preview_upgrade(&human.policy).unwrap();
    assert_eq!(preview.contradictions.len(), 1);
    assert_eq!(preview.pending_revalidation.len(), 1);
    let proof = human.proof(&preview);
    let mut forged = proof.clone();
    forged.signature[0] ^= 1;
    assert!(kernel
        .apply_upgrade(
            &preview,
            &human.policy,
            &forged,
            b"reviewed contradictions and pending validations",
            || panic!("forged proof must refuse before time")
        )
        .is_err());
    assert_eq!(kernel.head().unwrap().unwrap(), original_root);
    let receipt = kernel
        .apply_upgrade(
            &preview,
            &human.policy,
            &proof,
            b"reviewed contradictions and pending validations",
            || Timestamp::from_millis(3),
        )
        .unwrap();
    let upgraded = kernel.snapshot().unwrap();
    assert_eq!(upgraded.revision, original_graph.revision.next().unwrap());
    assert!(upgraded.assertions.values().all(|a| matches!(&a.assessment, Assessment::Disputed { competing_assertions } if competing_assertions.len() == 1)));
    assert!(upgraded.assertions.values().all(|a| !a.is_current()));
    let questions = kernel.attention().unwrap();
    assert_eq!(questions.len(), 1);
    let question = &questions[0];
    assert_eq!(question.claims.len(), 2);
    assert!(!question.evidence.is_empty());
    assert!(question.question.contains("during which dates"));
    assert_eq!(kernel.attention_item(&question.subject).unwrap(), *question);
    let mut invalid_subject = *question.subject.clone();
    invalid_subject.blocker_id = ekr_core::contract_data::EssPresence::Present(Box::new(
        ekr_core::contract_data::EkrIntegrateIntegrationBlockerId(AssertionId::mint().to_string()),
    ));
    assert!(kernel.attention_item(&invalid_subject).is_err());
    // A read capture can be inspected independently. Bind which claim each retained source
    // supports, not just the union of evidence IDs: this changes no underlying store bytes.
    let mut capture = kernel.read(None).unwrap();
    let claim_ids: Vec<_> = capture.graph.assertions.keys().copied().collect();
    let peer_evidence = capture.graph.assertions[&claim_ids[1]]
        .evidence
        .iter()
        .next()
        .unwrap()
        .id();
    std::sync::Arc::make_mut(&mut capture.graph)
        .attachments
        .entry(claim_ids[0])
        .or_default()
        .insert(ekr_graph::AttachedEvidence {
            evidence: ekr_graph::CanonicalRef::new(peer_evidence),
            revision: capture.root.revision,
        });
    let changed_support = capture.dispute_attention().unwrap();
    assert_eq!(changed_support[0].evidence, question.evidence);
    assert_ne!(
        changed_support[0].basis.evidence_digest, question.basis.evidence_digest,
        "the same source attached to another claim changes what the reviewer sees"
    );
    assert_eq!(kernel.attention().unwrap(), questions);
    assert_eq!(kernel.replay(RevisionNumber::SEED).unwrap(), original_graph);
    assert_eq!(
        kernel.transaction_states([id]).unwrap()[&id],
        TransactionState::Proposed
    );
    assert!(kernel
        .commit(id, context().operator, || Timestamp::from_millis(4))
        .is_err());
    let retry = kernel
        .apply_upgrade(
            &preview,
            &human.policy,
            &proof,
            b"reviewed contradictions and pending validations",
            || panic!("retry must not sample time"),
        )
        .unwrap();
    assert_eq!(
        serde_json::to_vec(&receipt).unwrap(),
        serde_json::to_vec(&retry).unwrap()
    );
    assert!(matches!(
        kernel
            .validate(id, upgraded.revision, || Timestamp::from_millis(4))
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        kernel
            .commit(id, context().operator, || Timestamp::from_millis(5))
            .unwrap(),
        CommitCommandResult::Committed(_)
    ));
    let unchanged_question = kernel.attention().unwrap().remove(0);
    assert_eq!(unchanged_question.subject, question.subject);
    assert_eq!(
        unchanged_question.basis.evidence_digest,
        question.basis.evidence_digest
    );
    assert_eq!(
        unchanged_question.basis.options_digest,
        question.basis.options_digest
    );
    assert_eq!(
        unchanged_question.basis.effects_digest,
        question.basis.effects_digest
    );
    assert_ne!(
        unchanged_question.basis.observed_revision,
        question.basis.observed_revision
    );
    // A new ordinary claim is assessed under the activated rules, too. Equal red claims do
    // not conflict with one another; both conflict with the independently retained green claim.
    let mut claim = doc
        .graph
        .assertions
        .values()
        .find(|a| a.object == Object::Value(Value::String("red".into())))
        .unwrap()
        .clone();
    claim.id = AssertionId::mint();
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        evidence: claim.evidence.clone(),
        operations: vec![GraphOperation::AddAssertion(Box::new(claim))],
        schema_version: None,
    };
    kernel
        .propose(&encode(&tx), context().operator, || {
            Timestamp::from_millis(6)
        })
        .unwrap();
    assert!(matches!(
        kernel
            .validate(tx.id, kernel.head().unwrap().unwrap().revision, || {
                Timestamp::from_millis(7)
            })
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(8))
            .unwrap(),
        CommitCommandResult::Committed(_)
    ));
    let graph = kernel.snapshot().unwrap();
    assert_eq!(graph.assertions.len(), 3);
    for assertion in graph.assertions.values() {
        let Assessment::Disputed {
            competing_assertions,
        } = &assertion.assessment
        else {
            panic!("new claim was left settled")
        };
        let expected = if assertion.object
            == Object::Value(ekr_graph::CanonicalValue::String("green".into()))
        {
            2
        } else {
            1
        };
        assert_eq!(competing_assertions.len(), expected);
        assert!(competing_assertions
            .iter()
            .all(|other| other.id() != assertion.id && graph.assertions.contains_key(&other.id())));
    }
    let final_root = kernel.head().unwrap();
    let final_questions = kernel.attention().unwrap();
    assert_eq!(final_questions.len(), 1);
    assert_eq!(final_questions[0].claims.len(), 3);
    assert_ne!(
        final_questions[0].basis.options_digest,
        question.basis.options_digest
    );
    assert_eq!(
        kernel.content(&old_validation).unwrap().unwrap(),
        old_validation_bytes
    );
    drop(kernel);
    assert!(
        open(None, true).head().is_err(),
        "retained agent data cannot provision the reviewer host"
    );
    for full in [false, true] {
        let reopened = open(Some(human.binding.clone()), full);
        assert_eq!(reopened.head().unwrap(), final_root);
        assert_eq!(reopened.attention().unwrap(), final_questions);
        assert_eq!(
            reopened.replay(RevisionNumber::SEED).unwrap(),
            original_graph
        );
        assert!(reopened
            .snapshot()
            .unwrap()
            .assertions
            .values()
            .all(|a| matches!(a.assessment, Assessment::Disputed { .. })));
        assert_eq!(
            reopened
                .content(&receipt.review.proof_object_hash.0.parse().unwrap())
                .unwrap()
                .unwrap(),
            review::proof_bytes(&proof).unwrap()
        );
    }
}
#[test]
fn upgrade_preserves_historical_rules_hashes_and_requires_revalidation() {
    let mut direct = anchor();
    direct.validation_profile = ValidationProfileV1::knowledge(context().validator);
    assert!(direct.validation_profile.disputes());
    assert!(direct.validation_profile.keeps_identities());
    assert!(
        Commit::<FileStore>::over_with_authority(context(), direct, |_| panic!(
            "knowledge authority cannot bypass upgrade"
        ))
        .is_err()
    );
    let file = tempfile::tempdir().unwrap();
    run(|binding, full| {
        let open = |authority| {
            let mut store = FileStore::file(file.path(), "upgrade-fixture", None)?.under(authority);
            store.set_full_replay(full);
            Ok(store)
        };
        match binding {
            Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
            None => Commit::over_with_authority(context(), anchor(), open),
        }
        .unwrap()
    });
    let sqlite = tempfile::tempdir().unwrap();
    run(|binding, full| {
        let open = |authority| {
            let mut store =
                SqliteStore::sqlite(&sqlite.path().join("store.db"), "upgrade-fixture", None)?
                    .under(authority);
            store.set_full_replay(full);
            Ok(store)
        };
        match binding {
            Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
            None => Commit::over_with_authority(context(), anchor(), open),
        }
        .unwrap()
    });
}

#[derive(serde::Serialize, serde::Deserialize)]
struct RetryPlan {
    file: bool,
    preview: ekr_core::contract_data::EkrKernelUpgradePreview,
    proof: Vec<u8>,
    policy: Vec<u8>,
    binding: Vec<u8>,
}
fn resume<S: RevisionLog + ObjectStore>(kernel: Commit<S>, plan: &RetryPlan) -> Vec<u8> {
    let result = kernel
        .apply_upgrade(
            &plan.preview,
            &review::read_policy(&plan.policy).unwrap(),
            &review::read_proof(&plan.proof).unwrap(),
            b"reviewed contradictions and pending validations",
            || panic!("retry sampled time"),
        )
        .unwrap();
    let bytes = serde_json::to_vec(&result).unwrap();
    assert_eq!(
        kernel
            .content(&ContentHash::of_bytes(&bytes))
            .unwrap()
            .unwrap(),
        bytes
    );
    assert_eq!(kernel.snapshot().unwrap().revision, RevisionNumber::new(1));
    bytes
}
fn child_retry(path: &std::path::Path) {
    let plan: RetryPlan =
        serde_json::from_slice(&std::fs::read(path.join("retry.json")).unwrap()).unwrap();
    let binding = review::read_host_binding(&plan.binding).unwrap();
    let result = if plan.file {
        resume(
            Commit::over_with_review_authority(context(), anchor(), binding, |a| {
                let mut s = FileStore::file(path, "upgrade-fixture", None)?.under(a);
                s.set_full_replay(true);
                Ok(s)
            })
            .unwrap(),
            &plan,
        )
    } else {
        resume(
            Commit::over_with_review_authority(context(), anchor(), binding, |a| {
                let mut s =
                    SqliteStore::sqlite(&path.join("store.db"), "upgrade-fixture", None)?.under(a);
                s.set_full_replay(true);
                Ok(s)
            })
            .unwrap(),
            &plan,
        )
    };
    std::fs::write(path.join("result.json"), result).unwrap();
}
fn recover<S: RevisionLog + ObjectStore + Initialize>(
    path: &std::path::Path,
    file: bool,
    fault: recovery::Fault,
    open: impl Fn(Option<m::TrustedReviewHostBinding>, recovery::Hooks) -> Commit<recovery::Probe<S>>,
) {
    let seeded = open(None, recovery::Hooks::new(recovery::Fault::Pass))
        .seed(seed(), || Timestamp::EPOCH)
        .unwrap();
    let human = Human::new(seeded.seed_hash);
    let hooks = recovery::Hooks::new(fault);
    let kernel = open(Some(human.binding.clone()), hooks.clone());
    let preview = kernel.preview_upgrade(&human.policy).unwrap();
    let proof = human.proof(&preview);
    let result = kernel.apply_upgrade(
        &preview,
        &human.policy,
        &proof,
        b"reviewed contradictions and pending validations",
        || Timestamp::from_millis(1),
    );
    assert!(
        matches!(
            result,
            Err(ekr_kernel::CommitError::Store(
                ekr_store::StoreError::UnknownCommit
            ))
        ),
        "{result:?}"
    );
    let attempts = hooks.resumed.borrow();
    assert_eq!(attempts.len(), 1);
    let elected = attempts[0].clone();
    assert_eq!(
        elected.format,
        ekr_store::PublicationPreparationV1::FORMAT_V4
    );
    assert_eq!(elected.decision.event.schema_version(), 3);
    assert!(elected.decision.event.supported());
    drop(attempts);
    drop(kernel);
    let plan = RetryPlan {
        file,
        preview,
        proof: review::proof_bytes(&proof).unwrap(),
        policy: review::policy_bytes(&human.policy).unwrap(),
        binding: review::host_binding_bytes(&human.binding).unwrap(),
    };
    std::fs::write(path.join("retry.json"), serde_json::to_vec(&plan).unwrap()).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "upgrade_port_fault_resumes_exactly_in_a_fresh_process",
            "--exact",
            "--test-threads=1",
        ])
        .env("EKR_UPGRADE_RETRY_PATH", path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let result = std::fs::read(path.join("result.json")).unwrap();
    assert_eq!(
        result,
        elected.decision.objects[&elected.decision.event.record_hash].bytes
    );
    let final_kernel = open(
        Some(human.binding.clone()),
        recovery::Hooks::new(recovery::Fault::Pass),
    );
    assert_eq!(resume(final_kernel, &plan), result);
}
/// Fresh process plus port-level before/after-write faults, not a native mid-transaction crash.
#[test]
fn upgrade_port_fault_resumes_exactly_in_a_fresh_process() {
    if let Some(path) = std::env::var_os("EKR_UPGRADE_RETRY_PATH") {
        child_retry(std::path::Path::new(&path));
        return;
    }
    for fault in [recovery::Fault::BeforeWrite, recovery::Fault::AfterWrite] {
        let file = tempfile::tempdir().unwrap();
        recover(file.path(), true, fault, |binding, hooks| {
            let open = |a| {
                Ok(recovery::Probe {
                    inner: FileStore::file(file.path(), "upgrade-fixture", None)?.under(a),
                    hooks,
                })
            };
            match binding {
                Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
                None => Commit::over_with_authority(context(), anchor(), open),
            }
            .unwrap()
        });
        let sqlite = tempfile::tempdir().unwrap();
        recover(sqlite.path(), false, fault, |binding, hooks| {
            let open = |a| {
                Ok(recovery::Probe {
                    inner: SqliteStore::sqlite(
                        &sqlite.path().join("store.db"),
                        "upgrade-fixture",
                        None,
                    )?
                    .under(a),
                    hooks,
                })
            };
            match binding {
                Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
                None => Commit::over_with_authority(context(), anchor(), open),
            }
            .unwrap()
        });
    }
}

fn tamper<S: RevisionLog + ObjectStore + Initialize>(
    open: impl Fn(ekr_kernel::KernelAuthority) -> S,
) {
    use ekr_store::CommitAuthority;
    let kernel = Commit::over_with_authority(context(), anchor(), |a| Ok(open(a))).unwrap();
    let seeded = kernel.seed(seed(), || Timestamp::EPOCH).unwrap();
    drop(kernel);
    let human = Human::new(seeded.seed_hash);
    let captured = std::cell::RefCell::new(None);
    let kernel =
        Commit::over_with_review_authority(context(), anchor(), human.binding.clone(), |a| {
            *captured.borrow_mut() = Some(a.clone());
            Ok(open(a))
        })
        .unwrap();
    let preview = kernel.preview_upgrade(&human.policy).unwrap();
    let proof = human.proof(&preview);
    let receipt = kernel
        .apply_upgrade(
            &preview,
            &human.policy,
            &proof,
            b"reviewed contradictions and pending validations",
            || Timestamp::from_millis(1),
        )
        .unwrap();
    let authority = captured.into_inner().unwrap();
    let history = open(authority.clone()).history().unwrap();
    assert!(authority
        .replay(&history, None, Some(RevisionNumber::new(1)))
        .unwrap()
        .is_some());
    for field in [
        "result",
        "operator",
        "preview",
        "policy",
        "proof",
        "host",
        "profile",
        "unknown",
        "duplicate",
        "noncanonical",
    ] {
        let mut forged = history.clone();
        let occurrence = forged.occurrences.last_mut().unwrap();
        let original = &history.objects[&occurrence.event.record_hash];
        let mut value = serde_json::to_value(&receipt).unwrap();
        match field {
            "result" => {
                let wrong = ContentHash::of_bytes(b"forged graph");
                value["result"]["knowledge_root"] = wrong.to_string().into();
                let ekr_graph::RevisionPayload::AuthorityUpgraded { knowledge_root, .. } =
                    &mut occurrence.event.payload
                else {
                    panic!()
                };
                *knowledge_root = wrong;
            }
            "operator" => {
                value["review"]["operator"]["authentication_subject"] = "agent-self-approval".into()
            }
            "preview" => {
                value["preview"]["pending_revalidation"] =
                    serde_json::json!([TransactionId::mint().to_string()])
            }
            "policy" | "proof" => {
                let key = if field == "policy" {
                    "policy_object_hash"
                } else {
                    "proof_object_hash"
                };
                let address: ContentHash = value["review"][key].as_str().unwrap().parse().unwrap();
                forged.objects.remove(&address);
            }
            "host" => {
                value["host_binding_object_hash"] =
                    ContentHash::of_bytes(b"agent host").to_string().into()
            }
            "profile" => {
                value["target_profile_object_hash"] = ContentHash::of_bytes(b"unsupported profile")
                    .to_string()
                    .into()
            }
            "unknown" => value["agent_may_approve"] = true.into(),
            "duplicate" | "noncanonical" => {}
            _ => unreachable!(),
        }
        let encoded = serde_json::to_string(&value).unwrap();
        let bytes = match field {
            "duplicate" => format!(
                "{{\"format\":\"ekr.authority-transition/1\",{}",
                &encoded[1..]
            )
            .into_bytes(),
            "noncanonical" => format!(" {encoded}").into_bytes(),
            _ => encoded.into_bytes(),
        };
        let address = ContentHash::of_bytes(&bytes);
        let mut held = original.clone();
        held.metadata.content_hash = address;
        held.metadata.byte_len = bytes.len() as u64;
        held.bytes = bytes.into();
        occurrence.event.record_hash = address;
        forged.objects.insert(address, held);
        assert!(
            authority
                .replay(&forged, None, Some(RevisionNumber::new(1)))
                .is_err(),
            "accepted forged {field}"
        );
    }
}
#[test]
fn full_replay_rejects_changed_transition_records_and_missing_review_objects() {
    let file = tempfile::tempdir().unwrap();
    tamper(|a| {
        FileStore::file(file.path(), "upgrade-fixture", None)
            .unwrap()
            .under(a)
    });
    let sqlite = tempfile::tempdir().unwrap();
    tamper(|a| {
        SqliteStore::sqlite(&sqlite.path().join("store.db"), "upgrade-fixture", None)
            .unwrap()
            .under(a)
    });
}

fn race<S: RevisionLog + ObjectStore + Initialize + 'static>(
    open: impl Fn(Option<m::TrustedReviewHostBinding>, recovery::Hooks) -> Commit<recovery::Probe<S>>
        + Clone
        + 'static,
) {
    let doc = seed();
    let seeded = open(None, recovery::Hooks::new(recovery::Fault::Pass))
        .seed(doc.clone(), || Timestamp::EPOCH)
        .unwrap();
    let human = Human::new(seeded.seed_hash);
    let hooks = recovery::Hooks::new(recovery::Fault::Pass);
    let kernel = open(Some(human.binding.clone()), hooks.clone());
    let preview = kernel.preview_upgrade(&human.policy).unwrap();
    let proof = human.proof(&preview);
    let other = open.clone();
    let binding = human.binding.clone();
    let (_, proposal) = proposal(&doc);
    *hooks.before_prepare.borrow_mut() = Some(Box::new(move || {
        other(
            Some(binding.clone()),
            recovery::Hooks::new(recovery::Fault::Pass),
        )
        .propose(&proposal, context().operator, || Timestamp::from_millis(1))
        .unwrap();
    }));
    let lost = kernel.apply_upgrade(
        &preview,
        &human.policy,
        &proof,
        b"reviewed contradictions and pending validations",
        || Timestamp::from_millis(2),
    );
    assert!(lost
        .unwrap_err()
        .to_string()
        .contains("upgrade-preview-stale"));
    assert_eq!(
        kernel.head().unwrap().unwrap().revision,
        RevisionNumber::SEED
    );
    let fresh = kernel.preview_upgrade(&human.policy).unwrap();
    assert_ne!(fresh.preview_digest, preview.preview_digest);
    let proof = human.proof(&fresh);
    kernel
        .apply_upgrade(
            &fresh,
            &human.policy,
            &proof,
            b"reviewed contradictions and pending validations",
            || Timestamp::from_millis(3),
        )
        .expect("a lost CAS must allow a newly reviewed preview");
    assert_eq!(
        kernel.head().unwrap().unwrap().revision,
        RevisionNumber::new(1)
    );
}
#[test]
fn a_lost_upgrade_race_allows_a_new_review_without_reusing_the_old_preparation() {
    let file = tempfile::tempdir().unwrap();
    let path = file.path().to_owned();
    race(move |binding, hooks| {
        let open = |a| {
            Ok(recovery::Probe {
                inner: FileStore::file(&path, "upgrade-fixture", None)?.under(a),
                hooks,
            })
        };
        match binding {
            Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
            None => Commit::over_with_authority(context(), anchor(), open),
        }
        .unwrap()
    });
    let sqlite = tempfile::tempdir().unwrap();
    let path = sqlite.path().to_owned();
    race(move |binding, hooks| {
        let open = |a| {
            Ok(recovery::Probe {
                inner: SqliteStore::sqlite(&path.join("store.db"), "upgrade-fixture", None)?
                    .under(a),
                hooks,
            })
        };
        match binding {
            Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
            None => Commit::over_with_authority(context(), anchor(), open),
        }
        .unwrap()
    });
}
