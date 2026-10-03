//! Independently supplied pre-upgrade data and test-only external human signer.
use ekr_core::contract_data as wire;
use ekr_core::contracts::kernel as m;
use ekr_core::{ContentHash, NodeId, PropertyId, Timestamp, TypeId};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Node, Object,
    Predicate, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    human_review as review, Agent, AuthorityStateV1, BootstrapContext, SeedDocument,
    ValidationProfileV1,
};
use ekr_ontology::{Cardinality, EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use ring::signature::{Ed25519KeyPair, KeyPair};
use serde_json::json;
use std::{collections::BTreeSet, fmt::Debug, str::FromStr};

pub const TENANT: &str = "upgrade-conformance";
pub const STATEMENT: &[u8] = b"reviewed fixture upgrade";
pub fn id<T: FromStr>(n: u64) -> T
where
    T::Err: Debug,
{
    format!("00000000-0000-4000-8000-{n:012}").parse().unwrap()
}
pub fn context() -> BootstrapContext {
    BootstrapContext {
        operator: id(3),
        validator: id(4),
    }
}
pub fn anchor() -> AuthorityStateV1 {
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

/// Input facts have stable fixture IDs; expected verdicts are authored in ESS, never here.
pub fn seed(exclusions: bool) -> SeedDocument {
    let mut doc = SeedDocument::from_yaml(include_str!(
        "../../../ekr-kernel/tests/fixtures/seed-minimal-v2.yaml"
    ))
    .unwrap();
    for group in 0..if exclusions { 3 } else { 1 } {
        let ty: TypeId = id(100 + group);
        let property: PropertyId = id(110 + group);
        let relation: TypeId = id(120 + group);
        let equal = exclusions && group == 0;
        let disjoint = exclusions && group == 1;
        let cardinality = if exclusions && group == 2 {
            Cardinality::Many
        } else {
            Cardinality::One
        };
        let mut definition = NodeType::new(ty, format!("Project{group}"));
        let mut p = PropertyDefinition::new(property, "health", ValueType::String);
        p.cardinality = cardinality;
        definition.properties.insert(property, p);
        doc.ontology.node_types.push(definition);
        let mut edge = EdgeType::new(relation, format!("ownership{group}"));
        edge.cardinality = cardinality;
        edge.source_types.insert(ty);
        edge.target_types.insert(ty);
        doc.ontology.edge_types.push(edge);
        let nodes: [NodeId; 3] = [
            id(200 + group * 3),
            id(201 + group * 3),
            id(202 + group * 3),
        ];
        for (index, node) in nodes.iter().enumerate() {
            doc.graph.nodes.insert(
                *node,
                Node::new(
                    *node,
                    doc.graph.root.id,
                    ty,
                    format!("Fixture {group}/{index}"),
                ),
            );
        }
        for claim in 0..4 {
            let ordinal = group * 4 + claim;
            let second = claim % 2 == 1;
            let payload = format!("Fixture observation {ordinal}").into_bytes();
            let address = ContentHash::of_bytes(&payload);
            let evidence = Evidence {
                id: id(500 + ordinal),
                source: EvidenceSource::HumanStatement { identity: None },
                content_hash: address,
                extracted_by: context().operator,
                observed_at: Timestamp::EPOCH,
                confidence: Confidence::CERTAIN,
            };
            let valid_time = if disjoint {
                if second {
                    TemporalRange::since(Timestamp::from_millis(10))
                } else {
                    TemporalRange::new(None, Some(Timestamp::from_millis(10))).unwrap()
                }
            } else {
                TemporalRange::UNBOUNDED
            };
            let (predicate, object) = if claim < 2 {
                (
                    Predicate::Property(property),
                    Object::Value(Value::String(
                        if second && !equal { "red" } else { "green" }.into(),
                    )),
                )
            } else {
                (
                    Predicate::Relation(relation),
                    Object::Node(nodes[if second && !equal { 2 } else { 1 }]),
                )
            };
            let assertion = Assertion {
                id: id(401 + ordinal),
                root_id: doc.graph.root.id,
                subject: Subject::Node(nodes[0]),
                predicate,
                object,
                evidence: [evidence.id].into_iter().collect(),
                proposed_by: context().operator,
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time,
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            };
            doc.graph.evidence.insert(evidence.id, evidence);
            doc.evidence_payloads.insert(address, payload.into());
            doc.graph.assertions.insert(assertion.id, assertion);
        }
    }
    doc
}

pub struct Human {
    key: Ed25519KeyPair,
    pub policy: m::ReviewerTrustPolicy,
    pub binding: m::TrustedReviewHostBinding,
}
impl Human {
    pub fn with_answers(mut self) -> Self {
        self.policy.keys[0]
            .scopes
            .insert(0, m::HumanDecisionScope::AnswerAttention);
        self.binding.reviewer_policy_digest = m::ContentHash(
            review::digest(&review::policy_bytes(&self.policy).unwrap()).to_string(),
        );
        self
    }

    pub fn sign(&self, proof: &mut wire::EkrKernelSignedHumanDecision) {
        let semantic = review::proof_from_document(proof).unwrap();
        proof.signature = ekr_core::bytes::encode(
            self.key
                .sign(&review::signing_bytes(&semantic.intent).unwrap())
                .as_ref(),
        );
    }

    pub fn key_digest(&self) -> String {
        review::digest(self.key.public_key().as_ref()).to_string()
    }
    pub fn new(seed: ContentHash) -> Self {
        let key = Ed25519KeyPair::from_seed_unchecked(&[37; 32]).unwrap();
        let document: wire::EkrKernelReviewerTrustPolicy = serde_json::from_value(json!({
            "format": "ekr.reviewer-trust/1", "audience": {"tenant": TENANT, "seed_anchor": seed.to_string()},
            "keys": [{"key_digest": review::digest(key.public_key().as_ref()).to_string(), "algorithm": "Ed25519",
                "public_key": ekr_core::bytes::encode(key.public_key().as_ref()),
                "operator": {"actor": context().operator.to_string(), "authentication_subject": "fixture-human"},
                "scopes": ["UpgradeAuthority"]}]
        })).unwrap();
        let policy = review::policy_from_document(&document).unwrap();
        let binding = m::TrustedReviewHostBinding {
            audience: policy.audience.clone(),
            reviewer_policy_digest: m::ContentHash(
                review::digest(&review::policy_bytes(&policy).unwrap()).to_string(),
            ),
        };
        Self {
            key,
            policy,
            binding,
        }
    }
    pub fn proof(
        &self,
        preview: &wire::EkrKernelUpgradePreview,
    ) -> wire::EkrKernelSignedHumanDecision {
        let mut proof: wire::EkrKernelSignedHumanDecision = serde_json::from_value(json!({
            "algorithm": "Ed25519", "signature": ekr_core::bytes::encode(&[0;64]),
            "intent": {"format": "ekr.human-decision/1", "decision_id": id::<ekr_core::EventId>(800).to_string(),
                "audience": {"tenant": TENANT, "seed_anchor": self.binding.audience.seed_anchor.0},
                "reviewer_policy_digest": self.binding.reviewer_policy_digest.0,
                "signer_key_digest": review::digest(self.key.public_key().as_ref()).to_string(),
                "statement_digest": review::digest(STATEMENT).to_string(),
                "target": {"kind": "UpgradeAuthority", "value": {"preview_digest": preview.preview_digest,
                    "reviewer_policy_digest": self.binding.reviewer_policy_digest.0}}}
        })).unwrap();
        let semantic = review::proof_from_document(&proof).unwrap();
        proof.signature = ekr_core::bytes::encode(
            self.key
                .sign(&review::signing_bytes(&semantic.intent).unwrap())
                .as_ref(),
        );
        proof
    }
}
