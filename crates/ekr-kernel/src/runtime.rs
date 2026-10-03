//! Kernel-owned provider opening for consumers which must never depend on the raw store.
use crate::{AuthorityStateV1, BootstrapContext, Commit, SeedDocument, SeedError, SeedResultV1};
use ekr_core::{ContentHash, RevisionNumber, Timestamp};
use ekr_graph::{CanonicalGraph, Root};
use ekr_store::{FileStore, SqliteStore, StoreError};
use std::path::Path;

/// One event the provider log published, as [`Runtime::published_events`] returns it.
pub use ekr_store::PublishedEvent;

/// Public runtime facade over one private native provider and the shared kernel handlers.
pub struct Runtime {
    backend: Backend,
}
enum Backend {
    File(Box<Commit<FileStore>>),
    Sqlite(Box<Commit<SqliteStore>>),
}
impl Runtime {
    /// Lists evidence-backed questions without creating queue state or canonical revisions.
    /// # Errors
    /// Invalid history, missing retained evidence or provider failure.
    pub fn attention(
        &self,
    ) -> Result<Vec<ekr_core::contract_data::EkrKernelAttentionItem>, StoreError> {
        match &self.backend {
            Backend::File(k) => k.attention(),
            Backend::Sqlite(k) => k.attention(),
        }
    }
    /// Reads one currently unresolved typed attention subject.
    /// # Errors
    /// Unknown, settled or malformed subject, or invalid retained state.
    pub fn attention_item(
        &self,
        subject: &ekr_core::contract_data::EkrKernelAttentionSubject,
    ) -> Result<ekr_core::contract_data::EkrKernelAttentionItem, StoreError> {
        match &self.backend {
            Backend::File(k) => k.attention_item(subject),
            Backend::Sqlite(k) => k.attention_item(subject),
        }
    }
    /// Materializes parked candidate nodes, edges and assertions in the existing transient space.
    /// # Errors
    /// Unknown version, invalid retained input or provider failure.
    pub fn incubation_graph(
        &self,
        version: &ekr_core::contract_data::EkrIntegrateInterpretationVersion,
    ) -> Result<ekr_graph::TransientGraph, StoreError> {
        match &self.backend {
            Backend::File(k) => k.incubation_graph(version),
            Backend::Sqlite(k) => k.incubation_graph(version),
        }
    }
    /// Retains immutable local interpretations and reports canonical vocabulary gaps.
    /// # Errors
    /// Invalid input, missing evidence, changed version or persistence failure.
    pub fn import_interpretation(
        &self,
        input: &ekr_core::contract_data::EkrIntegrateInterpretationImport,
        at: Timestamp,
    ) -> Result<ekr_core::contract_data::EkrIntegrateIncubationImportReceipt, StoreError> {
        match &self.backend {
            Backend::File(k) => k.import_interpretation(input, at),
            Backend::Sqlite(k) => k.import_interpretation(input, at),
        }
    }
    /// Lists immutable local interpretation coordinates and byte digests.
    /// # Errors
    /// Corrupt retained records or persistence failure.
    pub fn interpretations(
        &self,
    ) -> Result<Vec<ekr_core::contract_data::EkrIntegrateInterpretationVersion>, StoreError> {
        match &self.backend {
            Backend::File(k) => k.interpretations(),
            Backend::Sqlite(k) => k.interpretations(),
        }
    }
    /// Reads local declarations, facts, blockers and processing receipts.
    /// # Errors
    /// Unknown version, changed digest, missing evidence or persistence failure.
    pub fn interpretation(
        &self,
        version: &ekr_core::contract_data::EkrIntegrateInterpretationVersion,
    ) -> Result<ekr_core::contract_data::EkrIntegrateInterpretationRead, StoreError> {
        match &self.backend {
            Backend::File(k) => k.interpretation(version),
            Backend::Sqlite(k) => k.interpretation(version),
        }
    }
    /// Retains a checked observation and its exact bytes without a canonical revision.
    /// # Errors
    /// Invalid source metadata, changed prior import, read-only store or provider failure.
    pub fn import_observation(
        &self,
        input: &ekr_core::contract_data::EkrObserveObservationImport,
        at: Timestamp,
    ) -> Result<ekr_core::contract_data::EkrObserveObservationImportReceipt, StoreError> {
        match &self.backend {
            Backend::File(k) => k.import_observation(input, at),
            Backend::Sqlite(k) => k.import_observation(input, at),
        }
    }
    /// Lists independently retained source records in identity order.
    /// # Errors
    /// Invalid retained metadata, missing content or provider failure.
    pub fn observations(
        &self,
    ) -> Result<Vec<ekr_core::contract_data::EkrGraphObservationRecord>, StoreError> {
        match &self.backend {
            Backend::File(k) => k.observations(),
            Backend::Sqlite(k) => k.observations(),
        }
    }
    /// Reads one observation and its exact retained payload after reopen.
    /// # Errors
    /// Unknown observation, invalid retained content or provider failure.
    pub fn observation(
        &self,
        id: ekr_core::ObservationId,
    ) -> Result<ekr_core::contract_data::EkrObserveRetainedObservationRead, StoreError> {
        match &self.backend {
            Backend::File(k) => k.observation(id),
            Backend::Sqlite(k) => k.observation(id),
        }
    }
    /// Captures admitted graph, retained records and payloads at one verified history boundary.
    /// # Errors
    /// Missing seed/revision or invalid required history.
    pub fn read(
        &self,
        revision: Option<RevisionNumber>,
    ) -> Result<crate::VerifiedRead, crate::CommitError> {
        match &self.backend {
            Backend::File(k) => k.read(revision),
            Backend::Sqlite(k) => k.read(revision),
        }
    }
    /// The graph at `revision` and the schema history of its lineage, from one verified replay.
    /// # Errors
    /// Missing seed/revision or invalid required history.
    pub fn schema_history(
        &self,
        revision: RevisionNumber,
    ) -> Result<crate::SchemaHistory, crate::CommitError> {
        match &self.backend {
            Backend::File(k) => k.schema_history(revision),
            Backend::Sqlite(k) => k.schema_history(revision),
        }
    }
    /// Bounded reader ingress using the same frozen parser and retained proposal handler.
    /// # Errors
    /// Input limits, invalid document, authority/state refusal or provider failure.
    pub fn propose_reader(
        &self,
        reader: impl std::io::Read,
        actor: ekr_core::AgentId,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<crate::ProposalRecordV1, crate::CommitError> {
        match &self.backend {
            Backend::File(k) => k.propose_reader(reader, actor, now),
            Backend::Sqlite(k) => k.propose_reader(reader, actor, now),
        }
    }
    /// Reads every actual retained transaction record in one verified history capture. The
    /// snapshot is materialized on first read and shared by later reads of the same state.
    /// # Errors
    /// Missing seed or invalid retained history.
    pub fn transactions(
        &self,
    ) -> Result<
        std::sync::Arc<
            std::collections::BTreeMap<ekr_core::TransactionId, crate::TransactionRecord>,
        >,
        crate::CommitError,
    > {
        match &self.backend {
            Backend::File(k) => k.transactions(),
            Backend::Sqlite(k) => k.transactions(),
        }
    }
    /// Reads requested transaction lifecycle states at one verified boundary. Unknown ids are
    /// absent; the complete retained history is still verified when no ids are requested.
    /// # Errors
    /// Missing seed or invalid retained history.
    pub fn transaction_states(
        &self,
        ids: impl IntoIterator<Item = ekr_core::TransactionId>,
    ) -> Result<
        std::collections::BTreeMap<ekr_core::TransactionId, crate::TransactionState>,
        crate::CommitError,
    > {
        match &self.backend {
            Backend::File(k) => k.transaction_states(ids),
            Backend::Sqlite(k) => k.transaction_states(ids),
        }
    }
    /// Publishes the exact submitted transaction document through the shared handler.
    /// # Errors
    /// Invalid document, authority/state refusal or provider failure.
    pub fn propose(
        &self,
        bytes: &[u8],
        actor: ekr_core::AgentId,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<crate::ProposalRecordV1, crate::CommitError> {
        match &self.backend {
            Backend::File(k) => k.propose(bytes, actor, now),
            Backend::Sqlite(k) => k.propose(bytes, actor, now),
        }
    }
    /// Validates a retained proposal against the requested committed revision.
    /// # Errors
    /// Missing transaction/revision, wrong state or failed verification/publication.
    pub fn validate(
        &self,
        id: ekr_core::TransactionId,
        against: RevisionNumber,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<crate::ValidationCommandResult, crate::CommitError> {
        match &self.backend {
            Backend::File(k) => k.validate(id, against, now),
            Backend::Sqlite(k) => k.validate(id, against, now),
        }
    }
    /// Applies or returns the actual retained decision under trusted host identity and lazy time.
    /// # Errors
    /// Missing transaction, state/time/authority refusal or failed persistence.
    pub fn commit(
        &self,
        id: ekr_core::TransactionId,
        actor: ekr_core::AgentId,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<crate::CommitCommandResult, crate::CommitError> {
        match &self.backend {
            Backend::File(k) => k.commit(id, actor, now),
            Backend::Sqlite(k) => k.commit(id, actor, now),
        }
    }
    /// Opens the File provider under the explicit trusted host anchor.
    /// # Errors
    /// Invalid authority, runtime-context refusal or provider failure.
    pub fn file(
        path: &Path,
        tenant: &str,
        context: BootstrapContext,
        anchor: AuthorityStateV1,
    ) -> Result<Self, StoreError> {
        Ok(Self {
            backend: Backend::File(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| FileStore::file(path, tenant, None).map(|store| store.under(authority)),
            )?)),
        })
    }
    /// Opens the SQLite provider under the explicit trusted host anchor.
    /// # Errors
    /// Invalid authority, runtime-context refusal or provider failure.
    pub fn sqlite(
        path: &Path,
        tenant: &str,
        context: BootstrapContext,
        anchor: AuthorityStateV1,
    ) -> Result<Self, StoreError> {
        Ok(Self {
            backend: Backend::Sqlite(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    SqliteStore::sqlite(path, tenant, None).map(|store| store.under(authority))
                },
            )?)),
        })
    }
    /// Opens an already provisioned File store under the explicit trusted host anchor, creating
    /// nothing at a path that holds no store.
    /// # Errors
    /// Invalid authority, runtime-context refusal, a missing store or provider failure.
    pub fn file_existing(
        path: &Path,
        tenant: &str,
        context: BootstrapContext,
        anchor: AuthorityStateV1,
    ) -> Result<Self, StoreError> {
        Ok(Self {
            backend: Backend::File(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    FileStore::file_existing(path, tenant, None).map(|store| store.under(authority))
                },
            )?)),
        })
    }
    /// Opens an already provisioned SQLite store under the explicit trusted host anchor, creating
    /// nothing at a path that holds no store.
    /// # Errors
    /// Invalid authority, runtime-context refusal, a missing store or provider failure.
    pub fn sqlite_existing(
        path: &Path,
        tenant: &str,
        context: BootstrapContext,
        anchor: AuthorityStateV1,
    ) -> Result<Self, StoreError> {
        Ok(Self {
            backend: Backend::Sqlite(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    SqliteStore::sqlite_existing(path, tenant, None)
                        .map(|store| store.under(authority))
                },
            )?)),
        })
    }
    /// Opens an already provisioned File store for a caller that only reads it, under the explicit
    /// trusted host anchor: as [`Runtime::file_existing`] where this process may write the store,
    /// and otherwise read-only ([`Runtime::is_read_only`]), writing nothing at its path.
    /// # Errors
    /// Invalid authority, runtime-context refusal, a missing store or provider failure.
    pub fn file_reading(
        path: &Path,
        tenant: &str,
        context: BootstrapContext,
        anchor: AuthorityStateV1,
    ) -> Result<Self, StoreError> {
        Ok(Self {
            backend: Backend::File(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    FileStore::file_reading(path, tenant, None).map(|store| store.under(authority))
                },
            )?)),
        })
    }
    /// Opens an already provisioned SQLite store for a caller that only reads it, under the
    /// explicit trusted host anchor: as [`Runtime::sqlite_existing`] where this process may write
    /// the store, and otherwise read-only ([`Runtime::is_read_only`]), writing nothing at its path.
    /// # Errors
    /// Invalid authority, runtime-context refusal, a missing store or provider failure.
    pub fn sqlite_reading(
        path: &Path,
        tenant: &str,
        context: BootstrapContext,
        anchor: AuthorityStateV1,
    ) -> Result<Self, StoreError> {
        Ok(Self {
            backend: Backend::Sqlite(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    SqliteStore::sqlite_reading(path, tenant, None)
                        .map(|store| store.under(authority))
                },
            )?)),
        })
    }
    /// Whether this runtime's store was opened read-only: every write through it is refused as
    /// [`StoreError::ReadOnly`] and nothing is written at the store's path.
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        match &self.backend {
            Backend::File(kernel) => kernel.store.is_read_only(),
            Backend::Sqlite(kernel) => kernel.store.is_read_only(),
        }
    }
    /// Whether this runtime's store was opened read-only and the files at its path have changed
    /// since it read them: a long-lived reader then opens the store again, as for a store
    /// replaced at its path. Never true of a store opened for writing.
    #[must_use]
    pub fn source_changed(&self) -> bool {
        match &self.backend {
            Backend::File(kernel) => kernel.store.source_changed(),
            Backend::Sqlite(kernel) => kernel.store.source_changed(),
        }
    }
    /// Removes the private copy every read-only File store of this process reads, for a process
    /// about to end without dropping its runtimes: a signal handler's last act. A runtime that
    /// reads after this fails.
    pub fn remove_read_only_copies() {
        ekr_store::remove_read_only_copies();
    }
    /// Replays every read of this runtime from the seed, re-deriving every retained decision,
    /// instead of continuing from the store's replay checkpoint (design § 96). Checkpoints are
    /// still written. Takes effect only before the first read.
    pub fn set_full_replay(&mut self, full: bool) {
        match &mut self.backend {
            Backend::File(kernel) => kernel.store.set_full_replay(full),
            Backend::Sqlite(kernel) => kernel.store.set_full_replay(full),
        }
    }
    /// The check every constructor runs on the trusted host anchor before it touches a provider,
    /// run alone: a host that decides something about the store path first calls this, so an
    /// anchor refusal is reported before anything about the path.
    /// # Errors
    /// The anchor refusal the constructors report.
    pub fn check_anchor(
        context: BootstrapContext,
        anchor: &AuthorityStateV1,
    ) -> Result<(), StoreError> {
        anchor.check(context)
    }
    /// The kernel's full seed admission of `document`, without a provider: what
    /// [`Runtime::seed`] would refuse a new seed for. A host about to create a store for a seed
    /// calls this first, so a refused seed creates nothing.
    /// # Errors
    /// [`SeedError::Invalid`] for a seed the kernel does not admit.
    pub fn admit_seed(document: &SeedDocument, context: BootstrapContext) -> Result<(), SeedError> {
        // Admission does not depend on the instant: it only stamps the admitted assertions.
        crate::seed::admitted_graph(document, context, Timestamp::EPOCH).map(|_| ())
    }
    /// Executes the shared seed handler. A retained retry never calls the supplied host clock.
    /// # Errors
    /// Invalid seed, different existing seed or failed native publication.
    pub fn seed(
        &self,
        document: SeedDocument,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<SeedResultV1, SeedError> {
        match &self.backend {
            Backend::File(kernel) => kernel.seed(document, now),
            Backend::Sqlite(kernel) => kernel.seed(document, now),
        }
    }
    /// The complete verified current root.
    /// # Errors
    /// Invalid retained history.
    pub fn head(&self) -> Result<Option<Root>, StoreError> {
        match &self.backend {
            Backend::File(kernel) => kernel.head(),
            Backend::Sqlite(kernel) => kernel.head(),
        }
    }
    /// The complete current canonical graph.
    /// # Errors
    /// Missing seed or invalid retained history.
    pub fn snapshot(&self) -> Result<CanonicalGraph, StoreError> {
        match &self.backend {
            Backend::File(kernel) => kernel.snapshot(),
            Backend::Sqlite(kernel) => kernel.snapshot(),
        }
    }
    /// Reconstructs exactly the selected committed revision.
    /// # Errors
    /// Missing revision or invalid required history.
    pub fn replay(&self, revision: RevisionNumber) -> Result<CanonicalGraph, StoreError> {
        match &self.backend {
            Backend::File(kernel) => kernel.replay(revision),
            Backend::Sqlite(kernel) => kernel.replay(revision),
        }
    }
    /// At rest — a session at the end of its input: writes the replay checkpoint of the newest
    /// head this runtime reached when it is past the retained checkpoint (design § 99.5). Best
    /// effort: a checkpoint that is not written costs a later open time, never an answer.
    pub fn retain_checkpoint_at_rest(&self) {
        match &self.backend {
            Backend::File(kernel) => kernel.retain_checkpoint_at_rest(),
            Backend::Sqlite(kernel) => kernel.retain_checkpoint_at_rest(),
        }
    }
    /// How many replays this runtime's kernel has begun at the seed since it was opened: every
    /// verified read that could not continue from a state it had already reached, or from the
    /// store's replay checkpoint, counts one. A diagnostic of read cost; it changes nothing.
    #[must_use]
    pub fn seed_replays(&self) -> u64 {
        match &self.backend {
            Backend::File(kernel) => kernel.seed_replays(),
            Backend::Sqlite(kernel) => kernel.seed_replays(),
        }
    }
    /// How many times this runtime's kernel has decoded the retained seed envelope in full since
    /// it was opened. The kernel keeps the envelope it decoded for the seed it names, so this is
    /// at most one per seed. A diagnostic of read cost; it changes nothing.
    #[must_use]
    pub fn seed_envelope_decodes(&self) -> u64 {
        match &self.backend {
            Backend::File(kernel) => kernel.seed_envelope_decodes(),
            Backend::Sqlite(kernel) => kernel.seed_envelope_decodes(),
        }
    }
    /// Every event the provider log has published, in log order, through the provider handle
    /// this runtime already holds. It reads and interprets nothing beyond the log: kernel
    /// occurrences, publication preparations and stored objects come back as logged.
    /// # Errors
    /// Runtime-context refusal, provider failure or a log that disagrees with itself.
    pub fn published_events(&self) -> Result<Vec<PublishedEvent>, StoreError> {
        match &self.backend {
            Backend::File(kernel) => kernel.store.published_events(),
            Backend::Sqlite(kernel) => kernel.store.published_events(),
        }
    }
    /// The preserving migration (design § 100.3): this store's complete retained history
    /// re-published into `destination`, a store under the same host anchor that holds nothing yet,
    /// with its seed under `ekr-seed-envelope/3`. This store is only read; the destination is
    /// replayed in full and compared with it before the report is returned.
    /// # Errors
    /// `migrate-destination-not-empty`, `migrate-unresolved-preparation`, any refusal of either
    /// store's replay, and `migrate-verification-disagrees`.
    pub fn migrate_into(
        &self,
        destination: &Runtime,
    ) -> Result<crate::StoreMigrationV1, crate::CommitError> {
        match (&self.backend, &destination.backend) {
            (Backend::File(source), Backend::File(into)) => source.migrate_into(into),
            (Backend::File(source), Backend::Sqlite(into)) => source.migrate_into(into),
            (Backend::Sqlite(source), Backend::File(into)) => source.migrate_into(into),
            (Backend::Sqlite(source), Backend::Sqlite(into)) => source.migrate_into(into),
        }
    }
    /// Reads verified retained content through the shared handler.
    /// # Errors
    /// Invalid history or corrupt native blob binding.
    pub fn content(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        match &self.backend {
            Backend::File(kernel) => kernel.content(hash),
            Backend::Sqlite(kernel) => kernel.content(hash),
        }
    }
}
