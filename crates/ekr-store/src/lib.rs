//! The persistence layer of the Epistemic Knowledge Runtime.
//!
//! Implements the `ekr.store` domain, `systems/ekr/domains/store.yaml`: content-addressed
//! objects by storage class and snapshots of committed revisions, held in eventlog.
//!
//! Public modules:
//!
//! * [`log`] — [`RevisionLog`], the lineage: publish an occurrence with its objects atomically,
//!   fold the log, read the head [`Root`](ekr_graph::Root), replay to a revision. Replay admits
//!   history only through the injected [`CommitAuthority`], and [`knowledge_root`] and
//!   [`evidence_root`] are computed there.
//! * [`objects`] — [`StoredObject`] and [`StorageClass`], the content-addressed object store of
//!   design § 37 and § 57.
//! * [`snapshot`] — [`GraphDocument`], the materialised fold as bytes, and the one named place a
//!   document is serialized; kernel admission is delegated through the authority port.
//! * [`stage`] — a stage of a store (design § 107): [`stage_tenant`] derives its tenant from the
//!   store's tenant and the stage id, [`admit_store_tenant`] refuses a store's tenant that carries
//!   the marker reserved to stage tenants, and [`StageLog`] keeps each stage's record in the
//!   store's tenant, elects and appends its publication in `ekr.publication-preparation/4`, and
//!   forgets a Published or Abandoned stage's tenant.
//! * [`eventlog`] — the implementation over `eventlog-sqlite` and `eventlog-file`.
//! * [`legacy`] — supplied-byte verification of the original graph format.
//!
//! Publication preparations (design § 94, `architecture-decision-record:0009`) are private: they
//! retain an elected native request across an uncertain outcome and never confer canonical
//! authority.
//!
//! # Synchronous, over an async port
//!
//! `architecture-decision-record:0006-ekr-store-bridges-the-async-port`. `eventlog-core`'s
//! `EventStore` is async and needs a tokio runtime context; this crate owns one and exposes
//! nothing async, so `ekr-kernel`, the CLI and everything P1 builds later stay synchronous. The
//! cost is `task:ekr-store-block-on-cannot-nest`, and it is recorded rather than hidden.
//!
//! # No writer to canonical state
//!
//! AGENTS.md invariant 1: only `ekr-kernel` constructs a `ValidatedTransaction` and only one may
//! commit. Nothing here builds an event or decides that a transaction is valid. What this crate
//! does is *hold the kernel to its own record*: the fold refuses a commit the log never validated,
//! a revision that does not follow its parent, and a knowledge root the folded state does not
//! reach. A lineage is checked by replaying it, which is what `docs/roadmap.md` § 4 means by
//! "replay from the seed reproduces the root hash".
//!
//! **An event in the log is not that record**, which is
//! `architecture-decision-record:0007-the-commit-path-is-the-kernels`: the public `append` that
//! took a bare event made a commit reachable by anyone holding a store, and it is gone. Every
//! publication and every replay now asks a [`CommitAuthority`] injected at construction — a store
//! opened without one folds no commit — and `ekr-kernel` is the only crate in the workspace that
//! declares this one.
//!
//! # Where the membrane stops
//!
//! `task:the-membrane-stops-at-the-store-boundary`, decided in [`snapshot`]: the store never
//! deserialises straight into a canonical type. Read that module before writing a second
//! deserialiser here.

pub mod eventlog;
pub mod log;
pub mod objects;
pub mod postgres;
pub mod snapshot;
pub mod stage;
mod verified;

/// A stage's identity, `ekr.store.StageId`, minted in `ekr-core`'s `Id::mint()` family.
pub use ekr_core::StageId;
pub use eventlog::remove_read_only_copies;
pub use eventlog::{
    EventlogStore, FileStore, InventoriedObject, Inventory, PostgresStore, PublishedEvent,
    SqliteStore, StoreInventory,
};
pub use eventlog::{
    NativeBlobWrite, NativeClaim, NativeCommandMeta, NativeExpected, NativeExpectedKind,
    NativeNewEvent, NativePublicationRequest, NativeStreamAppend, NativeStreamId,
    PublicationCommandKey, PublicationCommandKind, PublicationPreparationV1,
    StagedPublicationObject,
};
pub use eventlog::{
    PublicationPreparationV4, StagePublication, StagePublicationCommandKey, StagePublicationObject,
};
#[doc(hidden)]
pub use log::knowledge_roots_hashed;
pub use log::{
    evidence_root, knowledge_root, AdmittedRevision, Appended, CommitAuthority, Initialize,
    Publication, PublicationObject, RecordedOccurrence, RetainedHistory, RetainedObject,
    RevisionLog,
};
pub use objects::{ObjectStore, StorageClass, StoredObject};
pub use snapshot::{Entity, GraphDocument, MembraneError};
pub use stage::{
    admit_store_tenant, stage_tenant, ProviderKind, Stage, StageLog, StagePublishedRecord,
    StageRecord, StageResult, StageState, Store, StoreTenant, STAGE_TENANT_MARKER,
};
#[doc(hidden)]
pub use stage::{on_stage_point, StageHookGuard, StagePoint};
#[doc(hidden)]
pub use verified::{objects_loaded, read_work, stream_reads, ReadWork, StreamReads};

use ekr_core::{ContentHash, EventId, RevisionNumber, TransactionId};

/// Everything a store refuses, and what a caller must tell apart.
///
/// Most of these are refusals of a **lineage**, not of a call: the log accepts what it is given and
/// the fold is what holds it to design § 34, so a defect written at one moment surfaces the next
/// time anything reads. Each names what it found, because a caller that cannot tell a missing seed
/// from a broken chain cannot act on either.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    /// A pending logical command was elected for different actual input.
    #[error("a publication preparation exists for different input")]
    PublicationInputConflict,
    /// Conditional publication lost a stream race; no new occurrence was published.
    #[error("revision stream moved before publication")]
    Conflict,
    /// Publication may have committed. Resolve the same immutable occurrence; never delete it.
    #[error("publication outcome is unknown; resolve the exact retained occurrence")]
    UnknownCommit,
    /// Retained bootstrap context or authority differs from the independently supplied anchor.
    #[error("bootstrap-authority-mismatch")]
    AuthorityMismatch,
    /// Synchronous persistence cannot run on a thread entered into a Tokio runtime.
    #[error("synchronous store access requires a thread outside a Tokio runtime")]
    RuntimeContext,
    /// Bootstrap admission needs a real authority, including on reopen.
    #[error("no seed authority was configured")]
    NoSeedAuthority,
    /// Bootstrap input failed kernel validation.
    #[error("invalid seed: {0}")]
    InvalidSeed(String),
    /// This seed predates the complete replayable envelope and needs an explicit migration.
    #[error(
        "legacy seed requires migration: full ontology and bootstrap attribution are unavailable"
    )]
    SeedMigrationRequired,
    /// Initialization may only create a lineage once.
    #[error("the lineage is already seeded")]
    AlreadySeeded,
    /// The provider could not answer.
    #[error("the store is unavailable: {0}")]
    Backend(String),

    /// The provider refused this handle's history: the history at the path is no longer the one
    /// the handle observed — a store replaced under the same device and inode. Nothing is
    /// answered from what the handle observed; a store opened at the path again reads what is
    /// there. It carries the provider's own report, and reads as [`StoreError::Backend`] does.
    #[error("the store is unavailable: {0}")]
    Diverged(String),

    /// `store-replaced`: the SQLite database file at this handle's path is no longer the one it
    /// opened — another file is there, or the file there holds a log whose first event, or the
    /// newest event the handle saw in it, is not the handle's (a database copied over it in
    /// place, for one). Nothing is answered or written through the handle; a store opened at the
    /// path again reads what is there, or is refused the same way. A long-running reader opens
    /// the store again on it, as on [`StoreError::Diverged`].
    #[error("store-replaced: {0}")]
    Replaced(String),

    /// `stage-tenant-reserved` (`ekr.store.StageTenantReserved`): a store was to be opened under
    /// a tenant that carries the marker reserved to stage tenants ([`STAGE_TENANT_MARKER`]), so
    /// it could be a stage's derived tenant. No store was opened; nothing was read, created or
    /// written. It carries the refused tenant.
    #[error(
        "stage-tenant-reserved: the tenant {0:?} carries the marker reserved to stage tenants \
         ({STAGE_TENANT_MARKER}); no store is opened under it"
    )]
    StageTenantReserved(String),

    /// `stage-not-found` (`ekr.store.StageNotFound`): no stage carries the id in this store.
    /// Nothing was read or written through a stage.
    #[error("stage-not-found: no stage {0} in this store")]
    StageNotFound(StageId),

    /// `ekr.store.StageStateConflict`: the stage is not in a state this acts from, named by its
    /// state ([`StageState::refusal`]): `stage-not-sealed`, `stage-sealed`,
    /// `stage-already-published` or `stage-already-abandoned`. Nothing was written.
    #[error("{}: stage {stage_id} is {state:?}", state.refusal())]
    StageStateConflict {
        /// The stage.
        stage_id: StageId,
        /// Its state.
        state: StageState,
    },

    /// `stage-write-landed` (`ekr.store.StageWriteLanded`): a write joined to the stage passed
    /// its check while the stage was Begun and landed after the stage left Begun. It is refused,
    /// and not reported successful; `event_ids` are the revision-stream occurrences it landed,
    /// which the store holds exactly when the stage's publication holds them.
    #[error(
        "stage-write-landed: a write joined to stage {stage_id} landed after it became \
         {state:?}; occurrences {event_ids:?} are in the store only if its publication holds them"
    )]
    StageWriteLanded {
        /// The stage.
        stage_id: StageId,
        /// Its state when the write's record read found it.
        state: StageState,
        /// The revision-stream occurrences the write landed.
        event_ids: Vec<EventId>,
    },

    /// `stage-head-moved` (`ekr.store.StageHeadMoved`): the store's head, its last committed
    /// revision, is not the expected head or not the stage's base. Nothing was appended.
    #[error(
        "stage-head-moved: stage {stage_id} expects head {expected} at base {base}, and the \
         store's head is {current}"
    )]
    StageHeadMoved {
        /// The stage.
        stage_id: StageId,
        /// The head the caller expected.
        expected: RevisionNumber,
        /// The stage's base.
        base: RevisionNumber,
        /// The store's head when this was decided.
        current: RevisionNumber,
    },

    /// `stage-stream-moved` (`ekr.store.StageStreamMoved`): the store's revision stream moved
    /// after the capture and its head did not; nothing was appended and the stage stays Sealing.
    #[error(
        "stage-stream-moved: stage {stage_id} captured the revision stream at {captured} and it \
         is at {current}; publish again"
    )]
    StageStreamMoved {
        /// The stage.
        stage_id: StageId,
        /// The version the group expected.
        captured: u64,
        /// The version found.
        current: u64,
    },

    /// `stage-object-moved` (`ekr.store.StageObjectMoved`): an object the publication appends
    /// moved in the store after the capture; nothing was appended and the stage stays Sealing.
    #[error(
        "stage-object-moved: stage {stage_id} found object {content_hash} moved; publish again"
    )]
    StageObjectMoved {
        /// The stage.
        stage_id: StageId,
        /// The object.
        content_hash: ContentHash,
    },

    /// `stage-incomplete` (`ekr.store.StageIncomplete`): the stage's copy holds no completion
    /// receipt; it can only be abandoned.
    #[error("stage-incomplete: the copy of stage {0} never finished; abandon it")]
    StageIncomplete(StageId),

    /// `unresolved-preparation` (`ekr.store.UnresolvedPreparation`): a decision was elected and
    /// never published. Nothing was copied, sealed or appended.
    #[error(
        "unresolved-preparation: occurrence {0} was elected and never published; resolve it with \
         the command that elected it"
    )]
    UnresolvedPreparation(EventId),

    /// `stage-suffix-refused` (`ekr.store.StageSuffixRefused`): the stage's suffix did not
    /// validate against the store; nothing was appended and the stage stays Sealing.
    #[error("stage-suffix-refused: stage {stage_id}: {code}: {reason}")]
    StageSuffixRefused {
        /// The stage.
        stage_id: StageId,
        /// The refusal's code.
        code: String,
        /// What was refused.
        reason: String,
    },

    /// `stage-unsupported-provider` (`ekr.store.StageUnsupportedProvider`): the provider admits
    /// no stage. Nothing was recorded or copied.
    #[error("stage-unsupported-provider: the {} provider admits no stage", .0.name())]
    StageUnsupportedProvider(ProviderKind),

    /// An existing-only open found no store at the path: nothing there, an empty directory, an
    /// empty file, a symlink to nothing, a SQLite database without the owner tables, or a File
    /// directory holding only what the provider writes before its manifest. Nothing was created.
    #[error("no store at {0}")]
    NoStore(String),

    /// This process may not write the store: a writing open found a path of it this process
    /// cannot write, or a write reached a store opened read-only. Nothing was written. A reader
    /// opens such a store with `file_reading` or `sqlite_reading` instead.
    #[error("the store is read-only to this process: {0}")]
    ReadOnly(String),

    /// A record could not be read as what it should be — or could not be written as one.
    #[error("a stored document could not be read: {0}")]
    Document(String),

    /// A document did not cross into canonical state.
    #[error("a stored document is not canonical state: {0}")]
    Membrane(#[from] MembraneError),

    /// The lineage does not begin at `ekr.kernel.Seeded`.
    #[error("the lineage has no seed: a fold has no state to begin from")]
    NotSeeded,

    /// A seed arrived after the lineage had already begun.
    #[error("a second seed would restart a lineage that is already running")]
    SeedIsNotFirst,

    /// A transaction was validated or committed without ever having been proposed.
    #[error("transaction {transaction_id} was never proposed")]
    ProposalMissing {
        /// The transaction.
        transaction_id: TransactionId,
    },

    /// A transaction committed without a validation standing for it — never validated, or
    /// rejected or gone stale since.
    #[error("transaction {transaction_id} committed without a standing validation")]
    ValidationMissing {
        /// The transaction.
        transaction_id: TransactionId,
    },

    /// A commit claimed a revision that is not the next one in the lineage.
    #[error("the lineage expected revision {expected} next and a commit claimed {found}")]
    RevisionOutOfOrder {
        /// The revision the lineage was ready for.
        expected: RevisionNumber,
        /// The revision the commit claimed.
        found: RevisionNumber,
    },

    /// A commit published a knowledge root the folded state does not reach.
    ///
    /// The P1 exit criterion is that replay *reproduces* the root hash. The published address is
    /// the kernel's claim and the folded one is what the log actually reaches; they agree or the
    /// lineage is not reproducible, and a fold that copied the claim through would report a
    /// reproduction it had not performed.
    #[error(
        "revision {revision} published knowledge root {published}, and the fold reaches {folded}"
    )]
    KnowledgeRootDisagrees {
        /// The revision that published it.
        revision: RevisionNumber,
        /// What the commit event claimed.
        published: ContentHash,
        /// What folding the log actually reaches.
        folded: ContentHash,
    },

    /// A replay was asked to begin at a revision this store holds no state at.
    ///
    /// P1 materialises state at the seed and nowhere else. Answering anyway would mean folding
    /// from a beginning that was never written down, which is a different lineage wearing the
    /// requested one's number.
    #[error(
        "no materialised state at revision {requested}: P1 materialises the seed and nothing after it"
    )]
    NoMaterialisedState {
        /// The revision the caller asked to begin at.
        requested: RevisionNumber,
    },
}

impl From<eventlog_core::EventLogError> for StoreError {
    /// Every provider failure is a backend failure to a caller of this crate.
    ///
    /// Flattened deliberately: the port's own taxonomy — invalid, conflict, idempotency mismatch,
    /// guard refused — is about the log's contract, and a consumer of a knowledge runtime cannot
    /// act differently on any of them. The one distinction this crate *does* act on is made where
    /// it matters, inside [`ObjectStore::put`], and is not re-exported as a shape callers would
    /// have to match on. A long-running reader does act on a diverged history, so the provider's
    /// refusal of one is [`StoreError::Diverged`], told once, in `eventlog`.
    fn from(error: eventlog_core::EventLogError) -> Self {
        match error {
            eventlog_core::EventLogError::UnknownCommit => Self::UnknownCommit,
            eventlog_core::EventLogError::Conflict { .. } => Self::Conflict,
            diverged if eventlog::diverged(&diverged) => Self::Diverged(diverged.to_string()),
            other => Self::Backend(other.to_string()),
        }
    }
}
/// Frozen original-format data and supplied-byte verification.
pub mod legacy;
