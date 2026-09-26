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
pub mod snapshot;

pub use eventlog::{EventlogStore, FileStore, PublishedEvent, SqliteStore};
pub use eventlog::{
    NativeBlobWrite, NativeClaim, NativeCommandMeta, NativeExpected, NativeExpectedKind,
    NativeNewEvent, NativePublicationRequest, NativeStreamAppend, NativeStreamId,
    PublicationCommandKey, PublicationCommandKind, PublicationPreparationV1,
};
pub use log::{
    evidence_root, knowledge_root, AdmittedRevision, Appended, CommitAuthority, Initialize,
    Publication, PublicationObject, RecordedOccurrence, RetainedHistory, RetainedObject,
    RevisionLog,
};
pub use objects::{ObjectStore, StorageClass, StoredObject};
pub use snapshot::{Entity, GraphDocument, MembraneError};

use ekr_core::{ContentHash, RevisionNumber, TransactionId};

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

    /// An existing-only open found no store at the path: nothing there, an empty directory, an
    /// empty file, a symlink to nothing, a SQLite database without the owner tables, or a File
    /// directory holding only what the provider writes before its manifest. Nothing was created.
    #[error("no store at {0}")]
    NoStore(String),

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
    /// have to match on.
    fn from(error: eventlog_core::EventLogError) -> Self {
        match error {
            eventlog_core::EventLogError::UnknownCommit => Self::UnknownCommit,
            eventlog_core::EventLogError::Conflict { .. } => Self::Conflict,
            other => Self::Backend(other.to_string()),
        }
    }
}
/// Frozen original-format data and supplied-byte verification.
pub mod legacy;
