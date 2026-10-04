//! Reports and receipts are derived from verified ordinary commits; elections are not progress.
use crate::{application_material, replay::ReplayState, schema_proposals::error, GraphOperation};
use ekr_core::{
    contract_data as w,
    generated_identity::{ApplicationReceiptId, Identity, ProcessingReceiptId},
    RevisionNumber,
};
use ekr_store::{RetainedHistory, StoreError};

fn processing_record(
    history: &RetainedHistory,
    publication: &w::EkrIntegrateApplicationPublicationRecord,
    id: Box<w::EkrIntegrateProcessingReceiptId>,
) -> Result<w::EkrIntegrateProcessingReceiptSnapshot, StoreError> {
    let w::EssPresence::Present(item) = &publication.guard.step.item else {
        return Err(error("mapping commit has no qualified item"));
    };
    let step = history
        .applications
        .steps()
        .iter()
        .find(|step| step.step_election_id == publication.guard.step_election_id)
        .ok_or_else(|| error("mapping commit step is unavailable"))?;
    let tx = crate::application_transaction::decode(&step.transaction)?;
    let assertions = tx
        .operations
        .iter()
        .filter_map(|op| match op {
            GraphOperation::AddAssertion(claim) => {
                Some(Box::new(w::EkrGraphAssertionId(claim.id.to_string())))
            }
            _ => None,
        })
        .collect();
    Ok(w::EkrIntegrateProcessingReceiptSnapshot {
        receipt_id: id,
        document_digest: item.source.document_digest.clone(),
        item: item.item.clone(),
        mapping_digest: w::EssPresence::Present(item.mapping_digest.clone()),
        transaction_id: w::EssPresence::Present(publication.transaction_id.clone()),
        assertions,
        disposition: Box::new(w::EkrIntegrateProcessingDisposition::V1),
        basis_digest: publication.record_hash.clone(),
    })
}

pub(crate) fn processing(
    history: &RetainedHistory,
    state: &ReplayState,
    election: &w::EkrIntegrateRetainedApplicationElection,
    through: RevisionNumber,
) -> Result<Vec<w::EkrIntegrateProcessingReceiptSnapshot>, StoreError> {
    let prefixes = application_material::prefixes(history, state, through)?;
    let Some(prefix) = prefixes.get(&election.proposal_id.0) else {
        return Ok(vec![]);
    };
    let mut result = vec![];
    for (_, publication) in &prefix.commits {
        if !matches!(
            *publication.guard.step.kind,
            w::EkrIntegrateApplicationStepKind::V1
        ) {
            continue;
        }
        let held = history.applications.processing_receipts().iter().find(|r| {
            r.transaction_id == w::EssPresence::Present(publication.transaction_id.clone())
        });
        let expected = processing_record(
            history,
            publication,
            held.map_or_else(
                || Box::new(ProcessingReceiptId::mint()),
                |r| r.receipt_id.clone(),
            ),
        )?;
        if held.is_some_and(|held| **held != expected) {
            return Err(error(
                "processing receipt differs from its committed mapping",
            ));
        }
        result.push(expected);
    }
    Ok(result)
}

pub(crate) fn snapshot(
    history: &RetainedHistory,
    state: &ReplayState,
    election: &w::EkrIntegrateRetainedApplicationElection,
    through: RevisionNumber,
) -> Result<Option<w::EkrIntegrateApplicationReceiptSnapshot>, StoreError> {
    let prefixes = application_material::prefixes(history, state, through)?;
    let Some(prefix) = prefixes.get(&election.proposal_id.0) else {
        return Ok(None);
    };
    let mut schemas = prefix
        .commits
        .iter()
        .filter(|(_, p)| matches!(*p.guard.step.kind, w::EkrIntegrateApplicationStepKind::V2));
    let Some((number, schema)) = schemas.next() else {
        return Ok(None);
    };
    if schemas.next().is_some() {
        return Err(error("application has more than one schema commit"));
    }
    let proposal = crate::application_inputs::proposal(history, election)?;
    let remaining = prefix.remaining(&proposal.proposal)?;
    let remaining_items = remaining
        .mappings
        .iter()
        .map(|m| crate::application_mapping::item(m).map(Box::new))
        .collect::<Result<Vec<_>, _>>()?;
    let corrections_pending = !remaining.corrections.is_empty();
    let progress = if remaining_items.is_empty() && !corrections_pending {
        w::EkrIntegrateApplicationProgress::V0
    } else if prefix.commits.len() == 1 {
        w::EkrIntegrateApplicationProgress::V4
    } else {
        w::EkrIntegrateApplicationProgress::V2
    };
    let processing = processing(history, state, election, through)?;
    Ok(Some(w::EkrIntegrateApplicationReceiptSnapshot {
        receipt_id: Box::new(ApplicationReceiptId::mint()),
        application_id: election.application_id.clone(),
        proposal_id: election.proposal_id.clone(),
        review_id: schema.guard.review_id.clone(),
        schema_transaction: schema.transaction_id.clone(),
        schema_revision: Box::new(w::EkrKernelRevisionNumber(number.get().into())),
        progress: Box::new(progress),
        processing_receipts: processing.into_iter().map(|r| r.receipt_id).collect(),
        remaining_items,
        corrections_pending,
        stop_reason: w::EssPresence::Absent,
    }))
}

fn receipt_revision(
    history: &RetainedHistory,
    state: &ReplayState,
    election: &w::EkrIntegrateRetainedApplicationElection,
    receipt: &w::EkrIntegrateApplicationReceiptSnapshot,
) -> Result<RevisionNumber, StoreError> {
    let mut through = RevisionNumber::new(
        receipt
            .schema_revision
            .0
            .as_u64()
            .ok_or_else(|| error("receipt schema revision"))?,
    );
    for id in &receipt.processing_receipts {
        let held = history
            .applications
            .processing_receipts()
            .iter()
            .find(|r| r.receipt_id == *id)
            .ok_or_else(|| error("receipt processing link is unavailable"))?;
        let w::EssPresence::Present(tx) = &held.transaction_id else {
            return Err(error("processing receipt has no commit"));
        };
        let committed = state
            .transactions
            .get(&tx.0.parse().map_err(error)?)
            .and_then(|t| t.committed.as_ref())
            .ok_or_else(|| error("processing transaction is uncommitted"))?;
        through = through.max(committed.result.revision);
    }
    let proposal = crate::application_inputs::proposal(history, election)?;
    if !receipt.corrections_pending && !proposal.proposal.corrections.is_empty() {
        let prefixes = application_material::prefixes(history, state, state.head().root.revision)?;
        let correction = prefixes
            .get(&election.proposal_id.0)
            .and_then(|p| {
                p.commits.iter().find(|(_, c)| {
                    matches!(*c.guard.step.kind, w::EkrIntegrateApplicationStepKind::V0)
                })
            })
            .ok_or_else(|| error("receipt corrections have no commit"))?;
        through = through.max(correction.0);
    }
    Ok(through)
}

pub(crate) fn verify_receipt(
    history: &RetainedHistory,
    state: &ReplayState,
    election: &w::EkrIntegrateRetainedApplicationElection,
    receipt: &w::EkrIntegrateApplicationReceiptSnapshot,
) -> Result<(), StoreError> {
    let through = receipt_revision(history, state, election, receipt)?;
    let mut expected = snapshot(history, state, election, through)?
        .ok_or_else(|| error("receipt has no schema commit"))?;
    expected.receipt_id = receipt.receipt_id.clone();
    if let w::EssPresence::Present(reason) = &receipt.stop_reason {
        if reason.trim().is_empty()
            || matches!(*expected.progress, w::EkrIntegrateApplicationProgress::V0)
        {
            return Err(error("receipt stop has no pending work"));
        }
        expected.stop_reason = receipt.stop_reason.clone();
        *expected.progress = w::EkrIntegrateApplicationProgress::V3;
    }
    if expected != *receipt {
        return Err(error("application receipt differs from committed prefix"));
    }
    Ok(())
}

pub(crate) fn verify_processing(
    history: &RetainedHistory,
    state: &ReplayState,
) -> Result<(), StoreError> {
    let prefixes = application_material::prefixes(history, state, state.head().root.revision)?;
    for held in history.applications.processing_receipts() {
        let publication = prefixes
            .values()
            .flat_map(|p| p.commits.iter())
            .map(|(_, p)| p)
            .find(|p| {
                matches!(*p.guard.step.kind, w::EkrIntegrateApplicationStepKind::V1)
                    && held.transaction_id == w::EssPresence::Present(p.transaction_id.clone())
            })
            .ok_or_else(|| error("processing receipt has no verified mapping commit"))?;
        if **held != processing_record(history, publication, held.receipt_id.clone())? {
            return Err(error(
                "processing receipt differs from exact committed mapping",
            ));
        }
    }
    Ok(())
}

pub(crate) fn report(
    history: &RetainedHistory,
    state: &ReplayState,
    election: &w::EkrIntegrateRetainedApplicationElection,
    receipt: &w::EkrIntegrateApplicationReceiptSnapshot,
    already_complete: bool,
) -> Result<w::EkrIntegrateApplicationReport, StoreError> {
    verify_receipt(history, state, election, receipt)?;
    let through = receipt_revision(history, state, election, receipt)?;
    let processed = processing(history, state, election, through)?;
    let mut items = vec![];
    for key in &election.selected_items {
        let held = processed.iter().find(|r| {
            r.document_digest == key.source.document_digest
                && r.item == key.item
                && r.mapping_digest == w::EssPresence::Present(key.mapping_digest.clone())
        });
        items.push(Box::new(w::EkrIntegrateIntegrationItemReceipt {
            source: key.source.clone(),
            item: key.item.clone(),
            mapping_digest: key.mapping_digest.clone(),
            disposition: Box::new(if held.is_some() {
                w::EkrIntegrateProcessingDisposition::V1
            } else {
                w::EkrIntegrateProcessingDisposition::V2
            }),
            transaction_id: held.map_or(w::EssPresence::Absent, |r| r.transaction_id.clone()),
            assertions: held.map_or_else(Vec::new, |r| r.assertions.clone()),
            blockers: vec![],
            reason: if held.is_some() {
                w::EssPresence::Absent
            } else {
                receipt.stop_reason.clone()
            },
        }));
    }
    Ok(w::EkrIntegrateApplicationReport {
        application_id: election.application_id.clone(),
        already_complete,
        schema_transaction: receipt.schema_transaction.clone(),
        schema_revision: w::EssPresence::Present(receipt.schema_revision.clone()),
        receipt_id: w::EssPresence::Present(receipt.receipt_id.clone()),
        progress: receipt.progress.clone(),
        remaining_items: receipt.remaining_items.clone(),
        corrections_pending: receipt.corrections_pending,
        items,
        stop_reason: receipt.stop_reason.clone(),
    })
}

#[cfg(test)]
mod tests;
