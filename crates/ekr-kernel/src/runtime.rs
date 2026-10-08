//! Kernel-owned provider opening for consumers which must never depend on the raw store.
//!
//! Every opener here takes the tenant a host configuration names, and refuses one that carries
//! the marker reserved to stage tenants (`stage-tenant-reserved`, design § 107.1) before it
//! reads, creates or writes anything: no store's tenant is ever a stage's derived tenant.
use crate::{
    AuthorityStateV1, BootstrapContext, Commit, CommitError, SeedDocument, SeedError, SeedResultV1,
    StageListing,
};
use ekr_core::{ContentHash, RevisionNumber, StageId, Timestamp};
use ekr_graph::{CanonicalGraph, Root};
use ekr_store::{
    admit_store_tenant, FileStore, PostgresStore, ProviderKind, SqliteStore, StageResult,
    StageState, StoreError,
};

pub use ekr_store::postgres::{PostgresConfiguration, PostgresPool};
use std::path::{Path, PathBuf};

/// One event the provider log published, as [`Runtime::published_events`] returns it.
pub use ekr_store::PublishedEvent;

/// Public runtime facade over one private native provider and the shared kernel handlers.
pub struct Runtime {
    backend: Backend,
    /// Where the store is, so that a stage command can open the stage's tenant of the same store.
    location: Location,
    /// The store's own tenant, the one its host configuration names: also for a runtime joined to
    /// a stage, whose handle reads and writes the stage's tenant.
    tenant: String,
    /// The stage this runtime is joined to (design § 107.3), if any.
    joined: Option<StageId>,
}
enum Backend {
    File(Box<Commit<FileStore>>),
    Sqlite(Box<Commit<SqliteStore>>),
    Postgres(Box<Commit<PostgresStore>>),
}
/// Where a runtime's store is, and how it was opened.
#[derive(Clone)]
enum Location {
    File,
    Sqlite {
        path: PathBuf,
        open: SqliteOpen,
    },
    Postgres {
        config: Box<PostgresConfiguration>,
        reading: bool,
    },
}
/// How a SQLite runtime opened its store: for writing, for a caller that only reads, or as one
/// captured image.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SqliteOpen {
    Existing,
    Reading,
    Image,
}
impl Runtime {
    /// Provisions the hosted provider's schema using separate schema-management credentials.
    /// # Errors
    /// Configuration, TLS, schema or provider refusal.
    pub fn postgres_schema(config: &PostgresConfiguration) -> Result<(), StoreError> {
        PostgresStore::postgres_schema(config)
    }

    /// Opens a hosted PostgreSQL store under the explicit trusted host anchor. `reading`
    /// disables all mutations, including checkpoint writes. Schema is never created here.
    /// # Errors
    /// Anchor, configuration, role, TLS, budget or provider refusal.
    pub fn postgres(
        config: &PostgresConfiguration,
        tenant: &str,
        context: BootstrapContext,
        anchor: AuthorityStateV1,
        reading: bool,
    ) -> Result<Self, StoreError> {
        admit_store_tenant(tenant)?;
        Ok(Self {
            backend: Backend::Postgres(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    PostgresStore::postgres(config, tenant, reading)
                        .map(|store| store.under(authority))
                },
            )?)),
            location: Location::Postgres {
                config: Box::new(config.clone()),
                reading,
            },
            tenant: tenant.to_owned(),
            joined: None,
        })
    }

    /// Captures one consistent SQLite image even when the source is writable. Subsequent reads
    /// stay on that image, and every write through this handle is refused.
    /// # Errors
    /// Invalid anchor, missing store, failed snapshot capture or provider refusal.
    pub fn sqlite_snapshot(
        path: &Path,
        tenant: &str,
        context: BootstrapContext,
        anchor: AuthorityStateV1,
    ) -> Result<Self, StoreError> {
        admit_store_tenant(tenant)?;
        Ok(Self {
            backend: Backend::Sqlite(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    SqliteStore::sqlite_read_only(path, tenant, None)
                        .map(|store| store.under(authority))
                },
            )?)),
            location: Location::Sqlite {
                path: path.to_owned(),
                open: SqliteOpen::Image,
            },
            tenant: tenant.to_owned(),
            joined: None,
        })
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
            Backend::Postgres(k) => k.read(revision),
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
            Backend::Postgres(k) => k.schema_history(revision),
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
            Backend::Postgres(k) => k.propose_reader(reader, actor, now),
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
            Backend::Postgres(k) => k.transactions(),
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
            Backend::Postgres(k) => k.transaction_states(ids),
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
            Backend::Postgres(k) => k.propose(bytes, actor, now),
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
            Backend::Postgres(k) => k.validate(id, against, now),
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
            Backend::Postgres(k) => k.commit(id, actor, now),
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
        admit_store_tenant(tenant)?;
        Ok(Self {
            backend: Backend::File(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| FileStore::file(path, tenant, None).map(|store| store.under(authority)),
            )?)),
            location: Location::File,
            tenant: tenant.to_owned(),
            joined: None,
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
        admit_store_tenant(tenant)?;
        Ok(Self {
            backend: Backend::Sqlite(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    SqliteStore::sqlite(path, tenant, None).map(|store| store.under(authority))
                },
            )?)),
            location: Location::Sqlite {
                path: path.to_owned(),
                open: SqliteOpen::Existing,
            },
            tenant: tenant.to_owned(),
            joined: None,
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
        admit_store_tenant(tenant)?;
        Ok(Self {
            backend: Backend::File(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    FileStore::file_existing(path, tenant, None).map(|store| store.under(authority))
                },
            )?)),
            location: Location::File,
            tenant: tenant.to_owned(),
            joined: None,
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
        admit_store_tenant(tenant)?;
        Ok(Self {
            backend: Backend::Sqlite(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    SqliteStore::sqlite_existing(path, tenant, None)
                        .map(|store| store.under(authority))
                },
            )?)),
            location: Location::Sqlite {
                path: path.to_owned(),
                open: SqliteOpen::Existing,
            },
            tenant: tenant.to_owned(),
            joined: None,
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
        admit_store_tenant(tenant)?;
        Ok(Self {
            backend: Backend::File(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    FileStore::file_reading(path, tenant, None).map(|store| store.under(authority))
                },
            )?)),
            location: Location::File,
            tenant: tenant.to_owned(),
            joined: None,
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
        admit_store_tenant(tenant)?;
        Ok(Self {
            backend: Backend::Sqlite(Box::new(Commit::over_with_authority(
                context,
                anchor,
                |authority| {
                    SqliteStore::sqlite_reading(path, tenant, None)
                        .map(|store| store.under(authority))
                },
            )?)),
            location: Location::Sqlite {
                path: path.to_owned(),
                open: SqliteOpen::Reading,
            },
            tenant: tenant.to_owned(),
            joined: None,
        })
    }
    /// Whether this runtime's store was opened read-only: every write through it is refused as
    /// [`StoreError::ReadOnly`] and nothing is written at the store's path.
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        match &self.backend {
            Backend::File(kernel) => kernel.store.is_read_only(),
            Backend::Sqlite(kernel) => kernel.store.is_read_only(),
            Backend::Postgres(kernel) => kernel.store.is_read_only(),
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
            Backend::Postgres(kernel) => kernel.store.source_changed(),
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
            Backend::Postgres(kernel) => kernel.store.set_full_replay(full),
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
            Backend::Postgres(kernel) => kernel.seed(document, now),
        }
    }
    /// Executes the shared seed handler only if the lineage has no seed: a seed already there —
    /// published, or elected and not yet published, the identical document included — refuses
    /// as `AlreadySeeded`, and so does losing the store's election for the seed to another caller.
    /// An elected seed not yet published is published first, so the lineage is seeded whenever
    /// this refuses. `Ok` means this call wrote the seed.
    /// # Errors
    /// Invalid seed, any existing or concurrently elected seed, or failed native publication.
    pub fn seed_if_absent(
        &self,
        document: SeedDocument,
        now: impl FnOnce() -> Timestamp,
    ) -> Result<SeedResultV1, SeedError> {
        match &self.backend {
            Backend::File(kernel) => kernel.seed_if_absent(document, now),
            Backend::Sqlite(kernel) => kernel.seed_if_absent(document, now),
            Backend::Postgres(kernel) => kernel.seed_if_absent(document, now),
        }
    }
    /// The complete verified current root.
    /// # Errors
    /// Invalid retained history.
    pub fn head(&self) -> Result<Option<Root>, StoreError> {
        match &self.backend {
            Backend::File(kernel) => kernel.head(),
            Backend::Sqlite(kernel) => kernel.head(),
            Backend::Postgres(kernel) => kernel.head(),
        }
    }
    /// The complete current canonical graph.
    /// # Errors
    /// Missing seed or invalid retained history.
    pub fn snapshot(&self) -> Result<CanonicalGraph, StoreError> {
        match &self.backend {
            Backend::File(kernel) => kernel.snapshot(),
            Backend::Sqlite(kernel) => kernel.snapshot(),
            Backend::Postgres(kernel) => kernel.snapshot(),
        }
    }
    /// Reconstructs exactly the selected committed revision.
    /// # Errors
    /// Missing revision or invalid required history.
    pub fn replay(&self, revision: RevisionNumber) -> Result<CanonicalGraph, StoreError> {
        match &self.backend {
            Backend::File(kernel) => kernel.replay(revision),
            Backend::Sqlite(kernel) => kernel.replay(revision),
            Backend::Postgres(kernel) => kernel.replay(revision),
        }
    }
    /// At rest — a session at the end of its input: writes the replay checkpoint of the newest
    /// head this runtime reached when it is past the retained checkpoint (design § 99.5). Best
    /// effort: a checkpoint that is not written costs a later open time, never an answer.
    pub fn retain_checkpoint_at_rest(&self) {
        match &self.backend {
            Backend::File(kernel) => kernel.retain_checkpoint_at_rest(),
            Backend::Sqlite(kernel) => kernel.retain_checkpoint_at_rest(),
            Backend::Postgres(kernel) => kernel.retain_checkpoint_at_rest(),
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
            Backend::Postgres(kernel) => kernel.seed_replays(),
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
            Backend::Postgres(kernel) => kernel.seed_envelope_decodes(),
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
            Backend::Postgres(kernel) => kernel.store.published_events(),
        }
    }
    /// The preserving migration (design § 100.3): a local store's complete retained history
    /// re-published into `destination`, a store under the same host anchor that holds nothing yet,
    /// with its seed under `ekr-seed-envelope/3`. This store is only read; the destination is
    /// replayed in full and compared with it before the report is returned.
    /// # Errors
    /// `migrate-source-not-supported` for PostgreSQL sources, `migrate-destination-not-empty`,
    /// `migrate-unresolved-preparation`, any refusal of either
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
            (Backend::File(source), Backend::Postgres(into)) => source.migrate_into(into),
            (Backend::Sqlite(source), Backend::Postgres(into)) => source.migrate_into(into),
            (Backend::Postgres(_), _) => Err(StoreError::Document(
                "migrate-source-not-supported: PostgreSQL source snapshots are not supported"
                    .into(),
            )
            .into()),
        }
    }
    /// Reads verified retained content through the shared handler.
    /// # Errors
    /// Invalid history or corrupt native blob binding.
    pub fn content(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        match &self.backend {
            Backend::File(kernel) => kernel.content(hash),
            Backend::Sqlite(kernel) => kernel.content(hash),
            Backend::Postgres(kernel) => kernel.content(hash),
        }
    }
}

/// A run staged and published whole, or dropped whole (design § 107). The four commands
/// (`ekr.cli.BeginStage`, `SealStage`, `PublishStage`, `AbandonStage`) run on the store's own
/// runtime; [`Runtime::join_stage`] gives the runtime every store verb of the run uses. Every
/// refusal is a [`StoreError`] named in `systems/ekr/domains/store.yaml`, inside
/// [`CommitError::Store`].
impl Runtime {
    /// The provider this runtime's store is on.
    #[must_use]
    pub const fn provider(&self) -> ProviderKind {
        match self.backend {
            Backend::File(_) => ProviderKind::File,
            Backend::Sqlite(_) => ProviderKind::Sqlite,
            Backend::Postgres(_) => ProviderKind::Postgres,
        }
    }
    /// The stage this runtime is joined to ([`Runtime::join_stage`]), if any.
    #[must_use]
    pub const fn joined_stage(&self) -> Option<StageId> {
        self.joined
    }
    /// The store's own tenant: for a runtime joined to a stage too.
    #[must_use]
    pub fn store_tenant(&self) -> &str {
        &self.tenant
    }
    /// Refuses a stage command on a runtime joined to a stage: stage commands run on the store.
    fn on_store(&self) -> Result<(), CommitError> {
        match self.joined {
            Some(stage) => Err(StoreError::Document(format!(
                "stage-command-on-stage: this runtime is joined to stage {stage}; a stage command \
                 runs on the store's own runtime"
            ))
            .into()),
            None => Ok(()),
        }
    }
    /// A SQLite handle on `tenant` of this runtime's database, under `kernel`'s context and anchor.
    fn sqlite_kernel(
        kernel: &Commit<SqliteStore>,
        path: &Path,
        tenant: &str,
        open: SqliteOpen,
    ) -> Result<Commit<SqliteStore>, StoreError> {
        Commit::over_with_authority(
            kernel.authority.context,
            kernel.authority.anchor.clone(),
            |authority| {
                match open {
                    SqliteOpen::Existing => SqliteStore::sqlite_existing(path, tenant, None),
                    SqliteOpen::Reading => SqliteStore::sqlite_reading(path, tenant, None),
                    SqliteOpen::Image => SqliteStore::sqlite_read_only(path, tenant, None),
                }
                .map(|store| store.under(authority))
            },
        )
    }
    /// A PostgreSQL handle on `tenant` at this runtime's location, under `kernel`'s context and
    /// anchor.
    fn postgres_kernel(
        kernel: &Commit<PostgresStore>,
        config: &PostgresConfiguration,
        tenant: &str,
        reading: bool,
    ) -> Result<Commit<PostgresStore>, StoreError> {
        Commit::over_with_authority(
            kernel.authority.context,
            kernel.authority.anchor.clone(),
            |authority| {
                PostgresStore::postgres(config, tenant, reading).map(|store| store.under(authority))
            },
        )
    }
    /// `ekr.cli.BeginStage`: mints a stage id, records the stage Begun at the store's head and
    /// copies the store at that head into the stage's own tenant (design § 107.2). The store's
    /// head and revision stream do not move.
    /// # Errors
    /// `stage-unsupported-provider` on File; [`CommitError::NotSeeded`];
    /// `unresolved-preparation`; any refusal of the store's full replay or of the copy.
    pub fn begin_stage(&self) -> Result<StageResult, CommitError> {
        self.on_store()?;
        match (&self.backend, &self.location) {
            (Backend::Sqlite(kernel), Location::Sqlite { path, .. }) => {
                let image = Self::sqlite_kernel(kernel, path, &self.tenant, SqliteOpen::Image)?;
                kernel.begin_stage(&image, |tenant| {
                    Self::sqlite_kernel(kernel, path, tenant, SqliteOpen::Existing)
                })
            }
            (Backend::Postgres(kernel), Location::Postgres { config, .. }) => kernel
                .begin_stage(kernel, |tenant| {
                    Self::postgres_kernel(kernel, config, tenant, false)
                }),
            _ => Err(StoreError::StageUnsupportedProvider(self.provider()).into()),
        }
    }
    /// `ekr.cli.SealStage`: seals a Begun stage whose copy is complete and which holds no decision
    /// elected and never published, while the store's head is `expect_head` and the stage's base.
    /// A stage already Sealing answers its original result.
    /// # Errors
    /// `stage-not-found`, `stage-head-moved`, `stage-incomplete`, `unresolved-preparation`, and
    /// `stage-already-published` or `stage-already-abandoned`.
    pub fn seal_stage(
        &self,
        stage: StageId,
        expect_head: RevisionNumber,
    ) -> Result<StageResult, CommitError> {
        self.on_store()?;
        match (&self.backend, &self.location) {
            (Backend::Sqlite(kernel), Location::Sqlite { path, .. }) => {
                kernel.seal_stage(stage, expect_head, |tenant| {
                    Self::sqlite_kernel(kernel, path, tenant, SqliteOpen::Image)
                })
            }
            (Backend::Postgres(kernel), Location::Postgres { config, .. }) => {
                kernel.seal_stage(stage, expect_head, |tenant| {
                    Self::postgres_kernel(kernel, config, tenant, false)
                })
            }
            _ => Err(StoreError::StageUnsupportedProvider(self.provider()).into()),
        }
    }
    /// `ekr.cli.PublishStage`: publishes a Sealing stage's suffix into the store in one append
    /// group and forgets the stage's tenant; resumes an elected attempt after an unknown outcome;
    /// returns the original result for a Published stage retried with the same expected head.
    /// # Errors
    /// `stage-not-found`, `stage-not-sealed`, `stage-already-abandoned` (also when an abandonment
    /// landed meanwhile), `stage-already-published` for another expected head,
    /// `stage-head-moved`, `stage-stream-moved`, `stage-object-moved`, `unresolved-preparation`,
    /// `stage-incomplete` and `stage-suffix-refused`.
    pub fn publish_stage(
        &self,
        stage: StageId,
        expect_head: RevisionNumber,
    ) -> Result<StageResult, CommitError> {
        self.on_store()?;
        match (&self.backend, &self.location) {
            (Backend::Sqlite(kernel), Location::Sqlite { path, .. }) => {
                kernel.publish_stage(stage, expect_head, |tenant| {
                    Self::sqlite_kernel(kernel, path, tenant, SqliteOpen::Image)
                })
            }
            (Backend::Postgres(kernel), Location::Postgres { config, .. }) => {
                kernel.publish_stage(stage, expect_head, |tenant| {
                    Self::postgres_kernel(kernel, config, tenant, false)
                })
            }
            _ => Err(StoreError::StageUnsupportedProvider(self.provider()).into()),
        }
    }
    /// `ekr stage publish <id> --expect-head <revision>` (design § 107.4): reads the stage's
    /// record and runs [`Runtime::seal_stage`] then [`Runtime::publish_stage`] on a Begun stage,
    /// and [`Runtime::publish_stage`] alone otherwise.
    /// # Errors
    /// What either command refuses.
    pub fn seal_and_publish_stage(
        &self,
        stage: StageId,
        expect_head: RevisionNumber,
    ) -> Result<StageResult, CommitError> {
        self.on_store()?;
        match (&self.backend, &self.location) {
            (Backend::Sqlite(kernel), Location::Sqlite { path, .. }) => kernel
                .seal_and_publish_stage(stage, expect_head, |tenant| {
                    Self::sqlite_kernel(kernel, path, tenant, SqliteOpen::Image)
                }),
            (Backend::Postgres(kernel), Location::Postgres { config, .. }) => kernel
                .seal_and_publish_stage(stage, expect_head, |tenant| {
                    Self::postgres_kernel(kernel, config, tenant, false)
                }),
            _ => Err(StoreError::StageUnsupportedProvider(self.provider()).into()),
        }
    }
    /// `ekr.cli.AbandonStage`: records a Begun or Sealing stage Abandoned and forgets its tenant;
    /// an Abandoned stage answers its original result and finishes the forgetting.
    /// # Errors
    /// `stage-not-found`, `stage-already-published`, `stage-unsupported-provider` on File.
    pub fn abandon_stage(&self, stage: StageId) -> Result<StageResult, CommitError> {
        self.on_store()?;
        match &self.backend {
            Backend::Sqlite(kernel) => kernel.abandon_stage(stage),
            Backend::Postgres(kernel) => kernel.abandon_stage(stage),
            Backend::File(_) => {
                Err(StoreError::StageUnsupportedProvider(ProviderKind::File).into())
            }
        }
    }
    /// Every stage the store has recorded, in every state (`ekr.cli.Stages`).
    /// # Errors
    /// Provider failure or a record that does not read.
    pub fn stages(&self) -> Result<Vec<StageListing>, CommitError> {
        self.on_store()?;
        match &self.backend {
            Backend::Sqlite(kernel) => kernel.stages(),
            Backend::Postgres(kernel) => kernel.stages(),
            Backend::File(_) => Ok(Vec::new()),
        }
    }
    /// How many events the tenant of `stage` holds, the tenant its `StageBegun` names: zero once
    /// the stage is abandoned or published and its tenant forgotten (design §§ 107.6, 107.9). A
    /// diagnostic of what a stage leaves behind, read through a handle on that tenant that holds
    /// no kernel authority and appends no event: the provider log of one SQLite image, or one
    /// PostgreSQL capture, never the change feed, which can withhold committed events (design
    /// § 107.12). Test support, not part of a store's reading surface: it decides nothing.
    ///
    /// On PostgreSQL the capture first ensures the tenant's capture identity, as every capture from
    /// a writing handle does: after the stage's tenant is forgotten it writes one row of provider
    /// metadata for that tenant (eventlog-postgres `_identity`), and no event, object or blob. No
    /// store reader sees it: every store verb reads the store's own tenant, where the stage's
    /// record is, and a joined verb is refused before it reads the stage's tenant once the stage is
    /// published or abandoned. A later forgetting of the tenant removes it with the rest.
    /// # Errors
    /// `stage-not-found`, `stage-unsupported-provider` on File, or provider failure.
    #[doc(hidden)]
    pub fn stage_tenant_events(&self, stage: StageId) -> Result<usize, CommitError> {
        self.on_store()?;
        let record = match &self.backend {
            Backend::Sqlite(kernel) => ekr_store::StageLog::stage_record(&kernel.store, stage)?,
            Backend::Postgres(kernel) => ekr_store::StageLog::stage_record(&kernel.store, stage)?,
            Backend::File(_) => {
                return Err(StoreError::StageUnsupportedProvider(ProviderKind::File).into())
            }
        }
        .ok_or(StoreError::StageNotFound(stage))?;
        let events = match &self.location {
            Location::Sqlite { path, .. } => {
                SqliteStore::sqlite_read_only(path, &record.tenant, None)?
                    .published_events()?
                    .len()
            }
            Location::Postgres { config, .. } => {
                PostgresStore::postgres(config, &record.tenant, false)?
                    .inventory()?
                    .events
            }
            Location::File => {
                return Err(StoreError::StageUnsupportedProvider(ProviderKind::File).into())
            }
        };
        Ok(events)
    }
    /// The runtime of a verb joined to `stage` (design § 107.3): the stage's tenant of this store,
    /// opened as this runtime's store is, which reads the stage's record in the store's tenant
    /// before every read and write and again after every write lands, and refuses once the stage
    /// is not Begun. Its reads see the store at the base and the run's own commits; its writes go
    /// to the stage.
    /// # Errors
    /// `stage-not-found`, `stage-sealed`, `stage-already-published`, `stage-already-abandoned`,
    /// `stage-incomplete` for a copy without its completion receipt, `stage-unsupported-provider`
    /// on File.
    pub fn join_stage(&self, stage: StageId) -> Result<Self, CommitError> {
        self.on_store()?;
        let record = match &self.backend {
            Backend::Sqlite(kernel) => ekr_store::StageLog::stage_record(&kernel.store, stage)?,
            Backend::Postgres(kernel) => ekr_store::StageLog::stage_record(&kernel.store, stage)?,
            Backend::File(_) => {
                return Err(StoreError::StageUnsupportedProvider(ProviderKind::File).into())
            }
        }
        .ok_or(StoreError::StageNotFound(stage))?;
        if record.state != StageState::Begun {
            return Err(StoreError::StageStateConflict {
                stage_id: stage,
                state: record.state,
            }
            .into());
        }
        let backend = match (&self.backend, &self.location) {
            (Backend::Sqlite(kernel), Location::Sqlite { path, open }) => {
                Backend::Sqlite(Box::new(Commit::over_with_authority(
                    kernel.authority.context,
                    kernel.authority.anchor.clone(),
                    |authority| {
                        match open {
                            SqliteOpen::Existing => {
                                SqliteStore::sqlite_existing(path, &record.tenant, None)
                            }
                            SqliteOpen::Reading => {
                                SqliteStore::sqlite_reading(path, &record.tenant, None)
                            }
                            SqliteOpen::Image => {
                                SqliteStore::sqlite_read_only(path, &record.tenant, None)
                            }
                        }?
                        .joined(&self.tenant, stage)
                        .map(|store| store.under(authority))
                    },
                )?))
            }
            (Backend::Postgres(kernel), Location::Postgres { config, reading }) => {
                Backend::Postgres(Box::new(Commit::over_with_authority(
                    kernel.authority.context,
                    kernel.authority.anchor.clone(),
                    |authority| {
                        PostgresStore::postgres(config, &record.tenant, *reading)?
                            .joined(&self.tenant, stage)
                            .map(|store| store.under(authority))
                    },
                )?))
            }
            _ => return Err(StoreError::StageUnsupportedProvider(self.provider()).into()),
        };
        let joined = Self {
            backend,
            location: self.location.clone(),
            tenant: self.tenant.clone(),
            joined: Some(stage),
        };
        match joined.head() {
            Ok(Some(_)) => Ok(joined),
            Ok(None) => Err(StoreError::StageIncomplete(stage).into()),
            Err(StoreError::Document(code)) if code.starts_with("migrate-incomplete") => {
                Err(StoreError::StageIncomplete(stage).into())
            }
            Err(error) => Err(error.into()),
        }
    }
}
