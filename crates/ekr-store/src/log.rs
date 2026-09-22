//! The revision log: what may be appended to a lineage, and what folding one produces.
//!
//! Design § 34. A committed revision is an event, the canonical graph is a fold over the log, and
//! the fold is a *verification* rather than a copy — `docs/roadmap.md` § 4's P1 exit criterion is
//! that "replay from the seed reproduces the root hash", and reproducing means computing.

use std::collections::BTreeMap;

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{AssertionId, ContentHash, EdgeId, NodeId, RevisionNumber, TransactionId};
use ekr_graph::{Assertion, CanonicalGraph, CanonicalValue, Edge, Node, RevisionEvent, Root};

use crate::StoreError;

/// The address P1 writes where a sub-root has no type to hash.
///
/// `task:two-of-the-five-revision-sub-roots-are-placeholders`. [`Root`] carries five content
/// addresses and three of them are computable as of wave p1-05: graph state, retained evidence and
/// the transaction. Two are not, and will not be in P1:
///
/// * **`ontology_root`** — `ekr_ontology::Ontology` has no `Canonical` implementation. It cannot be
///   written here either: `Canonical` belongs to `ekr-core` and `Ontology` to `ekr-ontology`, so
///   the orphan rule puts that implementation in `ekr-ontology`.
/// * **`agent_root`** — design § 18 gives an `Agent` and no crate declares one.
///
/// Thirty-two zero bytes, which is not the address of anything: every real address in this runtime
/// is a SHA-256 over a domain label and some bytes. A placeholder that *looked* like a hash is the
/// kind of thing a later wave builds on without noticing, so this one does not.
/// `crates/ekr-store/tests/fold_rules.rs` asserts that both fields are this value rather than
/// something derived, so the day either becomes real is a day a case changes.
pub const PLACEHOLDER_SUB_ROOT: ContentHash = ContentHash::from_bytes([0u8; 32]);

/// The address of a graph's knowledge state: `Root.knowledge_root`, design § 34.
///
/// Nodes, edges and assertions, in that order, each as its own map. Not the whole
/// [`CanonicalGraph`]: § 34 gives ontology, knowledge, evidence and agent state different
/// sub-roots precisely so that a retention sweep of evidence does not look like a change to
/// knowledge, and a single hash over all of them would undo that.
///
/// The order is the contract. Moving a field here moves every revision address ever recorded.
#[must_use]
pub fn knowledge_root(graph: &CanonicalGraph) -> ContentHash {
    ContentHash::of(&KnowledgeState {
        nodes: &graph.nodes,
        edges: &graph.edges,
        assertions: &graph.assertions,
    })
}

/// The address of a graph's retained evidence: `Root.evidence_root`, design § 34.
#[must_use]
pub fn evidence_root(graph: &CanonicalGraph) -> ContentHash {
    ContentHash::of(&graph.evidence)
}

/// Graph state as one canonical value, so that [`knowledge_root`] is one address over three maps.
struct KnowledgeState<'a> {
    nodes: &'a BTreeMap<NodeId, Node<CanonicalValue>>,
    edges: &'a BTreeMap<EdgeId, Edge<CanonicalValue>>,
    assertions: &'a BTreeMap<AssertionId, Assertion<CanonicalValue>>,
}

impl Canonical for KnowledgeState<'_> {
    /// The three maps in declaration order, structural and untagged: rule 5 of
    /// `ekr_core::canonical` — a composite value's own field structure is what distinguishes it.
    fn encode(&self, out: &mut Encoder) {
        self.nodes.encode(out);
        self.edges.encode(out);
        self.assertions.encode(out);
    }
}

/// What a `ekr.kernel.TransactionValidated` event says: a **claim** that a transaction was
/// validated, until something stands behind it.
///
/// The three fields of the event, and nothing derived. The fold holds one of these per validated
/// transaction and asks its [`CommitAuthority`] about it when the commit arrives.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RecordedValidation {
    /// The transaction the event named.
    pub transaction_id: TransactionId,
    /// The revision it says the transaction was validated against — design § 71, and what makes
    /// § 72's stale commit detectable.
    pub against: RevisionNumber,
    /// The address it gave the validation result.
    pub validation_hash: ContentHash,
}

/// Whatever the fold asks before it lets a commit move canonical state.
///
/// `architecture-decision-record:0007-the-commit-path-is-the-kernels`. AGENTS.md invariant 1 says
/// only a `ValidatedTransaction` commits, and until ADR 0007 nothing carried the second half:
/// [`RevisionLog::append`] is public and takes a bare [`RevisionEvent`], and the fold treated an
/// appended `TransactionValidated` as proof — *by anyone, carrying any hash*. The independent
/// review of the P1 core measured a commit landing at revision 1 in a process that cannot link
/// `ekr-kernel`.
///
/// # Why this is a trait here rather than a type
///
/// `ekr-store` sits **below** `ekr-kernel` in the workspace order (`docs/roadmap.md` § 3), so this
/// crate cannot name `ValidatedTransaction` and [`append`](RevisionLog::append) cannot take one.
/// Sealing does not help either: a sealed trait declared here is unimplementable *outside* here,
/// which excludes `ekr-kernel` along with everybody else, and Rust has no way to say *only that
/// other crate*.
///
/// So the guarantee is stated where it can actually hold, and ADR 0007 says so rather than
/// implying more: **no consumer of this runtime can reach a writer to canonical state without a
/// `ValidatedTransaction`** — `crates/ekr` declares no `ekr-store` dependency, `ekr-kernel` is the
/// only crate that does, and the implementation of this trait it injects is one only a
/// `ValidatedTransaction` can add to. Within the workspace that is the dependency graph and two
/// cases that read it (`crates/ekr/tests/story_contract.rs`), not a type.
///
/// # And a commit it refuses is not an error
///
/// The fold does not apply a commit this authority declines, and **reports no failure**: the head
/// stays where it was. That is deliberate and is the difference between a refusal of a *lineage*
/// and a refusal of a *claim*. A log is append-only and its writer is not this crate; making an
/// unattested commit poison the fold would let one append permanently brick every read of the
/// lineage, which is a worse answer than the true one — that the commit did not move canonical
/// state. `StoreError::ValidationMissing` stays for the lineage that is genuinely malformed: a
/// commit with no validation event at all behind it, or one that was rejected.
///
/// A store opened without an authority therefore folds **no** commit, and
/// [`RevisionLog::fold`] says so with [`StoreError::NoCommitAuthority`] rather than answering a
/// state it has no basis for. After ADR 0007 the only way to a store is through `ekr-kernel`, which
/// injects one, and this crate's own suite injects a stub whose doc says exactly which validations
/// it stands behind.
///
/// # It is a port, and `systems/ekr/domains/store.yaml` does not declare it
///
/// This trait, [`RecordedValidation`] and `EventlogStore::under` are public surface carrying half
/// of an `AGENTS.md` invariant, and they appear in no domain entry and in no story: the ESS domain
/// models entities and events, and a port a neighbouring crate implements is neither. Recorded
/// rather than left for a reader to notice, and raised by the adversary of wave p1-06; the
/// coordinator holds the question of whether `systems/` should grow a way to say it, because
/// `systems/` is not this crate's to edit.
pub trait CommitAuthority {
    /// Revalidates persisted bootstrap input against the complete configured ontology.
    /// Authorities which only attest transaction claims cannot admit a seed.
    ///
    /// # Errors
    /// A missing bootstrap authority or a refused seed.
    fn admit_seed(
        &self,
        _bytes: &[u8],
        _ontology: &ekr_ontology::Ontology,
    ) -> Result<CanonicalGraph, StoreError> {
        Err(StoreError::NoSeedAuthority)
    }
    /// Whether this authority stands behind `validation`.
    ///
    /// It is asked once per commit, about the claim the log recorded, and its answer decides
    /// whether the lineage advances under that transaction.
    fn attests(&self, validation: &RecordedValidation) -> bool;
}

/// What an append did: wrote the event, or recognised a request already on record.
///
/// Returned rather than discarded because the two are not the same thing and the store is the only
/// layer that can tell them apart. See [`RevisionLog::append`] for when the second happens without
/// the caller having retried anything.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[must_use = "an append that recognised a request wrote nothing, and the caller is what decides \
              whether that is a retry or a lost fact"]
pub enum Appended {
    /// The event is in the log because this call put it there.
    Written,
    /// A request with this exact content was already on record, so nothing was written and the
    /// earlier result stands.
    AlreadyRecorded,
}

/// The revision lineage a store holds: append one event, fold the lot.
///
/// **Synchronous**, and that is a decision rather than an omission:
/// `architecture-decision-record:0006-ekr-store-bridges-the-async-port`. The `eventlog-core` port
/// underneath is async — every method returns a `BoxFuture` — and the runtime that drives it lives
/// inside the implementation, so `ekr-kernel`, the CLI and everything P1 builds later stay
/// synchronous. The cost is recorded rather than hidden:
/// `task:ekr-store-block-on-cannot-nest` says what happens when a caller is already inside a
/// runtime, and what would close it.
///
/// # Appending is not validating
///
/// [`append`](RevisionLog::append) writes what it is given. Every rule about which event may
/// follow which is in the **fold**, and a log holding a sequence no fold accepts is a log that
/// reports an error when it is read rather than one that silently answers. That is deliberate: a
/// lineage is checked by replaying it, and a check that only ran at write time is a check no
/// reader can repeat.
pub trait RevisionLog {
    /// The verified bytes named by the first Seeded event, if any.
    ///
    /// # Errors
    /// Malformed lineage or missing/corrupt retained seed object.
    fn seed_bytes(&self) -> Result<Option<Vec<u8>>, StoreError>;
    /// Appends one event to the lineage, and says which of the two things happened.
    ///
    /// # An `Ok` is not a write
    ///
    /// The idempotency key is the event's own content, so an event byte-identical to one already in
    /// the log is read as a **retry** and nothing is written. That is what makes an interrupted
    /// append safe to repeat, and it is why this returns [`Appended`] rather than `()`: an `Ok` the
    /// caller cannot tell apart from a write is the lossy part, and it is lossy exactly when the
    /// caller did not intend a retry.
    ///
    /// The adversary of wave p1-05 measured the shape that makes it reachable.
    /// `ekr_graph::RevisionEvent::TransactionRejected` carries a transaction and an issue count and
    /// nothing else, so a transaction rejected, proposed again and rejected again for the same
    /// number of issues publishes **two byte-identical events** — two different facts about the
    /// world with one encoding. The second is answered [`Appended::AlreadyRecorded`], the log holds
    /// one, and a fold reading that log cannot see the refusal that was appended to it.
    /// `TransactionStale` has the same shape.
    ///
    /// That is a defect in the *vocabulary* and not in this key: a key over content plus position
    /// would write both, and would also write a genuine retry twice, which is the more dangerous
    /// half and was this crate's defect one round earlier. Giving the two variants something to
    /// tell them apart is a change to `ekr_graph::RevisionEvent` and
    /// `systems/ekr/domains/kernel.yaml` together, filed as
    /// `task:two-revision-events-have-no-discriminator` and blocking
    /// `story:commit-and-revision-lineage`, which writes the first code that can produce that
    /// lineage.
    ///
    /// Until then this method reports, and the caller decides. Nothing in P1 calls it; `ekr-kernel`
    /// does in wave p1-06, and an [`Appended::AlreadyRecorded`] it did not expect is a signal it can
    /// act on rather than a silence it cannot.
    ///
    /// # Errors
    ///
    /// [`StoreError::Backend`] when the provider is unavailable, and [`StoreError::Document`] when
    /// the event cannot be serialised.
    fn append(&self, event: &RevisionEvent) -> Result<Appended, StoreError>;

    /// The canonical graph the whole log folds to.
    ///
    /// # Errors
    ///
    /// [`StoreError::NotSeeded`] when the lineage does not begin at a seed, and whichever refusal
    /// of [`StoreError`] the sequence of events earns.
    fn fold(&self) -> Result<CanonicalGraph, StoreError>;

    /// The root of the lineage's latest revision, or `None` when the log is empty.
    ///
    /// `None` means **the log holds no events at all**, and it is an answer rather than an error: a
    /// store that has never been seeded has no head, which is a different thing from a store whose
    /// lineage does not fold. A seeded lineage with no commits yet has a head — the seed's own
    /// `Root`, at [`RevisionNumber::SEED`] — because design § 34's lineage is `Seed -> Root₀ ->
    /// Root₁` and `Root₀` exists as soon as the seed does.
    ///
    /// # Errors
    ///
    /// Whichever refusal of [`StoreError`] the sequence of events earns.
    fn head(&self) -> Result<Option<Root>, StoreError>;

    /// The fold of the lineage from `from` forward.
    ///
    /// P1 materialises state at the **seed** and nowhere else — no revision after it has a stored
    /// graph to begin from — so `from` beyond [`RevisionNumber::SEED`] is refused with
    /// [`StoreError::NoMaterialisedState`] rather than answered with a fold that silently began
    /// somewhere it had no state for. What closes that is a materialised snapshot per revision,
    /// which is `story:commit-and-revision-lineage`'s, not this story's.
    ///
    /// # Errors
    ///
    /// [`StoreError::NoMaterialisedState`] for a revision this store holds no state at, and
    /// whichever refusal of [`StoreError`] the sequence of events earns.
    fn replay(&self, from: RevisionNumber) -> Result<CanonicalGraph, StoreError>;
}

/// Atomic publication port used only by the kernel's validated bootstrap path.
pub trait Initialize: RevisionLog {
    /// Publishes the retained seed and its first event as one atomic group.
    ///
    /// # Errors
    /// Existing lineage, absent authority, invalid seed, or provider failure.
    fn initialize(&self, bytes: &[u8], at: ekr_core::Timestamp) -> Result<(), StoreError>;
}

/// The fold in progress: the state the events have moved so far.
///
/// Held by the implementation while it walks the stream. Not public: what a caller gets is the
/// graph or the head, and a half-applied lineage is neither.
pub(crate) struct Fold<'a> {
    /// The state, as far as the events have moved it.
    graph: CanonicalGraph,
    /// The root of the last revision committed, or the seed's.
    head: Root,
    /// Transactions that have been proposed and not yet resolved, by the address of their
    /// operations — which is what a committed [`Root`] carries as its `transaction`.
    proposed: BTreeMap<TransactionId, ContentHash>,
    /// What the log *claims* about each of those, for the ones a `TransactionValidated` named.
    ///
    /// A claim and not a conclusion, which is the whole of ADR 0007 in this struct: it was a
    /// `BTreeSet<TransactionId>` filled by the arrival of an event, so appending one was the same
    /// thing as being validated.
    validated: BTreeMap<TransactionId, RecordedValidation>,
    /// What decides whether a claim in `validated` moves canonical state, or `None` for a store
    /// opened without one — which folds no commit at all. See [`CommitAuthority`].
    authority: Option<&'a dyn CommitAuthority>,
    /// The first commit this fold could not evaluate **because there was nobody to ask**, which is
    /// a different thing from one an authority declined and is reported rather than absorbed.
    unauthorised: Option<TransactionId>,
}

impl<'a> Fold<'a> {
    /// The fold at the seed: revision zero, no parent, the seed's own state.
    pub(crate) fn seeded(
        mut graph: CanonicalGraph,
        seed_hash: ContentHash,
        authority: Option<&'a dyn CommitAuthority>,
    ) -> Self {
        graph.revision = RevisionNumber::SEED;
        let head = Root {
            revision: RevisionNumber::SEED,
            parent: None,
            ontology_root: PLACEHOLDER_SUB_ROOT,
            knowledge_root: knowledge_root(&graph),
            evidence_root: evidence_root(&graph),
            agent_root: PLACEHOLDER_SUB_ROOT,
            // Design § 34's lineage begins `Seed -> Root0`. The seed is what produced this root,
            // and the seed's address is what names it.
            transaction: seed_hash,
        };
        Self {
            graph,
            head,
            proposed: BTreeMap::new(),
            validated: BTreeMap::new(),
            authority,
            unauthorised: None,
        }
    }

    /// Applies one event, or refuses it.
    ///
    /// Committed events only; nothing here re-runs validation. What it does check is that the
    /// sequence is a lineage: proposed before validated, validated before committed, one revision
    /// per commit in order, and a published knowledge root the fold's own state reaches.
    ///
    /// # How far P1's reproducibility actually goes
    ///
    /// [`commit`](Fold::commit) applies **no operations**, because no `RevisionEvent` variant
    /// carries any: the six move the lineage and name addresses, and the graph changes a
    /// transaction made are nowhere in the log. So the knowledge root this fold reaches is always
    /// the seed's, and [`StoreError::KnowledgeRootDisagrees`] refuses any lineage in which
    /// knowledge changed.
    ///
    /// Stated plainly, because it is easy to read the green acceptance as more than it is:
    /// `docs/roadmap.md` § 4's P1 exit criterion, "replay from the seed reproduces the root hash",
    /// holds **only for lineages in which knowledge never changes.** That is the whole of what this
    /// unit delivers against it.
    ///
    /// The refusal is deliberately the loud half of that. Copying the published root through
    /// instead would make replay report a reproduction it had not performed, and would make the
    /// exit criterion true by construction for every lineage — including the ones where the store
    /// has no idea what the state is. The first wave whose kernel commits a real change will meet
    /// this refusal, and meeting it is the signal that graph operations have to enter the fold.
    pub(crate) fn apply(&mut self, event: &RevisionEvent) -> Result<(), StoreError> {
        match event {
            RevisionEvent::Seeded { .. } => return Err(StoreError::SeedIsNotFirst),
            RevisionEvent::TransactionProposed {
                transaction_id,
                operations_hash,
                ..
            } => {
                self.proposed.insert(*transaction_id, *operations_hash);
            }
            RevisionEvent::TransactionValidated {
                transaction_id,
                against,
                validation_hash,
            } => {
                if !self.proposed.contains_key(transaction_id) {
                    return Err(StoreError::ProposalMissing {
                        transaction_id: *transaction_id,
                    });
                }
                // Recorded, not believed. What the event says is kept whole — including the
                // `against` the fold used to discard — and [`Fold::commit`] is where it is asked
                // whether any of it stands.
                self.validated.insert(
                    *transaction_id,
                    RecordedValidation {
                        transaction_id: *transaction_id,
                        against: *against,
                        validation_hash: *validation_hash,
                    },
                );
            }
            RevisionEvent::TransactionRejected { transaction_id, .. }
            | RevisionEvent::TransactionStale { transaction_id, .. } => {
                self.proposed.remove(transaction_id);
                self.validated.remove(transaction_id);
            }
            RevisionEvent::RevisionCommitted {
                transaction_id,
                number,
                knowledge_root: published,
                ..
            } => self.commit(*transaction_id, *number, *published)?,
        }
        Ok(())
    }

    /// The lineage advances: design § 34's `Root_n -> Root_n+1`.
    fn commit(
        &mut self,
        transaction_id: TransactionId,
        number: RevisionNumber,
        published: ContentHash,
    ) -> Result<(), StoreError> {
        let validation = self
            .validated
            .remove(&transaction_id)
            .ok_or(StoreError::ValidationMissing { transaction_id })?;
        let operations = self
            .proposed
            .remove(&transaction_id)
            .ok_or(StoreError::ProposalMissing { transaction_id })?;

        // Two different things, and the difference is whose error it is.
        //
        // **Nobody to ask** is the caller's: this store was opened without an authority, so it
        // cannot say whether the commit stands. The lineage does not advance and the fold records
        // it, and `RevisionLog::fold` turns that into `StoreError::NoCommitAuthority` rather than
        // answering a state it has no basis for.
        let Some(authority) = self.authority else {
            self.unauthorised.get_or_insert(transaction_id);
            return Ok(());
        };
        // **Asked and declined** is the log's, and is silent. AGENTS.md invariant 1 at the one
        // place this crate can carry it: a validation the authority does not stand behind is a
        // claim in an append-only log and not a commit. The transaction is resolved either way —
        // it is out of `proposed` and `validated` above — and the lineage does not advance under
        // it. Silent because an error here would let one append by anyone with the log brick every
        // later read of it, which is a worse answer than the true one.
        if !authority.attests(&validation) {
            return Ok(());
        }

        // Design § 72: "a transaction is committed only against the revision it was validated
        // against". The fold was handed that revision in `TransactionValidated.against` and threw
        // it away, so a transaction validated at revision 0 that another revision landed under
        // replayed as valid — a replay reporting as reproducible a lineage the design calls stale.
        // It does not advance the lineage, for the reason an unattested one does not: the log
        // records that the kernel claimed it, and the fold records that it did not take.
        if validation.against != self.head.revision {
            return Ok(());
        }

        let expected = self
            .head
            .revision
            .next()
            .ok_or(StoreError::RevisionOutOfOrder {
                expected: self.head.revision,
                found: number,
            })?;
        if number != expected {
            return Err(StoreError::RevisionOutOfOrder {
                expected,
                found: number,
            });
        }

        self.graph.revision = number;
        let folded = knowledge_root(&self.graph);
        if folded != published {
            return Err(StoreError::KnowledgeRootDisagrees {
                revision: number,
                published,
                folded,
            });
        }

        self.head = Root {
            revision: number,
            // The hash of the previous root rather than its number, which is what makes the
            // lineage a chain a reader can verify rather than a sequence a writer asserts.
            parent: Some(ContentHash::of(&self.head)),
            ontology_root: PLACEHOLDER_SUB_ROOT,
            knowledge_root: folded,
            evidence_root: evidence_root(&self.graph),
            agent_root: PLACEHOLDER_SUB_ROOT,
            transaction: operations,
        };
        Ok(())
    }

    /// The first commit this fold had nobody to ask about, if there was one.
    pub(crate) const fn unauthorised(&self) -> Option<TransactionId> {
        self.unauthorised
    }

    /// The state the events moved.
    pub(crate) fn into_graph(self) -> CanonicalGraph {
        self.graph
    }

    /// The root of the last revision committed.
    pub(crate) const fn head(&self) -> Root {
        self.head
    }
}
