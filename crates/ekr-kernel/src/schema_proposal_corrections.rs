//! Applicability of selected claim corrections without allocating future canonical records.
use crate::{human_review as h, schema_proposals::error, VerifiedRead};
use ekr_core::{contract_data as w, AssertionId};
use ekr_store::StoreError;
use std::collections::BTreeSet;

/// Capture every affected dispute, including competitors not explicitly selected by a correction.
/// Callers hash its material digests, excluding the read revision and presentation text.
pub(super) fn components(
    read: &VerifiedRead,
    proposal: &w::EkrIntegrateSchemaProposalDocument,
) -> Result<Vec<w::EkrKernelAttentionItem>, StoreError> {
    if proposal.corrections.is_empty() {
        return Ok(Vec::new());
    }
    let selected: BTreeSet<_> = proposal
        .corrections
        .iter()
        .map(|c| &c.assertion_id.0)
        .collect();
    Ok(read
        .dispute_attention()?
        .into_iter()
        .filter(|item| item.claims.iter().any(|id| selected.contains(&id.0)))
        .collect())
}

pub(super) fn validate(
    read: &VerifiedRead,
    proposal: &w::EkrIntegrateSchemaProposalDocument,
) -> Result<(), StoreError> {
    if proposal.corrections.is_empty() {
        return Ok(());
    }
    let corrections = proposal
        .corrections
        .iter()
        .map(|c| h::correction_from_document(c).map_err(|e| error(e.reason)))
        .collect::<Result<Vec<_>, _>>()?;
    let mut assigned = BTreeSet::new();
    for item in components(read, proposal)? {
        let claims = item
            .claims
            .iter()
            .map(|id| id.0.parse::<AssertionId>().map_err(error))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let selected = corrections
            .iter()
            .filter(|c| {
                claims
                    .iter()
                    .any(|id| id.to_string() == c.assertion_id.0 .0)
            })
            .cloned()
            .collect::<Vec<_>>();
        if selected.is_empty() {
            continue;
        }
        h::validate_corrections(&read.graph, &claims, &selected).map_err(|e| error(e.reason))?;
        assigned.extend(selected.iter().map(|c| c.assertion_id.0 .0.clone()));
    }
    if assigned.len() != corrections.len() {
        return Err(error(
            "corrections must name distinct active claims in current disputes",
        ));
    }
    Ok(())
}
