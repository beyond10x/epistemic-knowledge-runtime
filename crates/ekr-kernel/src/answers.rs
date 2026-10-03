//! Admission shared by answer publication and replay. No ordinary validation profile opts in.
use crate::human_review::{self as review, VerifiedDecision};
use crate::replay::{self, ReplayState};
use crate::{
    EvidenceAddition, GraphOperation, GraphTransaction, KernelAuthority, Pipeline,
    ValidatedTransaction,
};
use ekr_core::contracts::kernel as m;
use ekr_core::{ContentHash, EvidenceId, Timestamp, TransactionId};
use ekr_graph::{Confidence, Evidence, EvidenceSource, GraphSnapshot};
use ekr_store::{RetainedHistory, StorageClass, StoreError};

fn error(detail: impl std::fmt::Display) -> StoreError {
    StoreError::Document(format!("attention-answer: {detail}"))
}
fn reviewed<T>(result: Result<T, m::KnowledgeRefused>) -> Result<T, StoreError> {
    result.map_err(|refusal| error(format!("{}: {}", refusal.code, refusal.reason)))
}

impl KernelAuthority {
    /// Validates a derived ordinary transaction using only enrolled retained policy and the
    /// current verified stream state. Allocated IDs/time are fixed by the publication attempt.
    /// The returned sealed transaction is still not a publication or an answer receipt.
    pub(crate) fn validate_answer(
        &self,
        history: &RetainedHistory,
        state: &ReplayState,
        input: &m::AttentionAnswerApplication,
        replacements: &[m::ClaimReplacement],
        allocation: (TransactionId, EvidenceId, Timestamp),
    ) -> Result<(ValidatedTransaction, VerifiedDecision), StoreError> {
        let transition = state
            .transition
            .as_ref()
            .ok_or_else(|| error("authority-upgrade-required"))?;
        let active = state.active_authority(&self.anchor);
        replay::require(
            active.validation_profile.disputes(),
            "answer-authority-profile",
        )?;
        let prior = state.head();
        let graph = prior.graph()?;
        let (transaction_id, statement_id, at) = allocation;
        replay::require(at >= prior.committed_at, "answer-time-precedes-head")?;
        replay::require(
            !state.transactions.contains_key(&transaction_id)
                && !state
                    .answers
                    .values()
                    .any(|answer| answer.transaction_id.0 == transaction_id.to_string()),
            "answer-transaction-identity-reused",
        )?;
        replay::require(
            !state
                .answers
                .values()
                .any(|answer| answer.answer_id.0 == input.human_proof.intent.decision_id.0),
            "answer-decision-identity-reused",
        )?;
        let policy_hash: ContentHash = transition
            .review
            .policy_object_hash
            .0
            .parse()
            .map_err(error)?;
        let policy = reviewed(review::read_policy(
            history.content(policy_hash, StorageClass::Canonical)?,
        ))?;
        let reviewer = self.reviewer(state, &policy)?;
        let m::HumanDecisionTarget::AnswerAttention(target) = &input.human_proof.intent.target
        else {
            return Err(error("review-target"));
        };
        replay::require(
            input.dispute_id == target.dispute_id && input.basis == target.basis,
            "answer-input-disagrees-with-proof",
        )?;
        let previous = state
            .answers
            .values()
            .rev()
            .find(|answer| answer.dispute_id.0 == input.dispute_id.0 .0)
            .map(|answer| {
                answer
                    .review
                    .proof_digest
                    .0
                    .parse::<ContentHash>()
                    .map_err(error)
            })
            .transpose()?;
        let questions = crate::attention::disputes_at(graph, prior.root.revision, |hash| {
            history.content(*hash, StorageClass::Provenance)
        })?;
        let current = questions
            .iter()
            .map(crate::attention_behavior::item)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .find(|item| item.subject.dispute_id.as_ref() == Some(&input.dispute_id))
            .ok_or_else(|| error("answer-review-required: question is no longer current"))?;
        let proof = reviewed(reviewer.verify_attention(
            &input.human_proof,
            &current,
            &input.corrections,
            &input.statement,
            previous,
        ))?;
        let operator = proof.operator().actor.0 .0.parse().map_err(error)?;
        replay::require(
            active.agents.contains_key(&operator),
            "answer-operator-unregistered",
        )?;
        let mut operations = reviewed(proof.corrections_at(
            graph,
            &current,
            &input.corrections,
            replacements,
            statement_id,
        ))?;
        let withdrawals = operations
            .iter()
            .filter_map(|op| match op {
                GraphOperation::RetractAssertion(change) => Some(change.assertion),
                _ => None,
            })
            .collect();
        let evidence = operations
            .iter()
            .flat_map(|op| match op {
                GraphOperation::AddAssertion(claim) => {
                    claim.evidence.iter().copied().collect::<Vec<_>>()
                }
                _ => Vec::new(),
            })
            .collect();
        operations.push(GraphOperation::AddEvidence(Box::new(EvidenceAddition {
            evidence: Evidence {
                id: statement_id,
                source: EvidenceSource::HumanStatement {
                    identity: Some(proof.operator().authentication_subject.clone()),
                },
                content_hash: ContentHash::of_bytes(proof.statement()),
                extracted_by: operator,
                observed_at: at,
                confidence: Confidence::CERTAIN,
            },
            payload: proof.statement().to_vec(),
        })));
        let transaction = GraphTransaction {
            id: transaction_id,
            proposer: operator,
            operations,
            evidence,
            schema_version: None,
        };
        let lineage = crate::validate::schema::lineage(
            state.revisions.values().map(|revision| &*revision.ontology),
        );
        let pipeline = Pipeline::reviewed_answer(
            self.context.validator,
            lineage,
            state.held.at(prior.root.revision),
            withdrawals,
        );
        let validated = pipeline
            .validate(&GraphSnapshot::of(graph), &transaction)
            .map_err(|issues| {
                error(
                    serde_json::to_string(&issues)
                        .unwrap_or_else(|_| "validation serialization".into()),
                )
            })?;
        Ok((validated, proof))
    }
}

#[cfg(test)]
#[allow(dead_code, unused_imports)]
mod tests;

mod durable;
pub(crate) use durable::{replay_answer, required};

mod publish;
