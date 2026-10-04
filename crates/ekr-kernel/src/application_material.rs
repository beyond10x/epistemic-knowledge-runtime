//! Review material tracks only commits already admitted by this replay, never receipts or matches.
use crate::{replay::ReplayState, schema_proposals::error};
use ekr_core::{contract_data as w, RevisionNumber};
use ekr_store::{RetainedHistory, StoreError};
use std::collections::BTreeMap;

/// Internal material for hashing, not another persisted application model.
#[derive(Clone, Debug, serde::Serialize)]
pub(crate) struct Prefix {
    pub application: Box<w::EkrIntegrateSchemaApplicationId>,
    pub proposal_digest: Box<w::EkrKernelContentHash>,
    pub commits: Vec<(RevisionNumber, w::EkrIntegrateApplicationPublicationRecord)>,
}
impl Prefix {
    pub(crate) fn schema_done(&self) -> bool {
        self.commits
            .iter()
            .any(|(_, p)| matches!(*p.guard.step.kind, w::EkrIntegrateApplicationStepKind::V2))
    }
    pub(crate) fn corrections_done(&self) -> bool {
        self.commits
            .iter()
            .any(|(_, p)| matches!(*p.guard.step.kind, w::EkrIntegrateApplicationStepKind::V0))
    }
    pub(crate) fn remaining(
        &self,
        original: &w::EkrIntegrateSchemaProposalDocument,
    ) -> Result<w::EkrIntegrateSchemaProposalDocument, StoreError> {
        let mut pending = original.clone();
        if self.schema_done() {
            pending.additions.clear();
        }
        pending.mappings.clear();
        for mapping in &original.mappings {
            let key = crate::application_mapping::item(mapping)?;
            if !self.commits.iter().any(|(_, p)| matches!(&p.guard.step.item, w::EssPresence::Present(item) if **item == key)) {
                pending.mappings.push(mapping.clone());
            }
        }
        if self.corrections_done() {
            pending.corrections.clear();
        }
        Ok(pending)
    }
}

pub(crate) fn prefixes(
    history: &RetainedHistory,
    state: &ReplayState,
    number: RevisionNumber,
) -> Result<BTreeMap<String, Prefix>, StoreError> {
    let mut result: BTreeMap<String, Prefix> = BTreeMap::new();
    for tx in state.transactions.values() {
        let Some(committed) = &tx.committed else {
            continue;
        };
        if committed.result.revision > number {
            continue;
        }
        let occurrence = history
            .occurrences
            .iter()
            .find(|o| o.event.event_id == committed.event_id)
            .ok_or_else(|| error("verified application commit occurrence is unavailable"))?;
        let Some(guard) = occurrence.event.application.as_ref() else {
            continue;
        };
        let guard = guard.as_data();
        let prefix = result
            .entry(guard.proposal_id.0.clone())
            .or_insert_with(|| Prefix {
                application: guard.application_id.clone(),
                proposal_digest: guard.proposal_digest.clone(),
                commits: vec![],
            });
        if prefix.application != guard.application_id
            || prefix.proposal_digest != guard.proposal_digest
        {
            return Err(error("verified application prefix changes election"));
        }
        prefix.commits.push((
            committed.result.revision,
            w::EkrIntegrateApplicationPublicationRecord {
                guard: Box::new(guard.clone()),
                event_id: Box::new(w::EkrKernelEventId(occurrence.event.event_id.to_string())),
                record_hash: Box::new(w::EkrKernelContentHash(
                    occurrence.event.record_hash.to_string(),
                )),
                transaction_id: guard.attempt_transaction.clone(),
                command: Box::new(w::EkrStorePublicationCommandKind::V2),
            },
        ));
    }
    for prefix in result.values_mut() {
        prefix.commits.sort_by_key(|(revision, _)| *revision);
    }
    Ok(result)
}
