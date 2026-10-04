//! Authenticate immutable application sources from one retained history, never a live root.
use crate::{schema_proposals as proposals, VerifiedRead};
use ekr_core::{contract_data as w, ContentHash, ObservationId, Timestamp};
use ekr_store::{RetainedHistory, StorageClass, StoreError};
use std::collections::BTreeMap;

pub(crate) fn proposal(
    history: &RetainedHistory,
    election: &w::EkrIntegrateRetainedApplicationElection,
) -> Result<w::EkrIntegrateRetainedSchemaProposal, StoreError> {
    let hash: ContentHash = election
        .proposal_digest
        .0
        .parse()
        .map_err(proposals::error)?;
    let bytes = history.content(hash, StorageClass::Provenance)?;
    let proposal: w::EkrIntegrateSchemaProposalDocument =
        serde_json::from_slice(bytes).map_err(proposals::error)?;
    if proposal.proposal_id != election.proposal_id || proposal.base_schema != election.base_schema
    {
        return Err(proposals::error(
            "application proposal coordinates disagree",
        ));
    }
    Ok(w::EkrIntegrateRetainedSchemaProposal {
        proposal: Box::new(proposal),
        proposal_digest: election.proposal_digest.clone(),
        payload: ekr_core::bytes::encode(bytes),
    })
}

pub(crate) fn observed(
    observations: &[w::EkrObserveObservationImport],
) -> Result<BTreeMap<ObservationId, w::EkrObserveRetainedObservationRead>, StoreError> {
    let mut result = BTreeMap::new();
    for input in observations {
        let id = crate::observations::validate(input)?;
        let read = w::EkrObserveRetainedObservationRead {
            observation: input.observation.clone(),
            key: input.key.clone(),
            payload: input.payload.clone(),
        };
        if result.insert(id, read).is_some() {
            return Err(proposals::error("duplicate captured observation"));
        }
    }
    Ok(result)
}

pub(crate) fn interpretation(
    history: &RetainedHistory,
    observations: &BTreeMap<ObservationId, w::EkrObserveRetainedObservationRead>,
    version: &w::EkrIntegrateInterpretationVersion,
) -> Result<w::EkrIntegrateInterpretationDocument, StoreError> {
    let hash = version
        .document_digest
        .0
        .parse()
        .map_err(proposals::error)?;
    let bytes = history.content(hash, StorageClass::Provenance)?;
    let document: w::EkrIntegrateInterpretationDocument =
        serde_json::from_slice(bytes).map_err(proposals::error)?;
    if document.version.interpretation_id != version.interpretation_id
        || document.version.version != version.version
    {
        return Err(proposals::error(
            "captured interpretation coordinates disagree",
        ));
    }
    crate::incubation::checked_document(&document, Timestamp::EPOCH, |id| {
        observation(observations, id).cloned()
    })?;
    Ok(document)
}

fn observation(
    observations: &BTreeMap<ObservationId, w::EkrObserveRetainedObservationRead>,
    id: ObservationId,
) -> Result<&w::EkrObserveRetainedObservationRead, StoreError> {
    observations
        .get(&id)
        .ok_or_else(|| proposals::error("missing captured observation"))
}

pub(crate) fn support(
    history: &RetainedHistory,
    observations: &BTreeMap<ObservationId, w::EkrObserveRetainedObservationRead>,
    proposal: &w::EkrIntegrateSchemaProposalDocument,
    read: &VerifiedRead,
) -> Result<proposals::SourceSupport, StoreError> {
    proposals::source_support(
        proposal,
        read,
        |version| interpretation(history, observations, version),
        |id| observation(observations, id.0.parse().map_err(proposals::error)?).map(|_| ()),
    )
}

pub(crate) fn project(
    history: &RetainedHistory,
    observations: &BTreeMap<ObservationId, w::EkrObserveRetainedObservationRead>,
    record: &w::EkrIntegrateRetainedSchemaProposal,
    read: &VerifiedRead,
    base: &ekr_ontology::Ontology,
) -> Result<w::EkrIntegrateSchemaProposalRead, StoreError> {
    let support = support(history, observations, &record.proposal, read)?;
    let observed = record
        .proposal
        .observations
        .iter()
        .map(|id| observation(observations, id.0.parse().map_err(proposals::error)?).cloned())
        .collect::<Result<Vec<_>, StoreError>>()?;
    proposals::project_captured(record, false, read, base, support, &observed)
}

/// Capture only correction commits already verified by replay at the requested revision.
pub(crate) fn correction_explanations(
    history: &RetainedHistory,
    prefixes: &BTreeMap<String, crate::application_material::Prefix>,
) -> Result<Vec<w::EkrKernelExplainedSchemaCorrection>, StoreError> {
    let mut result = Vec::new();
    for prefix in prefixes.values() {
        for (_, publication) in &prefix.commits {
            if !matches!(
                *publication.guard.step.kind,
                w::EkrIntegrateApplicationStepKind::V0
            ) {
                continue;
            }
            let guard = &publication.guard;
            let election = history
                .applications
                .elections()
                .iter()
                .find(|e| e.application_id == guard.application_id)
                .ok_or_else(|| proposals::error("correction explanation election missing"))?;
            let step = history
                .applications
                .steps()
                .iter()
                .find(|s| s.step_election_id == guard.step_election_id)
                .ok_or_else(|| proposals::error("correction explanation step missing"))?;
            let review = history
                .applications
                .coordination()
                .iter()
                .find(|c| c.proposal_id == guard.proposal_id)
                .and_then(|c| {
                    c.entries.iter().find_map(|entry| match &*entry.record {
                        w::EkrIntegrateProposalCoordinationRecord::V1(row)
                            if row.value.review.review_id == guard.review_id =>
                        {
                            Some(row.value.clone())
                        }
                        _ => None,
                    })
                })
                .ok_or_else(|| proposals::error("correction explanation review missing"))?;
            let original_review = history
                .applications
                .coordination()
                .iter()
                .find(|c| c.proposal_id == guard.proposal_id)
                .and_then(|c| {
                    c.entries.iter().find_map(|entry| match &*entry.record {
                        w::EkrIntegrateProposalCoordinationRecord::V1(row)
                            if step.correction_review_id
                                == w::EssPresence::Present(row.value.review.review_id.clone()) =>
                        {
                            Some(row.value.clone())
                        }
                        _ => None,
                    })
                })
                .ok_or_else(|| {
                    proposals::error("correction explanation originating review missing")
                })?;
            result.push(w::EkrKernelExplainedSchemaCorrection {
                proposal: Box::new(proposal(history, election)?),
                review,
                original_review,
                step: step.clone(),
                publication: Box::new(publication.clone()),
            });
        }
    }
    Ok(result)
}
