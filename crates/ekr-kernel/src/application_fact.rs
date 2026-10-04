//! Pure frozen mapping-step construction and checks; publication authority remains with replay.
use crate::{
    application_mapping as mapping, schema_proposals::Source, GraphOperation, GraphTransaction,
    VerifiedRead,
};
use ekr_core::{
    contract_data as w,
    generated_identity::{ApplicationStepId, Identity, MappingRecordId, SchemaApplicationId},
    AgentId, AssertionId, ContentHash, EvidenceId, NodeId, Timestamp, TransactionId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalRef, EvidenceSource, Object, Predicate,
    Subject, TemporalRange, TransactionTime,
};
use ekr_store::StoreError;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct BuildInputs<'a> {
    pub read: &'a VerifiedRead,
    pub proposal: &'a w::EkrIntegrateRetainedSchemaProposal,
    pub mapping: &'a w::EkrIntegrateKnowledgeMapping,
    pub sources: &'a BTreeMap<String, Source>,
    pub evidence_map: &'a BTreeMap<EvidenceId, EvidenceId>,
    pub application_id: &'a w::EkrIntegrateSchemaApplicationId,
    pub proposer: AgentId,
    pub elected_at: Timestamp,
}
#[derive(Debug, PartialEq)]
pub(crate) enum FactStepOutcome {
    Ready(Box<w::EkrIntegrateRetainedApplicationStep>),
    Blocked(Vec<String>),
}
#[derive(Clone)]
struct Identities {
    step: w::EkrIntegrateApplicationStepId,
    transaction: TransactionId,
    assertion: AssertionId,
    mapping: w::EkrIntegrateMappingRecordId,
    derivations: Vec<String>,
}
impl Identities {
    fn mint(supports: usize) -> Self {
        Self {
            step: ApplicationStepId::mint(),
            transaction: TransactionId::mint(),
            assertion: AssertionId::mint(),
            mapping: MappingRecordId::mint(),
            // This generated field is the UUID scalar, not a named identity wrapper.
            derivations: (0..supports).map(|_| NodeId::mint().to_string()).collect(),
        }
    }
    fn check(&self, supports: usize) -> Result<(), StoreError> {
        ApplicationStepId::parse_identity(&self.step.0).map_err(error)?;
        MappingRecordId::parse_identity(&self.mapping.0).map_err(error)?;
        if self.derivations.len() != supports {
            return Err(error("derivation count differs from selected support"));
        }
        let mut seen: BTreeSet<String> = [
            self.step.0.clone(),
            self.transaction.to_string(),
            self.assertion.to_string(),
            self.mapping.0.clone(),
        ]
        .into();
        if seen.len() != 4 {
            return Err(error("step allocation identities collide"));
        }
        for id in &self.derivations {
            let _: NodeId = id.parse().map_err(error)?;
            if !seen.insert(id.clone()) {
                return Err(error("duplicate derivation identity"));
            }
        }
        Ok(())
    }
}
fn error(reason: impl std::fmt::Display) -> StoreError {
    StoreError::Document(format!("application-fact: {reason}"))
}

/// No identity is minted until mapping resolution and selected canonical support succeed.
pub(crate) fn build(input: &BuildInputs<'_>) -> Result<FactStepOutcome, StoreError> {
    build_using(input, Identities::mint)
}
fn build_using(
    input: &BuildInputs<'_>,
    allocate: impl FnOnce(usize) -> Identities,
) -> Result<FactStepOutcome, StoreError> {
    let originals = selected_evidence(input)?;
    let claim = match mapping::resolve(
        input.read,
        input.mapping,
        &input.read.graph.ontology,
        input.sources,
    )? {
        mapping::MappingOutcome::Ready(claim) => claim,
        mapping::MappingOutcome::Blocked(reasons) => return Ok(FactStepOutcome::Blocked(reasons)),
    };
    if claim.evidence != originals {
        return Err(error("resolved source support changed"));
    }
    let supports = support(input, &originals)?;
    wire_time(input.elected_at)?;
    let identities = allocate(supports.len());
    render(input, claim, &supports, &identities).map(|step| FactStepOutcome::Ready(Box::new(step)))
}

/// Checks immutable membership without resolving aliases or locating a revision by timestamp.
fn selected_evidence(input: &BuildInputs<'_>) -> Result<Vec<EvidenceId>, StoreError> {
    SchemaApplicationId::parse_identity(&input.application_id.0).map_err(error)?;
    let proposal_bytes = ekr_core::bytes::decode(&input.proposal.payload).map_err(error)?;
    let proposal_hash: ContentHash = input.proposal.proposal_digest.0.parse().map_err(error)?;
    let proposal: w::EkrIntegrateSchemaProposalDocument =
        serde_json::from_slice(&proposal_bytes).map_err(error)?;
    if ContentHash::of_bytes(&proposal_bytes) != proposal_hash
        || proposal != *input.proposal.proposal
    {
        return Err(error("retained proposal bytes disagree"));
    }
    if proposal
        .mappings
        .iter()
        .filter(|value| value.as_ref() == input.mapping)
        .count()
        != 1
    {
        return Err(error("mapping is not selected exactly once"));
    }
    if !proposal.sources.iter().any(|source| {
        source.version == input.mapping.source && source.items.contains(&input.mapping.source_item)
    }) {
        return Err(error("mapping source item is outside proposal selection"));
    }
    let source = input
        .sources
        .get(&crate::schema_proposals::coordinate(&input.mapping.source)?)
        .ok_or_else(|| error("mapping source is unavailable"))?;
    if !source.selected.contains(&input.mapping.source_item) {
        return Err(error("mapping source item is unavailable"));
    }
    let fact = source
        .document
        .facts
        .get(crate::schema_proposals::fact_index(
            &input.mapping.source_item,
        )?)
        .ok_or_else(|| error("mapping fact is unavailable"))?;
    if fact.evidence().is_empty() {
        return Err(error("selected fact has no supporting evidence"));
    }
    Ok(fact.evidence().to_vec())
}
fn support(
    input: &BuildInputs<'_>,
    originals: &[EvidenceId],
) -> Result<BTreeSet<EvidenceId>, StoreError> {
    originals
        .iter()
        .map(|original| {
            let mapped = *input
                .evidence_map
                .get(original)
                .ok_or_else(|| error("source evidence has no frozen schema correspondence"))?;
            let evidence = input
                .read
                .graph
                .evidence
                .get(&mapped)
                .ok_or_else(|| error("mapped evidence is not canonical"))?;
            if evidence.id != mapped {
                return Err(error("mapped evidence identity disagrees"));
            }
            Ok(mapped)
        })
        .collect()
}
fn wire_time(time: Timestamp) -> Result<w::EssTimestamp, StoreError> {
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(time.millis()) * 1_000_000)
        .map(w::EssTimestamp)
        .map_err(error)
}
fn render(
    input: &BuildInputs<'_>,
    claim: mapping::MappedClaim,
    supports: &BTreeSet<EvidenceId>,
    identities: &Identities,
) -> Result<w::EkrIntegrateRetainedApplicationStep, StoreError> {
    identities.check(supports.len())?;
    if !matches!(claim.subject, Subject::Node(_))
        || !matches!(
            (&claim.predicate, &claim.object),
            (Predicate::Property(_), Object::Value(_)) | (Predicate::Relation(_), Object::Node(_))
        )
    {
        return Err(error("mapping claim has an unsupported shape"));
    }
    let assertion = Assertion {
        id: identities.assertion,
        root_id: input.read.graph.root.id,
        subject: claim.subject,
        predicate: claim.predicate,
        object: claim.object,
        evidence: supports.iter().copied().map(CanonicalRef::new).collect(),
        proposed_by: input.proposer,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(input.elected_at),
    };
    let transaction = GraphTransaction {
        id: identities.transaction,
        proposer: input.proposer,
        operations: vec![GraphOperation::AddAssertion(Box::new(assertion))],
        evidence: supports.clone(),
        schema_version: None,
    };
    let mapping_bytes = mapping::payload(input.mapping)?;
    let mapping_record = w::EkrIntegrateRetainedMappingRecord {
        mapping_id: Box::new(identities.mapping.clone()),
        proposal_digest: input.proposal.proposal_digest.clone(),
        mapping_digest: Box::new(w::EkrKernelContentHash(
            ContentHash::of_bytes(&mapping_bytes).to_string(),
        )),
        source_document_digest: input.mapping.source.document_digest.clone(),
        mapping: Box::new(input.mapping.clone()),
        evidence: supports
            .iter()
            .map(|id| Box::new(w::EkrGraphEvidenceId(id.to_string())))
            .collect(),
        payload: ekr_core::bytes::encode(&mapping_bytes),
    };
    let derivations = supports
        .iter()
        .zip(&identities.derivations)
        .map(|(evidence, id)| {
            let source = &input
                .read
                .graph
                .evidence
                .get(evidence)
                .ok_or_else(|| error("mapped evidence is unavailable"))?
                .source;
            Ok(Box::new(w::EkrIntegrateCanonicalDerivationRecord {
                derivation_id: id.clone(),
                assertion_id: Box::new(w::EkrGraphAssertionId(identities.assertion.to_string())),
                mapping_id: Box::new(identities.mapping.clone()),
                evidence_id: Box::new(w::EkrGraphEvidenceId(evidence.to_string())),
                observation_id: match source {
                    EvidenceSource::Observation(id) => {
                        w::EssPresence::Present(Box::new(w::EkrGraphObservationId(id.to_string())))
                    }
                    _ => w::EssPresence::Absent,
                },
            }))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    Ok(w::EkrIntegrateRetainedApplicationStep {
        step_election_id: Box::new(identities.step.clone()),
        application_id: Box::new(input.application_id.clone()),
        step: Box::new(w::EkrIntegrateApplicationStep {
            kind: Box::new(w::EkrIntegrateApplicationStepKind::V1),
            item: w::EssPresence::Present(Box::new(mapping::item(input.mapping)?)),
        }),
        transaction: Box::new(crate::application_transaction::encode(&transaction)?),
        replacements: vec![],
        mappings: vec![Box::new(mapping_record)],
        derivations,
        elected_at: wire_time(input.elected_at)?,
    })
}

fn frozen(
    step: &w::EkrIntegrateRetainedApplicationStep,
) -> Result<(Identities, Assertion), StoreError> {
    let transaction = crate::application_transaction::decode(&step.transaction)?;
    let [GraphOperation::AddAssertion(assertion)] = transaction.operations.as_slice() else {
        return Err(error("mapping step must contain exactly one AddAssertion"));
    };
    let [mapping] = step.mappings.as_slice() else {
        return Err(error(
            "mapping step must contain exactly one mapping record",
        ));
    };
    let identities = Identities {
        step: step.step_election_id.as_ref().clone(),
        transaction: transaction.id,
        assertion: assertion.id,
        mapping: mapping.mapping_id.as_ref().clone(),
        derivations: step
            .derivations
            .iter()
            .map(|row| row.derivation_id.clone())
            .collect(),
    };
    Ok((identities, assertion.as_ref().clone()))
}
/// Check exact retained shape and support without resolving the current aliases. This does not
/// prove the subject/predicate/object matched the source at the first guarded Propose.
pub(crate) fn verify_structure(
    input: &BuildInputs<'_>,
    step: &w::EkrIntegrateRetainedApplicationStep,
) -> Result<(), StoreError> {
    let evidence = selected_evidence(input)?;
    let supports = support(input, &evidence)?;
    let (identities, assertion) = frozen(step)?;
    let claim = mapping::MappedClaim {
        subject: assertion.subject,
        predicate: assertion.predicate,
        object: assertion.object,
        evidence,
    };
    let expected = render(input, claim, &supports, &identities)?;
    if expected != *step {
        return Err(error("frozen mapping step fields disagree"));
    }
    Ok(())
}
/// Only the first guarded Propose supplies the semantic read. A later canonical revision, or a
/// timestamp-derived guess at one, must not replace that verified historical boundary.
pub(crate) fn verify_semantic(
    input: &BuildInputs<'_>,
    step: &w::EkrIntegrateRetainedApplicationStep,
) -> Result<(), StoreError> {
    verify_structure(input, step)?;
    let (identities, _) = frozen(step)?;
    match build_using(input, |_| identities)? {
        FactStepOutcome::Ready(expected) if *expected == *step => Ok(()),
        _ => Err(error(
            "frozen mapping step differs from first-proposal semantic resolution",
        )),
    }
}
#[cfg(test)]
mod tests;
