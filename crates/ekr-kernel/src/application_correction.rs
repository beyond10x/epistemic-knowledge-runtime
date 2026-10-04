//! The exact reviewed final correction, frozen before ordinary transaction admission.
use crate::{
    human_review::VerifiedDecision, schema_proposals::error, GraphOperation, GraphTransaction,
    VerifiedRead,
};
use ekr_core::{
    contract_data as w,
    contracts::{graph as g, kernel as m, primitives::Uuid},
    generated_identity::{ApplicationStepId, Identity},
    AssertionId, EvidenceId, Timestamp, TransactionId,
};
use ekr_graph::{CanonicalValue, TransactionTime};
use ekr_store::StoreError;

pub(crate) struct Inputs<'a> {
    pub read: &'a VerifiedRead,
    pub proposal: &'a w::EkrIntegrateRetainedSchemaProposal,
    pub review: &'a w::EkrIntegrateRetainedProposalReview,
    pub decision: &'a VerifiedDecision,
    pub election: &'a w::EkrIntegrateRetainedApplicationElection,
}

pub(crate) fn unresolved(proposal: &w::EkrIntegrateSchemaProposalDocument) -> bool {
    proposal
        .corrections
        .iter()
        .any(|c| matches!(*c.kind, w::EkrKernelClaimCorrectionKind::V3))
}

fn transaction(
    input: &Inputs<'_>,
    step: &w::EkrIntegrateRetainedApplicationStep,
) -> Result<GraphTransaction<CanonicalValue>, StoreError> {
    if !matches!(
        *input.review.review.decision,
        w::EkrIntegrateReviewDecision::V0
    ) || input.review.review.recorded_at > step.elected_at
        || step.correction_review_id
            != w::EssPresence::Present(input.review.review.review_id.clone())
    {
        return Err(error("correction step has no prior exact approval"));
    }
    let replacements = step
        .replacements
        .iter()
        .map(|r| m::ClaimReplacement {
            previous: g::AssertionId(Uuid(r.previous.0.clone())),
            replacement: g::AssertionId(Uuid(r.replacement.0.clone())),
        })
        .collect::<Vec<_>>();
    let statement_id: EvidenceId = input.review.review.evidence_id.0.parse().map_err(error)?;
    let mut operations = input
        .decision
        .schema_correction_operations(input.read, input.proposal, &replacements, statement_id)
        .map_err(|refusal| error(format!("{}: {}", refusal.code, refusal.reason)))?;
    let at = crate::incubation_document::timestamp_value(&step.elected_at)?;
    for operation in &mut operations {
        if let GraphOperation::AddAssertion(assertion) = operation {
            assertion.transaction_time = TransactionTime::since(at);
        }
    }
    let evidence = operations
        .iter()
        .filter_map(|op| match op {
            GraphOperation::AddAssertion(a) => Some(a.evidence.iter().copied()),
            _ => None,
        })
        .flatten()
        .collect();
    let native = GraphTransaction {
        id: step.transaction.id.0.parse().map_err(error)?,
        proposer: input
            .review
            .review
            .operator
            .actor
            .0
            .parse()
            .map_err(error)?,
        operations,
        evidence,
        schema_version: None,
    };
    let mut encoded = crate::application_transaction::encode(&native.try_into().map_err(error)?)?;
    // The actual signed human statement is canonical already when it supported the schema.
    // A renewed approval may carry a new statement, retained by this same atomic correction.
    let mut statement = encoded.clone();
    statement.operations = vec![Box::new(w::EkrKernelCanonicalOperationProjection::V2(
        w::EkrKernelCanonicalOperationProjectionVariant2 {
            kind: w::EkrKernelCanonicalOperationProjectionVariant2Kind::V0,
            value: input.review.statement.clone(),
        },
    ))];
    let decoded = crate::application_transaction::decode(&statement)?;
    let [GraphOperation::AddEvidence(addition)] = decoded.operations.as_slice() else {
        return Err(error("invalid correction statement"));
    };
    if addition.evidence.id != statement_id {
        return Err(error("correction statement identity differs"));
    }
    if let Some(held) = input.read.graph.evidence.get(&statement_id) {
        if held != &addition.evidence
            || input.read.content(&held.content_hash) != Some(addition.payload.as_slice())
        {
            return Err(error("canonical correction statement differs"));
        }
    } else {
        encoded.operations.extend(statement.operations);
    }
    crate::application_transaction::decode(&encoded)
}

pub(crate) fn build(
    input: &Inputs<'_>,
    at: Timestamp,
) -> Result<w::EkrIntegrateRetainedApplicationStep, StoreError> {
    if input.proposal.proposal.corrections.is_empty() || unresolved(&input.proposal.proposal) {
        return Err(error(
            "schema-correction-unresolved: no executable final correction",
        ));
    }
    let mut step = w::EkrIntegrateRetainedApplicationStep {
        step_election_id: Box::new(ApplicationStepId::mint()),
        application_id: input.election.application_id.clone(),
        correction_review_id: w::EssPresence::Present(input.review.review.review_id.clone()),
        step: Box::new(w::EkrIntegrateApplicationStep {
            kind: Box::new(w::EkrIntegrateApplicationStepKind::V0),
            item: w::EssPresence::Absent,
        }),
        transaction: input.election.schema_transaction.clone(),
        replacements: input
            .proposal
            .proposal
            .corrections
            .iter()
            .filter(|c| matches!(*c.kind, w::EkrKernelClaimCorrectionKind::V1))
            .map(|c| {
                Box::new(w::EkrKernelClaimReplacement {
                    previous: c.assertion_id.clone(),
                    replacement: Box::new(w::EkrGraphAssertionId(AssertionId::mint().to_string())),
                })
            })
            .collect(),
        mappings: vec![],
        derivations: vec![],
        elected_at: crate::schema_application::wire_time(at)?,
    };
    step.transaction.id = Box::new(w::EkrKernelTransactionId(TransactionId::mint().to_string()));
    step.transaction = Box::new(crate::application_transaction::encode(&transaction(
        input, &step,
    )?)?);
    verify_structure(input.proposal, &step)?;
    Ok(step)
}

pub(crate) fn verify_structure(
    proposal: &w::EkrIntegrateRetainedSchemaProposal,
    step: &w::EkrIntegrateRetainedApplicationStep,
) -> Result<(), StoreError> {
    if proposal.proposal.corrections.is_empty()
        || unresolved(&proposal.proposal)
        || !matches!(*step.step.kind, w::EkrIntegrateApplicationStepKind::V0)
        || !matches!(step.step.item, w::EssPresence::Absent)
        || !matches!(step.correction_review_id, w::EssPresence::Present(_))
        || !step.mappings.is_empty()
        || !step.derivations.is_empty()
    {
        return Err(error("unapproved correction step shape"));
    }
    let tx = crate::application_transaction::decode(&step.transaction)?;
    if tx.schema_version.is_some()
        || tx.operations.is_empty()
        || tx.operations.iter().any(|op| {
            !matches!(
                op,
                GraphOperation::RetractAssertion(_)
                    | GraphOperation::AddAssertion(_)
                    | GraphOperation::AddEvidence(_)
            )
        })
    {
        return Err(error("correction step contains unapproved operation kinds"));
    }
    let expected = proposal
        .proposal
        .corrections
        .iter()
        .filter(|c| matches!(*c.kind, w::EkrKernelClaimCorrectionKind::V1))
        .map(|c| c.assertion_id.0.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let previous = step
        .replacements
        .iter()
        .map(|r| r.previous.0.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let replacements = step
        .replacements
        .iter()
        .map(|r| r.replacement.0.clone())
        .collect::<std::collections::BTreeSet<_>>();
    if expected != previous
        || previous.len() != step.replacements.len()
        || replacements.len() != previous.len()
        || !previous.is_disjoint(&replacements)
    {
        return Err(error("correction replacement coverage differs"));
    }
    Ok(())
}

pub(crate) fn verify_semantic(
    input: &Inputs<'_>,
    step: &w::EkrIntegrateRetainedApplicationStep,
) -> Result<(), StoreError> {
    verify_structure(input.proposal, step)?;
    if crate::application_transaction::encode(&transaction(input, step)?)? != *step.transaction {
        return Err(error(
            "schema-review-required: frozen correction effects differ from exact approval",
        ));
    }
    Ok(())
}
