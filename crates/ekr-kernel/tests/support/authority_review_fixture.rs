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
    let mut doc =
        SeedDocument::from_yaml(include_str!("../fixtures/seed-minimal-v2.yaml")).unwrap();
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
