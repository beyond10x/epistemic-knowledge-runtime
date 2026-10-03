//! Inverse of the declared review codec, with exact consumption and bounded lengths.
use super::{host_binding_bytes, policy_bytes, proof_bytes, refuse, Refusal};
use ekr_core::contracts::{integrate, kernel as m, primitives::Uuid};
use ekr_core::ContentHash;

struct Reader<'a> {
    rest: &'a [u8],
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Refusal> {
        let (value, rest) = self.rest.split_at_checked(count).ok_or_else(|| {
            refuse(
                "review-truncated",
                "canonical review document ended inside a field",
            )
        })?;
        self.rest = rest;
        Ok(value)
    }
    fn u64(&mut self) -> Result<u64, Refusal> {
        Ok(u64::from_be_bytes(
            self.take(8)?.try_into().expect("eight bytes"),
        ))
    }
    fn length(&mut self) -> Result<usize, Refusal> {
        let length = usize::try_from(self.u64()?)
            .map_err(|_| refuse("review-length", "length exceeds address space"))?;
        // Every supported list element occupies at least one byte. This also bounds loops and
        // allocations before interpreting attacker-controlled lengths.
        if length > self.rest.len() {
            return Err(refuse("review-length", "length exceeds remaining document"));
        }
        Ok(length)
    }
    fn bytes(&mut self) -> Result<Vec<u8>, Refusal> {
        let length = self.length()?;
        Ok(self.take(length)?.to_vec())
    }
    fn string(&mut self) -> Result<String, Refusal> {
        String::from_utf8(self.bytes()?)
            .map_err(|_| refuse("review-utf8", "review text is not UTF-8"))
    }
    fn label(&mut self, expected: &str) -> Result<(), Refusal> {
        if self.string()? != expected {
            return Err(refuse(
                "review-label",
                "unsupported review format or algorithm",
            ));
        }
        Ok(())
    }
    fn uuid(&mut self) -> Result<Uuid, Refusal> {
        let bits = u128::from_be_bytes(self.take(16)?.try_into().expect("sixteen bytes"));
        Ok(Uuid(format!(
            "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
            bits >> 96,
            (bits >> 80) & 0xffff,
            (bits >> 64) & 0xffff,
            (bits >> 48) & 0xffff,
            bits & 0xffffffffffff
        )))
    }
    fn hash(&mut self) -> Result<m::ContentHash, Refusal> {
        let bytes = self.take(32)?.try_into().expect("thirty-two bytes");
        Ok(m::ContentHash(ContentHash::from_bytes(bytes).to_string()))
    }
    fn optional_hash(&mut self) -> Result<Option<m::ContentHash>, Refusal> {
        match self.take(1)?[0] {
            0 => Ok(None),
            1 => Ok(Some(self.hash()?)),
            _ => Err(refuse(
                "review-option",
                "optional value must be absent or present",
            )),
        }
    }
    fn audience(&mut self) -> Result<m::HumanDecisionAudience, Refusal> {
        Ok(m::HumanDecisionAudience {
            tenant: self.string()?,
            seed_anchor: self.hash()?,
        })
    }
    fn basis(&mut self) -> Result<m::ReviewBasis, Refusal> {
        let revision = i64::try_from(self.u64()?).map_err(|_| {
            refuse(
                "review-invalid-revision",
                "revision exceeds the generated contract's integer range",
            )
        })?;
        Ok(m::ReviewBasis {
            observed_revision: m::RevisionNumber(revision),
            evidence_digest: self.hash()?,
            options_digest: self.hash()?,
            effects_digest: self.hash()?,
        })
    }
    fn scope(&mut self) -> Result<m::HumanDecisionScope, Refusal> {
        match self.string()?.as_str() {
            "AnswerAttention" => Ok(m::HumanDecisionScope::AnswerAttention),
            "ApproveSchemaProposal" => Ok(m::HumanDecisionScope::ApproveSchemaProposal),
            "RejectSchemaProposal" => Ok(m::HumanDecisionScope::RejectSchemaProposal),
            "UpgradeAuthority" => Ok(m::HumanDecisionScope::UpgradeAuthority),
            _ => Err(refuse("review-scope", "unknown decision scope")),
        }
    }
    fn schema_target(&mut self) -> Result<m::SchemaReviewTarget, Refusal> {
        Ok(m::SchemaReviewTarget {
            proposal_id: integrate::SchemaProposalId(self.uuid()?),
            proposal_digest: self.hash()?,
            basis: self.basis()?,
        })
    }
    fn target(&mut self) -> Result<m::HumanDecisionTarget, Refusal> {
        Ok(match self.scope()? {
            m::HumanDecisionScope::AnswerAttention => {
                m::HumanDecisionTarget::AnswerAttention(m::AttentionAnswerTarget {
                    dispute_id: m::DisputeId(self.uuid()?),
                    basis: self.basis()?,
                    corrections_digest: self.hash()?,
                })
            }
            m::HumanDecisionScope::ApproveSchemaProposal => {
                m::HumanDecisionTarget::ApproveSchemaProposal(self.schema_target()?)
            }
            m::HumanDecisionScope::RejectSchemaProposal => {
                m::HumanDecisionTarget::RejectSchemaProposal(self.schema_target()?)
            }
            m::HumanDecisionScope::UpgradeAuthority => {
                m::HumanDecisionTarget::UpgradeAuthority(m::AuthorityUpgradeTarget {
                    preview_digest: self.hash()?,
                    reviewer_policy_digest: self.hash()?,
                })
            }
        })
    }
    fn intent(&mut self) -> Result<m::HumanDecisionIntent, Refusal> {
        self.label("ekr.human-decision/1")?;
        Ok(m::HumanDecisionIntent {
            format: m::HumanDecisionFormat::HumanDecision1,
            decision_id: self.uuid()?,
            audience: self.audience()?,
            reviewer_policy_digest: self.hash()?,
            signer_key_digest: self.hash()?,
            target: self.target()?,
            statement_digest: self.hash()?,
            expected_previous_decision: self.optional_hash()?,
        })
    }
    fn key(&mut self) -> Result<m::ReviewerVerificationKey, Refusal> {
        let key_digest = self.hash()?;
        self.label("Ed25519")?;
        let public_key = self.bytes()?;
        let operator = m::TrustedOperatorIdentity {
            actor: m::AgentId(self.uuid()?),
            authentication_subject: self.string()?,
        };
        let count = self.length()?;
        let mut scopes = Vec::new();
        for _ in 0..count {
            scopes.push(self.scope()?);
        }
        Ok(m::ReviewerVerificationKey {
            key_digest,
            algorithm: m::ReviewSignatureAlgorithm::Ed25519,
            public_key,
            operator,
            scopes,
        })
    }
    fn finish(self) -> Result<(), Refusal> {
        if !self.rest.is_empty() {
            return Err(refuse(
                "review-trailing-bytes",
                "canonical review document has trailing bytes",
            ));
        }
        Ok(())
    }
}

/// Decode retained policy bytes, checking scalars, key digests, uniqueness and ordering.
/// This returns generated data, never a trusted capability.
pub fn read_policy(bytes: &[u8]) -> Result<m::ReviewerTrustPolicy, Refusal> {
    let mut reader = Reader { rest: bytes };
    reader.label("ekr.reviewer-trust/1")?;
    let audience = reader.audience()?;
    let count = reader.length()?;
    let mut keys = Vec::new();
    for _ in 0..count {
        keys.push(reader.key()?);
    }
    reader.finish()?;
    let policy = m::ReviewerTrustPolicy {
        format: m::ReviewerTrustFormat::ReviewerTrust1,
        audience,
        keys,
    };
    policy_bytes(&policy)?;
    Ok(policy)
}

/// Decode a canonical proof envelope, without claiming signature validity or approval.
pub fn read_proof(bytes: &[u8]) -> Result<m::SignedHumanDecision, Refusal> {
    let mut reader = Reader { rest: bytes };
    let intent = reader.intent()?;
    reader.label("Ed25519")?;
    let signature = reader.bytes()?;
    reader.finish()?;
    let proof = m::SignedHumanDecision {
        intent,
        algorithm: m::ReviewSignatureAlgorithm::Ed25519,
        signature,
    };
    proof_bytes(&proof)?;
    Ok(proof)
}

/// Decode retained host-binding bytes. The host must still independently select this binding;
/// reading a requester-supplied document does not provision trust.
pub fn read_host_binding(bytes: &[u8]) -> Result<m::TrustedReviewHostBinding, Refusal> {
    let mut reader = Reader { rest: bytes };
    let binding = m::TrustedReviewHostBinding {
        audience: reader.audience()?,
        reviewer_policy_digest: reader.hash()?,
    };
    reader.finish()?;
    host_binding_bytes(&binding)?;
    Ok(binding)
}
