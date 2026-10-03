//! Exact schema-review targets with material revalidation after unrelated revisions.
use super::{model, refuse, ContentHash, Encoder, Refusal, Reviewer, VerifiedDecision};

impl Reviewer {
    /// Verify the exact proposal, operation and statement against current review material.
    /// The caller supplies kernel-derived material and the retained decision predecessor.
    /// This verifies endorsement only; publication still requires an atomic predecessor check.
    pub fn verify_schema_proposal(
        &self,
        proof: &model::SignedHumanDecision,
        current: &model::SchemaReviewTarget,
        approve: bool,
        statement: &[u8],
        previous: Option<ContentHash>,
    ) -> Result<VerifiedDecision, Refusal> {
        let reviewed = match (&proof.intent.target, approve) {
            (model::HumanDecisionTarget::ApproveSchemaProposal(target), true)
            | (model::HumanDecisionTarget::RejectSchemaProposal(target), false) => target,
            _ => {
                return Err(refuse(
                    "review-target",
                    "proof names a different review operation",
                ))
            }
        };
        let target = model::SchemaReviewTarget {
            proposal_id: current.proposal_id.clone(),
            proposal_digest: current.proposal_digest.clone(),
            basis: reviewed.basis.clone(),
        };
        let target = if approve {
            model::HumanDecisionTarget::ApproveSchemaProposal(target)
        } else {
            model::HumanDecisionTarget::RejectSchemaProposal(target)
        };
        let verified = self.verify(proof, &target, statement, previous)?;
        Encoder::default().basis(&current.basis)?;
        let prior = &reviewed.basis;
        let now = &current.basis;
        if prior.observed_revision.0 > now.observed_revision.0
            || prior.evidence_digest != now.evidence_digest
            || prior.options_digest != now.options_digest
            || prior.effects_digest != now.effects_digest
        {
            return Err(refuse(
                "schema-review-required",
                "the reviewed evidence, options or effects no longer match current knowledge",
            ));
        }
        Ok(verified)
    }
}
