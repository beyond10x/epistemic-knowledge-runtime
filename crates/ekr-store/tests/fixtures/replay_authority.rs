//! Test-only authority for the private provider-counting unit fixture. Production authorities
//! remain owned exclusively by ekr-kernel; this module is included only by eventlog_reads tests.
use std::collections::BTreeSet;

use ekr_core::{ContentHash, RevisionNumber};
use ekr_ontology::Ontology;

use crate::{AdmittedRevision, CommitAuthority, RetainedHistory, StoreError};

pub(crate) struct NeedsEvidence(pub(crate) ContentHash);

impl CommitAuthority for NeedsEvidence {
    fn required_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::from([self.0]))
    }

    fn replay_objects(&self, _: &RetainedHistory) -> Result<BTreeSet<ContentHash>, StoreError> {
        Ok(BTreeSet::new())
    }

    fn replay(
        &self,
        _: &RetainedHistory,
        _: Option<&Ontology>,
        _: Option<RevisionNumber>,
    ) -> Result<Option<AdmittedRevision>, StoreError> {
        Ok(None)
    }
}
