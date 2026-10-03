//! Native durable command election for signed answers.
use super::durable::{self, bytes, decode};
use super::{error, reviewed};
use crate::{Commit, CommitError};
use ekr_core::contract_data::EkrKernelHumanAnswerRecord;
use ekr_core::contracts::{graph as g, kernel as m, primitives::Uuid};
use ekr_core::{
    AssertionId, ContentHash, EventId, EvidenceId, RevisionId, Timestamp, TransactionId,
};
use ekr_store::{
    ObjectStore, PublicationCommandKey, PublicationCommandKind, PublicationObject, RevisionLog,
    StorageClass, StoreError,
};

fn material(input: &m::AttentionAnswerApplication) -> Result<Vec<u8>, StoreError> {
    // The proof commits the material basis and effects, but redundant request fields must agree.
    let m::HumanDecisionTarget::AnswerAttention(target) = &input.human_proof.intent.target else {
        return Err(error("review-target"));
    };
    crate::replay::require(
        input.dispute_id == target.dispute_id && input.basis == target.basis,
        "answer-input-disagrees-with-proof",
    )?;
    bytes(&(
        reviewed(crate::human_review::proof_bytes(&input.human_proof))?,
        reviewed(crate::human_review::corrections_bytes(&input.corrections))?,
        &input.statement,
    ))
}
impl<S: RevisionLog + ObjectStore> Commit<S> {
    /// Apply an exactly signed answer through retained reviewer policy and ordinary validators.
    /// Returns the immutable original record on an exact retry, including when the question has
    /// settled. A publication conflict may be retried with the same proof; changed material basis
    /// requires a fresh human review. The clock is used only when electing a new attempt.
    /// # Errors
    /// Missing review authority, invalid/stale proof, invalid corrections or provider failure.
    pub fn answer_attention(
        &self,
        input: &m::AttentionAnswerApplication,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<EkrKernelHumanAnswerRecord, CommitError> {
        // A fresh material review reads existing claim evidence, even when incremental replay
        // already admitted those bytes and would otherwise omit them from its working history.
        let history = self.store.history()?;
        let state = self
            .authority
            .reconstruct(&history, None, None)?
            .ok_or(CommitError::NotSeeded)?;
        let material = material(input)?;
        let answer_id: EventId = input
            .human_proof
            .intent
            .decision_id
            .0
            .parse()
            .map_err(error)?;
        if let Some(record) = state
            .answers
            .values()
            .find(|record| record.answer_id.0 == answer_id.to_string())
        {
            if material != self::material(&durable::input(&history, record)?)? {
                return Err(StoreError::PublicationInputConflict.into());
            }
            return Ok(record.clone());
        }
        let predecessor = history.occurrences.last().ok_or(StoreError::NotSeeded)?;
        let key = PublicationCommandKey {
            kind: PublicationCommandKind::AnswerAttention,
            answer_id: Some(answer_id.into()),
            transaction_id: None,
            predecessor_event_id: Some(predecessor.event.event_id),
            predecessor_record_hash: Some(predecessor.event.record_hash),
        };
        let input_hash = crate::commands::input_hash(
            "AnswerAttention",
            &material,
            self.authority.context.operator,
            &self.authority,
        );
        let prepared = if let Some(pending) = self.pending(&key, input_hash)? {
            pending
        } else {
            let at = now();
            let transaction_id = TransactionId::mint();
            let evidence_id = EvidenceId::mint();
            let replacements = input
                .corrections
                .iter()
                .filter(|correction| correction.kind == m::ClaimCorrectionKind::CorrectTime)
                .map(|correction| m::ClaimReplacement {
                    previous: correction.assertion_id.clone(),
                    replacement: g::AssertionId(Uuid(AssertionId::mint().to_string())),
                })
                .collect::<Vec<_>>();
            let (validated, proof) = self.authority.validate_answer(
                &history,
                &state,
                input,
                &replacements,
                (transaction_id, evidence_id, at),
            )?;
            let event_id = EventId::mint();
            let (record, _, root, _) = durable::record(
                &self.authority,
                &history,
                &state,
                input,
                &replacements,
                &validated,
                &proof,
                (event_id, RevisionId::mint(), evidence_id),
                at,
            )?;
            let mut decision = crate::commands::publication(
                event_id,
                durable::payload(&record, &root)?,
                bytes(&record)?,
                at,
                state.version,
            );
            for retained in [
                proof.canonical_policy().to_vec(),
                proof.canonical_proof().to_vec(),
                input.statement.clone(),
                bytes(validated.transaction())?,
            ] {
                decision.objects.insert(
                    ContentHash::of_bytes(&retained),
                    PublicationObject {
                        bytes: retained,
                        storage_class: StorageClass::Canonical,
                        stored_at: at,
                    },
                );
            }
            self.store.prepare(&key, input_hash, &decision, None)?
        };
        // Admission inside resume independently replays every retained input. A changed predecessor
        // needs a fresh command slot, not a mutation of this elected attempt's predecessor.
        let prepared = self.drive(prepared, |_, _| Err(StoreError::Conflict.into()))?;
        Ok(decode(crate::commands::elected_bytes(&prepared)?)?)
    }
    /// Immutable answer records in revision order, optionally restricted to one question.
    /// # Errors
    /// Retained history or reviewer authority cannot be verified.
    pub fn answer_history(
        &self,
        dispute: Option<&m::DisputeId>,
    ) -> Result<Vec<EkrKernelHumanAnswerRecord>, CommitError> {
        let (_, state) = self.replayed_state()?;
        Ok(state
            .answers
            .values()
            .filter(|record| dispute.is_none_or(|id| id.0 .0 == record.dispute_id.0))
            .cloned()
            .collect())
    }
}
