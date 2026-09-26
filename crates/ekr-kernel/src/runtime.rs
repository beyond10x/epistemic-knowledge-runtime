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
