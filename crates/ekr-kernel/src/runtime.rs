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
    /// Reads every actual retained transaction state in one verified history capture.
    /// # Errors
    /// Missing seed or invalid retained history.
    pub fn transactions(
        &self,
    ) -> Result<
        std::collections::BTreeMap<ekr_core::TransactionId, crate::TransactionRecord>,
        crate::CommitError,
    > {
        match &self.backend {
            Backend::File(k) => k.transactions(),
            Backend::Sqlite(k) => k.transactions(),
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
