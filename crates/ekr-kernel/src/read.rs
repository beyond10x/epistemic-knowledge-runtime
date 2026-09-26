//! Kernel-owned immutable read captures; projection consumers receive no storage authority.
use crate::{
    AuthorityStateV1, BootstrapContext, Commit, CommitError, SeedDocument, SeedResultV1,
    TransactionRecord,
};
use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{CanonicalGraph, Root};
use ekr_store::{ObjectStore, RevisionLog};
use std::collections::BTreeMap;
/// Complete verified coordinates of one retained canonical revision.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedRevision {
    /// Domain revision identity.
    pub revision_id: RevisionId,
    /// Immutable occurrence identity.
    pub event_id: EventId,
    /// Payload address of the actual seed result or commit receipt.
    pub record_hash: ContentHash,
    /// Trusted original publication time.
    pub committed_at: Timestamp,
    /// Recomputed complete root.
    pub root: Root,
}
/// One verified read boundary, with enough retained input for explanation without another read.
/// Only the kernel constructs it; owning the capture grants no persistence authority.
pub struct VerifiedRead {
    /// Admitted graph at the chosen boundary.
    pub graph: CanonicalGraph,
    /// Recomputed root for that graph.
    pub root: Root,
    /// Actual original seed result, even after later head advancement.
    pub seed: SeedResultV1,
    /// Original admitted seed input, including complete ontology and evidence declarations.
    pub seed_input: SeedDocument,
    /// Actual retained bootstrap identities checked against the host at this same boundary.
    pub context: BootstrapContext,
    /// Complete original registry and validation profile, verified against the host anchor.
    pub authority: AuthorityStateV1,
    /// All canonical revision coordinates through this boundary.
    pub revisions: BTreeMap<RevisionNumber, VerifiedRevision>,
    /// Actual retained transaction decisions through this boundary.
    pub transactions: BTreeMap<TransactionId, TransactionRecord>,
    objects: BTreeMap<ContentHash, Vec<u8>>,
}
impl VerifiedRead {
    /// Already verified retained bytes, with no provider access or new history observation.
    #[must_use]
    pub fn content(&self, hash: &ContentHash) -> Option<&[u8]> {
        self.objects.get(hash).map(Vec::as_slice)
    }
}
impl<S: RevisionLog + ObjectStore> Commit<S> {
    /// Captures one verified current or historical graph together with its actual retained input.
    /// # Errors
    /// Missing seed/revision or any required history/object verification failure.
    pub fn read(&self, revision: Option<RevisionNumber>) -> Result<VerifiedRead, CommitError> {
        let history = match revision {
            Some(revision) => self
                .store
                .history_at(revision)
                .map_err(|error| match error {
                    ekr_store::StoreError::NoMaterialisedState { requested } => {
                        CommitError::RevisionNotFound { against: requested }
                    }
                    other => other.into(),
                })?,
            None => self.store.history()?,
        };
        let state = self
            .authority
            .reconstruct(&history, None, revision)?
            .ok_or(CommitError::NotSeeded)?;
        let envelope = crate::seed::envelope(
            history.content(state.seed.seed_hash, ekr_store::StorageClass::Canonical)?,
        )?;
        let head = state.head();
        let graph = head.graph()?.clone();
        let root = head.root;
        Ok(VerifiedRead {
            graph,
            root,
            seed: state.seed,
            seed_input: envelope.input,
            context: envelope.context,
            authority: envelope.authority,
            transactions: state.transactions,
            revisions: state
                .revisions
                .into_iter()
                .map(|(number, state)| {
                    (
                        number,
                        VerifiedRevision {
                            revision_id: state.revision_id,
                            event_id: state.event_id,
                            record_hash: state.record_hash,
                            committed_at: state.committed_at,
                            root: state.root,
                        },
                    )
                })
                .collect(),
            objects: history
                .objects
                .into_iter()
                .map(|(hash, object)| (hash, object.bytes))
                .collect(),
        })
    }
}
