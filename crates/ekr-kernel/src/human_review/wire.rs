//! Conversions between generated JSON data and generated review semantics.
use super::{policy_bytes, proof_bytes, refuse, Refusal};
use ekr_core::contract_data as w;
use ekr_core::contracts::{integrate as i, kernel as m, primitives::Uuid};

fn hash(value: &w::EkrKernelContentHash) -> m::ContentHash {
    m::ContentHash(value.0.clone())
}
fn audience(value: &w::EkrKernelHumanDecisionAudience) -> m::HumanDecisionAudience {
    m::HumanDecisionAudience {
        tenant: value.tenant.clone(),
        seed_anchor: hash(&value.seed_anchor),
    }
}
fn bytes(value: &str) -> Result<Vec<u8>, Refusal> {
    ekr_core::bytes::decode(value).map_err(|_| {
        refuse(
            "review-invalid-base64",
            "review bytes must use canonical padded base64",
        )
    })
}
fn basis(value: &w::EkrKernelReviewBasis) -> Result<m::ReviewBasis, Refusal> {
    Ok(m::ReviewBasis {
        observed_revision: m::RevisionNumber(value.observed_revision.0.as_i64().ok_or_else(
            || {
                refuse(
                    "review-invalid-revision",
                    "revision exceeds the command representation",
                )
            },
        )?),
        evidence_digest: hash(&value.evidence_digest),
        options_digest: hash(&value.options_digest),
        effects_digest: hash(&value.effects_digest),
    })
}
fn schema_target(value: &w::EkrKernelSchemaReviewTarget) -> Result<m::SchemaReviewTarget, Refusal> {
    Ok(m::SchemaReviewTarget {
        proposal_id: i::SchemaProposalId(Uuid(value.proposal_id.0.clone())),
        proposal_digest: hash(&value.proposal_digest),
        basis: basis(&value.basis)?,
    })
}
fn target(value: &w::EkrKernelHumanDecisionTarget) -> Result<m::HumanDecisionTarget, Refusal> {
    use w::EkrKernelHumanDecisionTarget as T;
    Ok(match value {
        T::V0(value) => m::HumanDecisionTarget::AnswerAttention(m::AttentionAnswerTarget {
            dispute_id: m::DisputeId(Uuid(value.value.dispute_id.0.clone())),
            basis: basis(&value.value.basis)?,
            corrections_digest: hash(&value.value.corrections_digest),
        }),
        T::V1(value) => m::HumanDecisionTarget::ApproveSchemaProposal(schema_target(&value.value)?),
        T::V2(value) => m::HumanDecisionTarget::RejectSchemaProposal(schema_target(&value.value)?),
        T::V3(value) => m::HumanDecisionTarget::UpgradeAuthority(m::AuthorityUpgradeTarget {
            preview_digest: hash(&value.value.preview_digest),
            reviewer_policy_digest: hash(&value.value.reviewer_policy_digest),
        }),
    })
}

/// Decode a generated JSON policy into the review codec's generated semantic representation.
/// This validates its structure; only an independent host binding can select it as trusted.
/// # Errors
/// Invalid base64, key digest, audience, operator identity or duplicate key/scope.
pub fn policy_from_document(
    value: &w::EkrKernelReviewerTrustPolicy,
) -> Result<m::ReviewerTrustPolicy, Refusal> {
    let policy = m::ReviewerTrustPolicy {
        format: match *value.format {
            w::EkrKernelReviewerTrustFormat::V0 => m::ReviewerTrustFormat::ReviewerTrust1,
        },
        audience: audience(&value.audience),
        keys: value
            .keys
            .iter()
            .map(|key| {
                Ok(m::ReviewerVerificationKey {
                    key_digest: hash(&key.key_digest),
                    algorithm: match *key.algorithm {
                        w::EkrKernelReviewSignatureAlgorithm::V0 => {
                            m::ReviewSignatureAlgorithm::Ed25519
                        }
                    },
                    public_key: bytes(&key.public_key)?,
                    operator: m::TrustedOperatorIdentity {
                        actor: m::AgentId(Uuid(key.operator.actor.0.clone())),
                        authentication_subject: key.operator.authentication_subject.clone(),
                    },
                    scopes: key
                        .scopes
                        .iter()
                        .map(|scope| match **scope {
                            w::EkrKernelHumanDecisionScope::V0 => {
                                m::HumanDecisionScope::AnswerAttention
                            }
                            w::EkrKernelHumanDecisionScope::V1 => {
                                m::HumanDecisionScope::ApproveSchemaProposal
                            }
                            w::EkrKernelHumanDecisionScope::V2 => {
                                m::HumanDecisionScope::RejectSchemaProposal
                            }
                            w::EkrKernelHumanDecisionScope::V3 => {
                                m::HumanDecisionScope::UpgradeAuthority
                            }
                        })
                        .collect(),
                })
            })
            .collect::<Result<_, Refusal>>()?,
    };
    policy_bytes(&policy)?;
    Ok(policy)
}

/// Decode a generated JSON proof without treating its claimed signer as authenticated.
/// Only the kernel's verifier under an independently pinned policy can grant approval.
/// # Errors
/// Invalid base64, UUID, digest, decision target or canonical review field.
pub fn proof_from_document(
    value: &w::EkrKernelSignedHumanDecision,
) -> Result<m::SignedHumanDecision, Refusal> {
    let intent = &value.intent;
    let proof = m::SignedHumanDecision {
        algorithm: match *value.algorithm {
            w::EkrKernelReviewSignatureAlgorithm::V0 => m::ReviewSignatureAlgorithm::Ed25519,
        },
        signature: bytes(&value.signature)?,
        intent: m::HumanDecisionIntent {
            format: match *intent.format {
                w::EkrKernelHumanDecisionFormat::V0 => m::HumanDecisionFormat::HumanDecision1,
            },
            decision_id: Uuid(intent.decision_id.clone()),
            audience: audience(&intent.audience),
            reviewer_policy_digest: hash(&intent.reviewer_policy_digest),
            signer_key_digest: hash(&intent.signer_key_digest),
            target: target(&intent.target)?,
            statement_digest: hash(&intent.statement_digest),
            expected_previous_decision: match &intent.expected_previous_decision {
                w::EssPresence::Absent => None,
                w::EssPresence::Present(value) => Some(hash(value)),
            },
        },
    };
    proof_bytes(&proof)?;
    Ok(proof)
}
