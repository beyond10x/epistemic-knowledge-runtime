//! Exact correction signing and material-basis revalidation for human answers.
use super::{digest, model, refuse, ContentHash, Encoder, Refusal, Reviewer, VerifiedDecision};

/// Canonical ordered correction-list bytes specified by design §105.6.
/// Encoding grants no authority and does not establish that the corrections are applicable.
pub fn corrections_bytes(corrections: &[model::ClaimCorrection]) -> Result<Vec<u8>, Refusal> {
    let mut out = Encoder::default();
    out.0.extend_from_slice(
        &u64::try_from(corrections.len())
            .expect("addressable length")
            .to_be_bytes(),
    );
    for correction in corrections {
        out.string(match correction.kind {
            model::ClaimCorrectionKind::Choose => "Choose",
            model::ClaimCorrectionKind::Retract => "Retract",
            model::ClaimCorrectionKind::CorrectTime => "CorrectTime",
            model::ClaimCorrectionKind::Unresolved => "Unresolved",
        });
        out.uuid(&correction.assertion_id.0 .0)?;
        for bound in [&correction.valid_from, &correction.valid_to] {
            out.0.push(u8::from(bound.is_some()));
            if let Some(bound) = bound {
                let instant = crate::incubation_document::timestamp(&bound.0).map_err(|_| {
                    refuse(
                        "review-invalid-time",
                        "correction times must be RFC 3339 instants at millisecond precision",
                    )
                })?;
                out.0.extend_from_slice(&instant.millis().to_be_bytes());
            }
        }
        out.string(&correction.reason);
    }
    Ok(out.0)
}

impl Reviewer {
    /// Verify an exact signed answer against a freshly projected dispute question.
    /// The caller must obtain `current` from the verified kernel read at publication. This
    /// does not apply corrections or replace validation and atomic predecessor checks.
    pub fn verify_attention(
        &self,
        proof: &model::SignedHumanDecision,
        current: &model::AttentionItem,
        corrections: &[model::ClaimCorrection],
        statement: &[u8],
        previous: Option<ContentHash>,
    ) -> Result<VerifiedDecision, Refusal> {
        let model::HumanDecisionTarget::AnswerAttention(reviewed) = &proof.intent.target else {
            return Err(refuse("review-target", "proof is not an attention answer"));
        };
        let subject = &current.subject;
        let dispute_id = match &subject.dispute_id {
            Some(id)
                if subject.kind == model::AttentionKind::Dispute
                    && subject.blocker_id.is_none()
                    && subject.proposal_id.is_none() =>
            {
                id.clone()
            }
            _ => return Err(refuse("review-target", "current question is not a dispute")),
        };
        // Preserve the signed observed revision. Only material digests are compared with the
        // current projection; changing the signed intent would invalidate its signature.
        let target = model::HumanDecisionTarget::AnswerAttention(model::AttentionAnswerTarget {
            dispute_id,
            basis: reviewed.basis.clone(),
            corrections_digest: model::ContentHash(
                digest(&corrections_bytes(corrections)?).to_string(),
            ),
        });
        let verified = self.verify(proof, &target, statement, previous)?;
        let mut checked = Encoder::default();
        checked.basis(&current.basis)?;
        let prior = &reviewed.basis;
        let now = &current.basis;
        if prior.observed_revision.0 > now.observed_revision.0
            || prior.evidence_digest != now.evidence_digest
            || prior.options_digest != now.options_digest
            || prior.effects_digest != now.effects_digest
        {
            return Err(refuse(
                "answer-review-required",
                "the reviewed evidence, options or effects no longer match current knowledge",
            ));
        }
        Ok(verified)
    }
}
