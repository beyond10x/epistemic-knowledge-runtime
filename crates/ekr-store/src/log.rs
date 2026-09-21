//! The revision log: what may be appended to a lineage, and what folding one produces.
//!
//! Design § 34. A committed revision is an event, the canonical graph is a fold over the log, and
//! the fold is a *verification* rather than a copy — `docs/roadmap.md` § 4's P1 exit criterion is
//! that "replay from the seed reproduces the root hash", and reproducing means computing.

use std::collections::{BTreeMap, BTreeSet};

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

/// The fold in progress: the state the events have moved so far.
///
/// Held by the implementation while it walks the stream. Not public: what a caller gets is the
/// graph or the head, and a half-applied lineage is neither.
pub(crate) struct Fold {
    /// The state, as far as the events have moved it.
    graph: CanonicalGraph,
    /// The root of the last revision committed, or the seed's.
    head: Root,
    /// Transactions that have been proposed and not yet resolved, by the address of their
    /// operations — which is what a committed [`Root`] carries as its `transaction`.
    proposed: BTreeMap<TransactionId, ContentHash>,
    /// Which of those have been validated. Design § 20 and AGENTS.md invariant 1: only a validated
    /// transaction commits, and the fold is where the log is held to it.
    validated: BTreeSet<TransactionId>,
}

impl Fold {
    /// The fold at the seed: revision zero, no parent, the seed's own state.
    pub(crate) fn seeded(mut graph: CanonicalGraph, seed_hash: ContentHash) -> Self {
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
            validated: BTreeSet::new(),
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
            RevisionEvent::TransactionValidated { transaction_id, .. } => {
                if !self.proposed.contains_key(transaction_id) {
                    return Err(StoreError::ProposalMissing {
                        transaction_id: *transaction_id,
                    });
                }
                self.validated.insert(*transaction_id);
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
        if !self.validated.remove(&transaction_id) {
            return Err(StoreError::ValidationMissing { transaction_id });
        }
        let operations = self
            .proposed
            .remove(&transaction_id)
            .ok_or(StoreError::ProposalMissing { transaction_id })?;

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

    /// The state the events moved.
    pub(crate) fn into_graph(self) -> CanonicalGraph {
        self.graph
    }

    /// The root of the last revision committed.
    pub(crate) const fn head(&self) -> Root {
        self.head
    }
}
