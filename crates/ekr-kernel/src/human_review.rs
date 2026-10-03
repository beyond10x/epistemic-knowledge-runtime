//! Human-review verification over generated ESS contracts (design § 105.6).
//!
//! Host provisioning supplies the trust binding independently of request content. This module
//! never signs, infers human presence, enrolls a policy, or writes a decision. Publication must
//! still atomically check the decision identity, predecessor and freshly computed target.
use ekr_core::contracts::kernel as model;
use ekr_core::{AgentId, ContentHash};
use ring::signature::{UnparsedPublicKey, ED25519};

mod attention;
mod corrections;
mod decode;
mod schema;
mod wire;
pub use attention::corrections_bytes;
pub(crate) use corrections::validate_corrections;
pub use decode::{read_host_binding, read_policy, read_proof};
pub(crate) use wire::basis as basis_from_document;
pub(crate) use wire::correction_from_document;
pub use wire::{answer_from_document, policy_from_document, proof_from_document};

type Refusal = model::KnowledgeRefused;
fn refuse(code: &str, reason: &str) -> Refusal {
    Refusal {
        code: code.into(),
        reason: reason.into(),
    }
}

// This is the review format's explicitly specified codec, not ekr-core's tagged value codec.
// No fields or DTOs are duplicated: all accesses name the generated semantic contracts.
#[derive(Default)]
struct Encoder(Vec<u8>);
impl Encoder {
    fn bytes(&mut self, value: &[u8]) {
        self.0.extend_from_slice(
            &u64::try_from(value.len())
                .expect("addressable length")
                .to_be_bytes(),
        );
        self.0.extend_from_slice(value);
    }
    fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }
    fn hash(&mut self, value: &model::ContentHash) -> Result<(), Refusal> {
        let parsed: ContentHash = value
            .0
            .parse()
            .map_err(|_| refuse("review-invalid-hash", "expected lowercase SHA-256 hex"))?;
        self.0.extend_from_slice(parsed.as_bytes());
        Ok(())
    }
    fn uuid(&mut self, value: &str) -> Result<(), Refusal> {
        let parsed: AgentId = value
            .parse()
            .map_err(|_| refuse("review-invalid-uuid", "expected canonical UUID"))?;
        self.0.extend_from_slice(parsed.to_uuid().as_bytes());
        Ok(())
    }
    fn audience(&mut self, value: &model::HumanDecisionAudience) -> Result<(), Refusal> {
        self.string(&value.tenant);
        self.hash(&value.seed_anchor)
    }
    fn optional_hash(&mut self, value: Option<&model::ContentHash>) -> Result<(), Refusal> {
        self.0.push(u8::from(value.is_some()));
        if let Some(value) = value {
            self.hash(value)?;
        }
        Ok(())
    }
    fn basis(&mut self, value: &model::ReviewBasis) -> Result<(), Refusal> {
        let revision = u64::try_from(value.observed_revision.0)
            .map_err(|_| refuse("review-invalid-revision", "revision cannot be negative"))?;
        self.0.extend_from_slice(&revision.to_be_bytes());
        self.hash(&value.evidence_digest)?;
        self.hash(&value.options_digest)?;
        self.hash(&value.effects_digest)
    }
    fn target(&mut self, value: &model::HumanDecisionTarget) -> Result<(), Refusal> {
        self.string(scope_label(target_scope(value)));
        match value {
            model::HumanDecisionTarget::AnswerAttention(value) => {
                self.uuid(&value.dispute_id.0 .0)?;
                self.basis(&value.basis)?;
                self.hash(&value.corrections_digest)
            }
            model::HumanDecisionTarget::ApproveSchemaProposal(value)
            | model::HumanDecisionTarget::RejectSchemaProposal(value) => {
                self.uuid(&value.proposal_id.0 .0)?;
                self.hash(&value.proposal_digest)?;
                self.basis(&value.basis)
            }
            model::HumanDecisionTarget::UpgradeAuthority(value) => {
                self.hash(&value.preview_digest)?;
                self.hash(&value.reviewer_policy_digest)
            }
        }
    }
    fn intent(&mut self, value: &model::HumanDecisionIntent) -> Result<(), Refusal> {
        let model::HumanDecisionFormat::HumanDecision1 = value.format;
        self.string("ekr.human-decision/1");
        self.uuid(&value.decision_id.0)?;
        self.audience(&value.audience)?;
        self.hash(&value.reviewer_policy_digest)?;
        self.hash(&value.signer_key_digest)?;
        self.target(&value.target)?;
        self.hash(&value.statement_digest)?;
        self.optional_hash(value.expected_previous_decision.as_ref())
    }
}
fn scope_label(scope: model::HumanDecisionScope) -> &'static str {
    match scope {
        model::HumanDecisionScope::AnswerAttention => "AnswerAttention",
        model::HumanDecisionScope::ApproveSchemaProposal => "ApproveSchemaProposal",
        model::HumanDecisionScope::RejectSchemaProposal => "RejectSchemaProposal",
        model::HumanDecisionScope::UpgradeAuthority => "UpgradeAuthority",
    }
}
fn target_scope(value: &model::HumanDecisionTarget) -> model::HumanDecisionScope {
    match value {
        model::HumanDecisionTarget::AnswerAttention(_) => {
            model::HumanDecisionScope::AnswerAttention
        }
        model::HumanDecisionTarget::ApproveSchemaProposal(_) => {
            model::HumanDecisionScope::ApproveSchemaProposal
        }
        model::HumanDecisionTarget::RejectSchemaProposal(_) => {
            model::HumanDecisionScope::RejectSchemaProposal
        }
        model::HumanDecisionTarget::UpgradeAuthority(_) => {
            model::HumanDecisionScope::UpgradeAuthority
        }
    }
}

/// The canonical signing message; independent of JSON and serde field ordering.
pub fn signing_bytes(intent: &model::HumanDecisionIntent) -> Result<Vec<u8>, Refusal> {
    let mut out = Encoder(b"ekr.human-decision/1\0".to_vec());
    out.intent(intent)?;
    Ok(out.0)
}

/// Canonical policy bytes, with key identities and ordering checked.
pub fn policy_bytes(policy: &model::ReviewerTrustPolicy) -> Result<Vec<u8>, Refusal> {
    let mut out = Encoder::default();
    let model::ReviewerTrustFormat::ReviewerTrust1 = policy.format;
    out.string("ekr.reviewer-trust/1");
    out.audience(&policy.audience)?;
    out.0.extend_from_slice(
        &u64::try_from(policy.keys.len())
            .expect("addressable length")
            .to_be_bytes(),
    );
    let mut previous_key: Option<&str> = None;
    for key in &policy.keys {
        if key.public_key.len() != 32 || key.key_digest.0 != digest(&key.public_key).to_string() {
            return Err(refuse(
                "review-invalid-key",
                "key must be 32 bytes with its exact SHA-256 digest",
            ));
        }
        if previous_key.is_some_and(|previous| previous >= key.key_digest.0.as_str()) {
            return Err(refuse(
                "review-key-order",
                "policy keys must be unique and ordered by digest",
            ));
        }
        previous_key = Some(&key.key_digest.0);
        out.hash(&key.key_digest)?;
        let model::ReviewSignatureAlgorithm::Ed25519 = key.algorithm;
        out.string("Ed25519");
        out.bytes(&key.public_key);
        out.uuid(&key.operator.actor.0 .0)?;
        out.string(&key.operator.authentication_subject);
        out.0.extend_from_slice(
            &u64::try_from(key.scopes.len())
                .expect("addressable length")
                .to_be_bytes(),
        );
        let mut previous_scope = None;
        for scope in &key.scopes {
            let label = scope_label(*scope);
            if previous_scope.is_some_and(|previous| previous >= label) {
                return Err(refuse(
                    "review-scope-order",
                    "scopes must be unique and ordered by wire label",
                ));
            }
            previous_scope = Some(label);
            out.string(label);
        }
    }
    Ok(out.0)
}

/// Canonical independently provisioned host-binding bytes, suitable for retained enrollment.
pub fn host_binding_bytes(binding: &model::TrustedReviewHostBinding) -> Result<Vec<u8>, Refusal> {
    let mut out = Encoder::default();
    out.audience(&binding.audience)?;
    out.hash(&binding.reviewer_policy_digest)?;
    Ok(out.0)
}

/// Canonical proof-envelope bytes. Encoding is not verification and grants no authority.
pub fn proof_bytes(proof: &model::SignedHumanDecision) -> Result<Vec<u8>, Refusal> {
    let mut out = Encoder::default();
    out.intent(&proof.intent)?;
    let model::ReviewSignatureAlgorithm::Ed25519 = proof.algorithm;
    out.string("Ed25519");
    if proof.signature.len() != 64 {
        return Err(refuse(
            "review-invalid-signature",
            "Ed25519 signature must contain 64 bytes",
        ));
    }
    out.bytes(&proof.signature);
    Ok(out.0)
}

/// SHA-256 for the raw-digest review protocol, distinct from EKR's domain-addressed payloads.
#[must_use]
pub fn digest(bytes: &[u8]) -> ContentHash {
    ContentHash::from_bytes(
        ring::digest::digest(&ring::digest::SHA256, bytes)
            .as_ref()
            .try_into()
            .expect("SHA-256"),
    )
}

/// A verifier selected by trusted host provisioning, never by a command's supplied policy.
pub struct Reviewer {
    policy: model::ReviewerTrustPolicy,
    policy_digest: ContentHash,
    canonical_policy: Vec<u8>,
}
impl Reviewer {
    /// Check an independently provisioned host binding against the selected public policy.
    /// Callers must obtain `binding` outside the agent's request/write authority.
    pub fn from_host(
        binding: &model::TrustedReviewHostBinding,
        policy: model::ReviewerTrustPolicy,
    ) -> Result<Self, Refusal> {
        host_binding_bytes(binding)?;
        let canonical_policy = policy_bytes(&policy)?;
        let policy_digest = digest(&canonical_policy);
        if binding.audience != policy.audience
            || binding.reviewer_policy_digest.0 != policy_digest.to_string()
        {
            return Err(refuse(
                "review-host-binding",
                "selected policy does not match the independent host binding",
            ));
        }
        Ok(Self {
            policy,
            policy_digest,
            canonical_policy,
        })
    }

    /// Verify the exact command target, statement and predecessor against the selected policy.
    /// The target and predecessor must come from the kernel's current retained state.
    pub fn verify(
        &self,
        proof: &model::SignedHumanDecision,
        target: &model::HumanDecisionTarget,
        statement: &[u8],
        previous: Option<ContentHash>,
    ) -> Result<VerifiedDecision, Refusal> {
        let message = signing_bytes(&proof.intent)?;
        let canonical = proof_bytes(proof)?;
        let intent = &proof.intent;
        if intent.audience != self.policy.audience
            || intent.reviewer_policy_digest.0 != self.policy_digest.to_string()
        {
            return Err(refuse(
                "review-audience",
                "proof does not name the enrolled audience and policy",
            ));
        }
        if &intent.target != target || intent.statement_digest.0 != digest(statement).to_string() {
            return Err(refuse(
                "review-target",
                "proof does not cover the exact target and statement",
            ));
        }
        if let model::HumanDecisionTarget::UpgradeAuthority(upgrade) = target {
            if upgrade.reviewer_policy_digest.0 != self.policy_digest.to_string() {
                return Err(refuse(
                    "review-upgrade-policy",
                    "upgrade must enroll the independently selected policy",
                ));
            }
        }
        let previous = previous.map(|hash| model::ContentHash(hash.to_string()));
        if intent.expected_previous_decision != previous {
            return Err(refuse(
                "review-predecessor",
                "a different decision is currently effective",
            ));
        }
        let key = self
            .policy
            .keys
            .iter()
            .find(|key| key.key_digest == intent.signer_key_digest)
            .ok_or_else(|| {
                refuse(
                    "review-untrusted-key",
                    "signer is absent from the selected policy",
                )
            })?;
        if !key.scopes.contains(&target_scope(target)) {
            return Err(refuse(
                "review-scope",
                "signer is not authorized for this decision scope",
            ));
        }
        UnparsedPublicKey::new(&ED25519, &key.public_key)
            .verify(&message, &proof.signature)
            .map_err(|_| {
                refuse(
                    "review-signature",
                    "signature does not verify over the exact intent",
                )
            })?;
        Ok(VerifiedDecision {
            proof: proof.clone(),
            operator: key.operator.clone(),
            canonical,
            policy: self.canonical_policy.clone(),
            statement: statement.to_vec(),
        })
    }
}

/// Verified endorsement of one exact intent. It is neither deserializable nor constructible by
/// a caller. This alone does not publish a decision or bypass the atomic predecessor check.
///
/// ```compile_fail
/// use ekr_kernel::human_review::VerifiedDecision;
/// let forged: VerifiedDecision = serde_json::from_str("{}").unwrap();
/// ```
pub struct VerifiedDecision {
    proof: model::SignedHumanDecision,
    operator: model::TrustedOperatorIdentity,
    canonical: Vec<u8>,
    policy: Vec<u8>,
    statement: Vec<u8>,
}
impl VerifiedDecision {
    /// The operator identity selected by the enrolled public policy.
    #[must_use]
    pub fn operator(&self) -> &model::TrustedOperatorIdentity {
        &self.operator
    }
    /// The exact verified intent, including the predecessor to check at publication.
    #[must_use]
    pub fn intent(&self) -> &model::HumanDecisionIntent {
        &self.proof.intent
    }
    /// Exact bytes that must be retained for this proof.
    #[must_use]
    pub fn canonical_proof(&self) -> &[u8] {
        &self.canonical
    }
    /// The protocol digest of the verified proof envelope.
    #[must_use]
    pub fn proof_digest(&self) -> ContentHash {
        digest(&self.canonical)
    }
    /// Canonical public policy bytes that endorsed this proof.
    #[must_use]
    pub fn canonical_policy(&self) -> &[u8] {
        &self.policy
    }
    /// The exact human statement covered by this proof.
    #[must_use]
    pub fn statement(&self) -> &[u8] {
        &self.statement
    }
    /// Project both the protocol digests and actual StoredObject addresses for publication.
    /// The supplied recording time is checked; this method reads no clock and writes no store.
    pub fn record(
        &self,
        at: ekr_core::contracts::primitives::Timestamp,
    ) -> Result<model::HumanDecisionRecord, Refusal> {
        crate::incubation_document::timestamp(&at.0).map_err(|_| {
            refuse(
                "review-invalid-time",
                "recorded time must be an RFC 3339 instant at millisecond precision",
            )
        })?;
        let hash = |bytes: &[u8]| model::ContentHash(ContentHash::of_bytes(bytes).to_string());
        Ok(model::HumanDecisionRecord {
            proof_object_hash: hash(&self.canonical),
            policy_object_hash: hash(&self.policy),
            statement_object_hash: hash(&self.statement),
            decision_id: self.proof.intent.decision_id.clone(),
            proof_digest: model::ContentHash(self.proof_digest().to_string()),
            policy_digest: self.proof.intent.reviewer_policy_digest.clone(),
            statement_digest: self.proof.intent.statement_digest.clone(),
            operator: self.operator.clone(),
            recorded_at: at,
        })
    }
}
