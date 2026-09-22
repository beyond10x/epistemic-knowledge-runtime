//! The commit path: the one caller a [`ValidatedTransaction`] has ever had.
//!
//! `architecture-decision-record:0007-the-commit-path-is-the-kernels`. AGENTS.md invariant 1 has
//! two halves. "Only `ekr-kernel` constructs a `ValidatedTransaction`" was held by two compile-fail
//! cases from the first day. "Only a `ValidatedTransaction` commits" was held by **nothing**: the
//! independent review of the P1 core landed a commit at revision 1 in a process that cannot link
//! this crate, on three hand-written events through the public `RevisionLog::append`, and found
//! that `ValidatedTransaction` had zero consumers in any `src/` in the workspace.
//!
//! This module is that consumer, and [`Commit::commit`] takes one **by value**.
//!
//! # What holds it, and what does not
//!
//! Not a type, and ADR 0007 says so rather than implying otherwise. `ekr-store` sits below this
//! crate, so its writer cannot take a `ValidatedTransaction`, and a sealed trait declared there
//! would exclude this crate along with everybody else. What holds is:
//!
//! * `ekr-store`'s fold asks a [`CommitAuthority`] before a commit moves canonical state, and a
//!   store opened without one folds no commit at all;
//! * the implementation this crate injects — [`Validations`] — can only be added to from a
//!   `ValidatedTransaction`, because `Validations::record` is `pub(crate)` and takes one;
//! * no crate outside this one declares `ekr-store`, which
//!   `crates/ekr/tests/story_contract.rs` reads off the manifests, and no `src/` outside this one
//!   implements `CommitAuthority`, which the same file reads off the sources.
//!
//! # This commit moves a revision and not the graph
//!
//! Stated here because it is easy to read a green commit as more than it is. No `RevisionEvent`
//! variant carries an operation, so `ekr-store`'s fold applies none, and the knowledge root this
//! path publishes is **the one the folded log already reaches**. A transaction's operations are
//! validated, content-addressed and recorded by address — and they do not change canonical state
//! yet. `story:commit-and-revision-lineage` is what applies them, through the trait this wave
//! added for the fold to learn validation through; `ekr-store`'s `Fold::apply` carries the same
//! statement from the other side.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use ekr_core::{ContentHash, RevisionId, RevisionNumber, TransactionId};
use ekr_graph::{CanonicalGraph, RevisionEvent, Root};
use ekr_store::{
    knowledge_root, Appended, CommitAuthority, ObjectStore, RecordedValidation, RevisionLog,
    StoreError,
};

use crate::seed::{self, BootstrapContext, SeedDocument, SeedError};
use crate::transaction::ValidatedTransaction;

/// The validations this kernel performed, and the only thing `ekr-store`'s fold will commit on.
///
/// A set of [`RecordedValidation`]s, shared with the store the [`Commit`] was opened over — the
/// store holds it as its [`CommitAuthority`] and this crate holds it to add to. **The only way in
/// is `Validations::record`, which is `pub(crate)` and takes a
/// [`ValidatedTransaction`]**, so a member of this set is a transaction some
/// [`Pipeline::validate`](crate::Pipeline::validate) produced.
///
/// # What it is not
///
/// It is not persistence. A process that reopens a lineage it did not commit attests nothing, so
/// its fold reports the seed as the head — the log records the addresses of validations and P1 has
/// no way to re-derive one, because `ekr.kernel.TransactionProposed` carries the operations' address
/// and not the operations. `story:commit-and-revision-lineage` writes the first lineage anybody
/// keeps and is where that is answered; naming the gap is cheaper than a fold that trusts the log
/// again.
#[derive(Clone, Default)]
pub struct Validations {
    transactions: Rc<RefCell<BTreeSet<RecordedValidation>>>,
    bootstrap: Option<BootstrapContext>,
}

impl Validations {
    /// An authority that stands behind nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records that this kernel validated `transaction`, against the revision it names.
    ///
    /// `pub(crate)` and taking a [`ValidatedTransaction`], which together are the whole of
    /// invariant 1 in this file: a crate that cannot build one of those cannot put anything here,
    /// and the fold commits on nothing else.
    pub(crate) fn record(&self, transaction: &ValidatedTransaction) {
        self.transactions.borrow_mut().insert(RecordedValidation {
            transaction_id: transaction.transaction().id,
            against: transaction.validated_against(),
            validation_hash: transaction.validation_hash(),
        });
    }
}

impl CommitAuthority for Validations {
    fn admit_seed(
        &self,
        bytes: &[u8],
        ontology: &ekr_ontology::Ontology,
    ) -> Result<CanonicalGraph, StoreError> {
        seed::replay(
            bytes,
            ontology,
            self.bootstrap.ok_or(StoreError::NoSeedAuthority)?,
        )
    }
    /// Whether this kernel performed exactly the validation the log recorded.
    ///
    /// All three fields, because any two of them leave the third free: the transaction alone would
    /// stand behind a commit at any revision, and the hash alone would stand behind it under any
    /// transaction id.
    fn attests(&self, validation: &RecordedValidation) -> bool {
        self.transactions.borrow().contains(validation)
    }
}

/// Everything the commit path refuses, and what a caller must tell apart.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CommitError {
    /// The store could not answer, or the lineage it holds does not fold.
    #[error(transparent)]
    Store(#[from] StoreError),

    /// There is no lineage to commit into.
    #[error("the lineage has no seed: there is no revision for a commit to follow")]
    NotSeeded,

    /// Design § 72: canonical state moved under the transaction while it was being validated.
    ///
    /// The refusal, and not a `TransactionStale` event. Publishing one is
    /// `story:commit-and-revision-lineage`'s: `ekr.kernel.TransactionStale` carries a transaction
    /// and two revision numbers and nothing else, so a transaction that goes stale twice against
    /// the same pair publishes two byte-identical events and the store's idempotency key reads the
    /// second as a retry — `task:two-revision-events-have-no-discriminator`, which blocks that
    /// story. A write known to be lossy is worse than a refusal the caller can act on.
    #[error("transaction {transaction} was validated against revision {validated_against} and the lineage is at {head}")]
    Stale {
        /// The transaction that did not commit.
        transaction: TransactionId,
        /// The revision it was validated against.
        validated_against: RevisionNumber,
        /// The revision the lineage is at now.
        head: RevisionNumber,
    },

    /// The lineage has no next revision number.
    #[error("the lineage is at {at}, which has no successor")]
    LineageExhausted {
        /// The revision it is at.
        at: RevisionNumber,
    },

    /// An event this commit wrote was already in the log, so the commit is a repeat of one that
    /// already happened rather than a new fact.
    ///
    /// Acted on rather than discarded, which is why `RevisionLog::append` answers
    /// [`Appended`] instead of `()`: an `Ok` a caller cannot tell from a write is lossy exactly
    /// when the caller did not intend a retry, and a commit never does.
    #[error("{event} was already on record: this commit repeats one the lineage already holds")]
    AlreadyRecorded {
        /// The event that was recognised rather than written.
        event: &'static str,
    },
}

/// The commit path over one store.
///
/// Generic over the store, so the properties its cases prove are proved against one body of code:
/// the two `ekr-store` providers are two instantiations, as they are there.
pub struct Commit<S: RevisionLog + ObjectStore> {
    store: S,
    validations: Validations,
}

impl<S: RevisionLog + ObjectStore> Commit<S> {
    /// The commit path over a store opened **under this path's own validations**.
    ///
    /// The store is built by the caller and the authority is not, which is the coupling stated as
    /// a signature: a `Commit` cannot be handed a store that answers to somebody else's
    /// validations, because the only [`Validations`] it holds is the one it passed in.
    ///
    /// ```
    /// # use ekr_kernel::Commit;
    /// # use ekr_store::{FileStore, StoreError};
    /// # fn open(path: &std::path::Path, ontology: ekr_ontology::Ontology)
    /// #     -> Result<Commit<FileStore>, StoreError> {
    /// Commit::over(|validations| {
    ///     Ok(FileStore::file(path, "ekr", ontology)?.under(validations))
    /// })
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// Whatever `open` returned.
    pub fn over<E>(open: impl FnOnce(Validations) -> Result<S, E>) -> Result<Self, E> {
        let validations = Validations::new();
        Ok(Self {
            store: open(validations.clone())?,
            validations,
        })
    }

    /// Opens a runtime with independently supplied bootstrap attribution.
    /// The same context must be supplied when reopening that seed.
    ///
    /// # Errors
    /// Whatever the provider constructor returns.
    pub fn over_with_bootstrap<E>(
        context: BootstrapContext,
        open: impl FnOnce(Validations) -> Result<S, E>,
    ) -> Result<Self, E> {
        let validations = Validations {
            bootstrap: Some(context),
            ..Validations::default()
        };
        Ok(Self {
            store: open(validations.clone())?,
            validations,
        })
    }

    /// Reads the admitted lineage head.
    ///
    /// # Errors
    /// A provider or replay admission refusal.
    pub fn head(&self) -> Result<Option<Root>, StoreError> {
        self.store.head()
    }

    /// Reads the admitted graph at the current revision.
    ///
    /// # Errors
    /// A provider or replay admission refusal.
    pub fn snapshot(&self) -> Result<CanonicalGraph, StoreError> {
        self.store.fold()
    }

    /// Replays from a materialised revision.
    ///
    /// # Errors
    /// A provider or replay admission refusal.
    pub fn replay(&self, from: RevisionNumber) -> Result<CanonicalGraph, StoreError> {
        self.store.replay(from)
    }

    /// Resolves retained content, including bootstrap statement payloads.
    ///
    /// # Errors
    /// A provider or replay admission refusal. Seed payloads are revalidated before lookup.
    pub fn content(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        if let Some(bytes) = self.store.seed_bytes()? {
            self.store.fold()?;
            if let Some(payload) = seed::content(&bytes, hash)? {
                return Ok(Some(payload));
            }
        }
        self.store.get(hash)
    }

    /// Commits `validated`, and answers the revision the lineage reached.
    ///
    /// Design § 19 and § 72, and AGENTS.md invariant 1. Takes the transaction **by value**: a
    /// `ValidatedTransaction` is spent by committing it, and a caller holding one after the fact
    /// could otherwise present the same validation at a later revision, which is exactly what § 72
    /// calls stale.
    ///
    /// Three events, in the order `systems/ekr/domains/kernel.yaml` declares them and the store's
    /// fold requires: the proposal with the operations' address, the validation with its own, and
    /// the commit with the knowledge root the fold reaches. See this module's header for why that
    /// last one is the *unchanged* root in P1.
    ///
    /// # Errors
    ///
    /// [`CommitError::NotSeeded`] when there is no lineage, [`CommitError::Stale`] when canonical
    /// state moved under the transaction (§ 72), [`CommitError::LineageExhausted`] at the end of
    /// the revision numbers, [`CommitError::AlreadyRecorded`] when an event this commit wrote was
    /// already on record, and [`CommitError::Store`] for whatever the store refused.
    pub fn commit(&self, validated: ValidatedTransaction) -> Result<Root, CommitError> {
        let head = self.store.head()?.ok_or(CommitError::NotSeeded)?;
        let transaction = validated.transaction();

        // § 72, before anything is written: "a transaction is committed only against the revision
        // it was validated against". The store's fold repeats this check when it replays the log,
        // which is the difference between a rule enforced at write time and a rule a reader can
        // repeat.
        if validated.validated_against() != head.revision {
            return Err(CommitError::Stale {
                transaction: transaction.id,
                validated_against: validated.validated_against(),
                head: head.revision,
            });
        }
        let number = head
            .revision
            .next()
            .ok_or(CommitError::LineageExhausted { at: head.revision })?;

        let events = [
            RevisionEvent::TransactionProposed {
                transaction_id: transaction.id,
                proposer: transaction.proposer,
                operations_hash: ContentHash::of(&transaction.operations),
            },
            RevisionEvent::TransactionValidated {
                transaction_id: transaction.id,
                against: validated.validated_against(),
                validation_hash: validated.validation_hash(),
            },
            RevisionEvent::RevisionCommitted {
                transaction_id: transaction.id,
                revision_id: RevisionId::mint(),
                number,
                knowledge_root: knowledge_root(&self.store.fold()?),
            },
        ];

        // Recorded before the events are written, because the fold reads the log and the log is
        // what a reader has: an event on record that this authority does not stand behind is a
        // commit that does not move canonical state, and the window where that is true is not one
        // to leave open across three appends.
        self.validations.record(&validated);

        for event in &events {
            if self.store.append(event)? == Appended::AlreadyRecorded {
                return Err(CommitError::AlreadyRecorded {
                    event: event.name(),
                });
            }
        }

        self.store.head()?.ok_or(CommitError::NotSeeded)
    }
}

impl<S: RevisionLog + ObjectStore + ekr_store::Initialize> Commit<S> {
    /// Validates and atomically publishes revision zero. There is no preceding revision.
    ///
    /// # Errors
    /// Invalid bootstrap input, missing authority, existing lineage or provider failure.
    pub fn seed(&self, document: SeedDocument) -> Result<Root, SeedError> {
        let context = self
            .validations
            .bootstrap
            .ok_or(StoreError::NoSeedAuthority)?;
        let at = document.graph.root.created_at;
        let validated = seed::validate(document, context)?;
        self.store.initialize(&validated.bytes(), at)?;
        self.store.head()?.ok_or(StoreError::NotSeeded.into())
    }
}
