//! Reconstruct schema support from immutable selected sources; no publication authority here.
use crate::{schema_proposals as proposals, EvidenceAddition, VerifiedRead};
use ekr_core::{contract_data as w, AgentId, EvidenceId, ObservationId};
use ekr_graph::{Confidence, Evidence, EvidenceSource};
use ekr_store::StoreError;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) struct SupportPlan {
    pub manifest: BTreeSet<EvidenceId>,
    pub additions: Vec<EvidenceAddition>,
    pub source_evidence: BTreeMap<EvidenceId, EvidenceId>,
}

pub(crate) fn check_current(
    read: &VerifiedRead,
    proposal: &w::EkrIntegrateSchemaProposalDocument,
    support: &proposals::SourceSupport,
) -> Result<(), StoreError> {
    let mut selected = proposal
        .evidence
        .iter()
        .map(|id| id.0.parse().map_err(proposals::error))
        .collect::<Result<BTreeSet<EvidenceId>, _>>()?;
    for source in support.sources.values() {
        for item in &source.selected {
            let fact = source
                .document
                .facts
                .get(proposals::fact_index(item)?)
                .ok_or_else(|| proposals::error("selected support fact is unavailable"))?;
            selected.extend(fact.evidence());
        }
    }
    for source in support.sources.values() {
        for entry in &source.document.evidence {
            if selected.contains(&entry.evidence.id)
                && read
                    .graph
                    .evidence
                    .get(&entry.evidence.id)
                    .is_some_and(|canonical| *canonical != entry.evidence)
            {
                return Err(proposals::error(
                    "canonical and source evidence identity disagree",
                ));
            }
        }
    }
    Ok(())
}

/// Allocation is injected so replay reconstructs the same plan with the already elected IDs.
pub(crate) fn plan(
    read: &VerifiedRead,
    proposal: &w::EkrIntegrateSchemaProposalDocument,
    support: &proposals::SourceSupport,
    observations: &BTreeMap<ObservationId, w::EkrObserveRetainedObservationRead>,
    actor: AgentId,
    mut allocate: impl FnMut() -> Result<EvidenceId, StoreError>,
) -> Result<SupportPlan, StoreError> {
    let mut selected = proposal
        .evidence
        .iter()
        .map(|id| id.0.parse().map_err(proposals::error))
        .collect::<Result<BTreeSet<EvidenceId>, _>>()?;
    let mut source_records = BTreeMap::new();
    for source in support.sources.values() {
        for item in &source.selected {
            let fact = source
                .document
                .facts
                .get(proposals::fact_index(item)?)
                .ok_or_else(|| proposals::error("selected support fact is unavailable"))?;
            selected.extend(fact.evidence());
        }
        for entry in &source.document.evidence {
            if source_records
                .insert(entry.evidence.id, entry.clone())
                .is_some_and(|old| old != *entry)
            {
                return Err(proposals::error("competing source evidence records"));
            }
        }
    }
    let mut used: BTreeSet<_> = read.graph.evidence.keys().copied().collect();
    used.extend(source_records.keys().copied());
    let mut fresh = || {
        let id = allocate()?;
        if !used.insert(id) {
            return Err(proposals::error(
                "application wrapper identity is not fresh",
            ));
        }
        Ok(id)
    };
    let mut plan = SupportPlan {
        manifest: BTreeSet::new(),
        additions: Vec::new(),
        source_evidence: BTreeMap::new(),
    };
    for id in selected {
        if let Some(canonical) = read.graph.evidence.get(&id) {
            if source_records
                .get(&id)
                .is_some_and(|entry| entry.evidence != *canonical)
            {
                return Err(proposals::error(
                    "canonical and source evidence identity disagree",
                ));
            }
            plan.manifest.insert(id);
            plan.source_evidence.insert(id, id);
        } else {
            let entry = source_records
                .get(&id)
                .ok_or_else(|| proposals::error("selected supporting evidence is unavailable"))?;
            let mut wrapper = entry.evidence.clone();
            wrapper.id = fresh()?;
            wrapper.extracted_by = actor;
            plan.manifest.insert(wrapper.id);
            plan.source_evidence.insert(id, wrapper.id);
            plan.additions.push(EvidenceAddition {
                evidence: wrapper,
                payload: entry.payload.clone(),
            });
        }
    }
    let selected_observations = proposal
        .observations
        .iter()
        .map(|id| id.0.parse().map_err(proposals::error))
        .collect::<Result<BTreeSet<ObservationId>, _>>()?;
    for id in selected_observations {
        let held = observations
            .get(&id)
            .ok_or_else(|| proposals::error("selected observation is unavailable"))?;
        let wrapper = Evidence {
            id: fresh()?,
            source: EvidenceSource::Observation(id),
            content_hash: held
                .observation
                .content_hash
                .0
                .parse()
                .map_err(proposals::error)?,
            extracted_by: actor,
            observed_at: crate::incubation_document::timestamp_value(
                &held.observation.captured_at,
            )?,
            confidence: Confidence::from_basis_points(0).expect("zero is a confidence"),
        };
        plan.manifest.insert(wrapper.id);
        plan.additions.push(EvidenceAddition {
            evidence: wrapper,
            payload: ekr_core::bytes::decode(&held.payload).map_err(proposals::error)?,
        });
    }
    Ok(plan)
}
