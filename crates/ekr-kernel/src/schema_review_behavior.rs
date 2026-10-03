//! Generated human-review obligations over the kernel admission path.
use crate::{human_review as h, schema_proposals::error, Commit};
use ekr_core::contracts::integrate::obligations::{
    ApproveSchemaProposalBehavior, RejectSchemaProposalBehavior,
};
use ekr_core::{
    contract_data as w,
    contracts::{integrate as m, kernel as k, obligation::UnmetObligation, primitives::Uuid},
    Timestamp,
};
use ekr_store::{
    HumanDecisionRetention, IncubationRetention, ObjectStore, ObservationRetention,
    ProposalReviewRetention, RevisionLog, SchemaProposalRetention, StoreError,
};

struct Behavior<'a, S: RevisionLog + ObjectStore> {
    commit: &'a Commit<S>,
    input: &'a w::EkrIntegrateSchemaProposalReviewApplication,
    at: Timestamp,
    held: Option<w::EkrIntegrateProposalReviewSnapshot>,
    fault: Option<StoreError>,
}

fn approve_input(
    input: &w::EkrIntegrateSchemaProposalReviewApplication,
) -> Result<m::ApproveSchemaProposal, StoreError> {
    Ok(m::ApproveSchemaProposal {
        human_proof: h::proof_from_document(&input.human_proof).map_err(|e| error(e.reason))?,
        proposal_id: m::SchemaProposalId(Uuid(input.proposal_id.0.clone())),
        proposal_digest: k::ContentHash(input.proposal_digest.0.clone()),
        basis: h::basis_from_document(&input.basis).map_err(|e| error(e.reason))?,
        statement: ekr_core::bytes::decode(&input.statement).map_err(error)?,
    })
}
impl<
        S: RevisionLog
            + ObjectStore
            + ObservationRetention
            + IncubationRetention
            + SchemaProposalRetention
            + HumanDecisionRetention
            + ProposalReviewRetention,
    > ApproveSchemaProposalBehavior for Behavior<'_, S>
{
    fn approve_schema_proposal(
        &mut self,
        input: m::ApproveSchemaProposal,
    ) -> Result<m::ApproveSchemaProposalOutcome, UnmetObligation> {
        let result = (|| {
            if approve_input(self.input)? != input {
                return Err(error("generated review input differs from transport"));
            }
            self.commit
                .retain_schema_proposal_review(self.input, true, self.at)
        })();
        match result {
            Ok(held) => {
                let result = m::ApproveSchemaProposalResult {
                    review_id: m::ProposalReviewId(Uuid(held.review_id.0.clone())),
                };
                self.held = Some(held);
                Ok(m::ApproveSchemaProposalOutcome::Answered {
                    approve_schema_proposal_result: result,
                })
            }
            Err(
                fault @ (StoreError::Document(_)
                | StoreError::Conflict
                | StoreError::PublicationInputConflict),
            ) => {
                let refusal = m::KnowledgeRefused {
                    code: "schema-review-refused".into(),
                    reason: fault.to_string(),
                };
                self.fault = Some(fault);
                Ok(m::ApproveSchemaProposalOutcome::Refused { error: refusal })
            }
            Err(fault) => {
                self.fault = Some(fault);
                Err(UnmetObligation {
                    capability: "available review persistence",
                    source: "ekr.integrate.ApproveSchemaProposal",
                })
            }
        }
    }
}
pub(super) fn approve<
    S: RevisionLog
        + ObjectStore
        + ObservationRetention
        + IncubationRetention
        + SchemaProposalRetention
        + HumanDecisionRetention
        + ProposalReviewRetention,
>(
    commit: &Commit<S>,
    input: &w::EkrIntegrateSchemaProposalReviewApplication,
    at: Timestamp,
) -> Result<w::EkrIntegrateProposalReviewSnapshot, StoreError> {
    let mut behavior = Behavior {
        commit,
        input,
        at,
        held: None,
        fault: None,
    };
    let result = behavior.approve_schema_proposal(approve_input(input)?);
    if let Some(fault) = behavior.fault {
        return Err(fault);
    }
    match result.map_err(error)? {
        m::ApproveSchemaProposalOutcome::Answered {
            approve_schema_proposal_result: result,
        } => {
            let held = behavior
                .held
                .ok_or_else(|| error("missing review result"))?;
            if held.review_id.0 != result.review_id.0 .0 {
                return Err(error("generated review result mismatch"));
            }
            Ok(held)
        }
        m::ApproveSchemaProposalOutcome::Refused { error: refusal } => Err(error(refusal.reason)),
    }
}

fn reject_input(
    input: &w::EkrIntegrateSchemaProposalReviewApplication,
) -> Result<m::RejectSchemaProposal, StoreError> {
    Ok(m::RejectSchemaProposal {
        human_proof: h::proof_from_document(&input.human_proof).map_err(|e| error(e.reason))?,
        proposal_id: m::SchemaProposalId(Uuid(input.proposal_id.0.clone())),
        proposal_digest: k::ContentHash(input.proposal_digest.0.clone()),
        basis: h::basis_from_document(&input.basis).map_err(|e| error(e.reason))?,
        statement: ekr_core::bytes::decode(&input.statement).map_err(error)?,
    })
}
impl<
        S: RevisionLog
            + ObjectStore
            + ObservationRetention
            + IncubationRetention
            + SchemaProposalRetention
            + HumanDecisionRetention
            + ProposalReviewRetention,
    > RejectSchemaProposalBehavior for Behavior<'_, S>
{
    fn reject_schema_proposal(
        &mut self,
        input: m::RejectSchemaProposal,
    ) -> Result<m::RejectSchemaProposalOutcome, UnmetObligation> {
        let result = (|| {
            if reject_input(self.input)? != input {
                return Err(error("generated review input differs from transport"));
            }
            self.commit
                .retain_schema_proposal_review(self.input, false, self.at)
        })();
        match result {
            Ok(held) => {
                let result = m::RejectSchemaProposalResult {
                    review_id: m::ProposalReviewId(Uuid(held.review_id.0.clone())),
                };
                self.held = Some(held);
                Ok(m::RejectSchemaProposalOutcome::Answered {
                    reject_schema_proposal_result: result,
                })
            }
            Err(
                fault @ (StoreError::Document(_)
                | StoreError::Conflict
                | StoreError::PublicationInputConflict),
            ) => {
                let refusal = m::KnowledgeRefused {
                    code: "schema-review-refused".into(),
                    reason: fault.to_string(),
                };
                self.fault = Some(fault);
                Ok(m::RejectSchemaProposalOutcome::Refused { error: refusal })
            }
            Err(fault) => {
                self.fault = Some(fault);
                Err(UnmetObligation {
                    capability: "available review persistence",
                    source: "ekr.integrate.RejectSchemaProposal",
                })
            }
        }
    }
}
pub(super) fn reject<
    S: RevisionLog
        + ObjectStore
        + ObservationRetention
        + IncubationRetention
        + SchemaProposalRetention
        + HumanDecisionRetention
        + ProposalReviewRetention,
>(
    commit: &Commit<S>,
    input: &w::EkrIntegrateSchemaProposalReviewApplication,
    at: Timestamp,
) -> Result<w::EkrIntegrateProposalReviewSnapshot, StoreError> {
    let mut behavior = Behavior {
        commit,
        input,
        at,
        held: None,
        fault: None,
    };
    let result = behavior.reject_schema_proposal(reject_input(input)?);
    if let Some(fault) = behavior.fault {
        return Err(fault);
    }
    match result.map_err(error)? {
        m::RejectSchemaProposalOutcome::Answered {
            reject_schema_proposal_result: result,
        } => {
            let held = behavior
                .held
                .ok_or_else(|| error("missing review result"))?;
            if held.review_id.0 != result.review_id.0 .0 {
                return Err(error("generated review result mismatch"));
            }
            Ok(held)
        }
        m::RejectSchemaProposalOutcome::Refused { error: refusal } => Err(error(refusal.reason)),
    }
}
