//! Project immutable schema manifests through the generated ESS read contract.

use ekr_core::contract_data::{
    EkrGraphEvidenceId, EkrKernelRevisionNumber, EkrKernelTransactionId, EkrOntologySchemaVersionId,
};
use ekr_kernel::SchemaHistory;

use crate::{ProjectError, SchemaEvidenceEntry};

/// Cited evidence for each schema transaction, ordered by revision and evidence identity.
/// Seed evidence and empty manifests are omitted. The caller supplies one verified kernel read.
///
/// # Errors
/// Returns [`ProjectError::Inconsistent`] if a manifest lacks its schema or transaction.
pub fn schema_evidence(history: &SchemaHistory) -> Result<Vec<SchemaEvidenceEntry>, ProjectError> {
    history
        .supporting_evidence
        .iter()
        .filter(|(revision, evidence)| **revision <= history.graph.revision && !evidence.is_empty())
        .map(|(revision, evidence)| {
            let schema = history.schemas.get(revision).ok_or_else(|| {
                ProjectError::Inconsistent(format!(
                    "schema evidence at revision {revision} has no schema"
                ))
            })?;
            let transaction = history.transactions.get(revision).ok_or_else(|| {
                ProjectError::Inconsistent(format!(
                    "schema evidence at revision {revision} has no transaction"
                ))
            })?;
            Ok(SchemaEvidenceEntry {
                revision: Box::new(EkrKernelRevisionNumber(revision.get().into())),
                schema_version: Box::new(EkrOntologySchemaVersionId(
                    schema.version().id.to_string(),
                )),
                transaction_id: Box::new(EkrKernelTransactionId(transaction.to_string())),
                evidence: evidence
                    .iter()
                    .map(|id| Box::new(EkrGraphEvidenceId(id.to_string())))
                    .collect(),
            })
        })
        .collect()
}
