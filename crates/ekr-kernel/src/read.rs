//! Kernel-owned immutable read captures; projection consumers receive no storage authority.
use crate::{
    AuthorityStateV1, BootstrapContext, Commit, CommitError, SeedDocument, SeedResultV1,
    TransactionRecord,
};
use ekr_core::{ContentHash, EventId, RevisionId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{CanonicalGraph, Root};
use ekr_ontology::Ontology;
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
/// One revision's canonical state with the schema history of its lineage, from one verified
/// replay: what a view of that revision reads, without a replay per schema version.
/// Only the kernel constructs it; owning it grants no persistence authority.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct SchemaHistory {
    /// Admitted graph at the chosen revision.
    pub graph: CanonicalGraph,
    /// Coordinates of every revision from the seed through the chosen one.
    pub revisions: BTreeMap<RevisionNumber, VerifiedRevision>,
    /// The transaction that committed each of those revisions; the seed has none.
    pub transactions: BTreeMap<RevisionNumber, TransactionId>,
    /// The ontology in force from each of those revisions whose ontology root differs from the
    /// revision before it: the seed's, and one more for each revision that changed the schema.
    pub schemas: BTreeMap<RevisionNumber, Ontology>,
}
impl<S: RevisionLog + ObjectStore> Commit<S> {
    /// The graph at `revision` and the schema history of its lineage.
    ///
    /// One verified replay of the current history, continued from whatever this authority or the
    /// store's replay checkpoint already reached, holds every revision's coordinates, committing
    /// transaction and ontology. The graph at `revision` comes from that same replay when it holds
    /// it; only when it does not (a state continued from a checkpoint holds only its head's graph)
    /// is the history replayed once more, up to `revision`.
    /// # Errors
    /// [`CommitError::NotSeeded`], [`CommitError::RevisionNotFound`] beyond the head, or any
    /// required history/object verification failure.
    pub fn schema_history(&self, revision: RevisionNumber) -> Result<SchemaHistory, CommitError> {
        let history = self.store.history()?;
        let state = self
            .authority
            .reconstruct(&history, None, None)?
            .ok_or(CommitError::NotSeeded)?;
        let chosen = state
            .revisions
            .get(&revision)
            .ok_or(CommitError::RevisionNotFound { against: revision })?;
        let graph = match &chosen.graph {
            Some(graph) => CanonicalGraph::clone(graph),
            None => self.store.replay(revision)?,
        };
        if graph.revision != revision
            || ContentHash::of(&graph.ontology) != chosen.root.ontology_root
        {
            return Err(ekr_store::StoreError::Document("schema-history-disagrees".into()).into());
        }
        let mut revisions = BTreeMap::new();
        let mut schemas = BTreeMap::new();
        let mut in_force = None;
        for (number, held) in state.revisions.range(..=revision) {
            if in_force != Some(held.root.ontology_root) {
                schemas.insert(*number, Ontology::clone(&held.ontology));
                in_force = Some(held.root.ontology_root);
            }
            revisions.insert(
                *number,
                VerifiedRevision {
                    revision_id: held.revision_id,
                    event_id: held.event_id,
                    record_hash: held.record_hash,
                    committed_at: held.committed_at,
                    root: held.root,
                },
            );
        }
        let transactions = state
            .transactions
            .iter()
            .filter_map(|(id, record)| {
                record
                    .committed
                    .as_ref()
                    .map(|receipt| (receipt.result.revision, *id))
            })
            .filter(|(number, _)| *number <= revision)
            .collect();
        Ok(SchemaHistory {
            graph,
            revisions,
            transactions,
            schemas,
        })
    }
    /// How many replays this handle's authority has begun at the seed.
    #[must_use]
    pub fn seed_replays(&self) -> u64 {
        self.authority
            .cache
            .lock()
            .map_or(0, |cache| cache.seed_replays)
    }
    /// How many times this handle's authority has decoded the retained seed envelope in full.
    /// A diagnostic of read cost; it changes nothing.
    #[must_use]
    pub fn seed_envelope_decodes(&self) -> u64 {
        self.authority
            .cache
            .lock()
            .map_or(0, |cache| cache.envelope_decodes)
    }
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
        let state = std::sync::Arc::unwrap_or_clone(state);
        let envelope = self
            .authority
            .seed_envelope(&history, state.seed.seed_hash)?;
        let head = state.head();
        let graph = head.graph()?.clone();
        let root = head.root;
        Ok(VerifiedRead {
            graph,
            root,
            seed: state.seed,
            seed_input: envelope.input.clone(),
            context: envelope.context,
            authority: envelope.authority.clone(),
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
