//! Complete retained decision verification; the same pure authority checks new publications.
use crate::{
    validate::{AssertedEdgesCell, HeldIdentities},
    AuthorityStateV1, CommitReceiptV1, GraphOperation, GraphTransaction, KernelAuthority, Pipeline,
    ProposalRecordV1, RejectionRecordV1, SeedResultV1, StaleRecordV1, TransactionDocument,
    ValidatedTransaction, ValidationBasisV1, ValidationMaterialV1, ValidationReceiptV1,
};
use ekr_core::{
    AgentId, Canonical, ContentHash, Encoder, EventId, IssueId, RevisionId, RevisionNumber,
    Timestamp, TransactionId,
};
use ekr_graph::{AliasIndex, CanonicalValue, GraphSnapshot, RevisionPayload};
use ekr_store::{AdmittedRevision, RecordedOccurrence, RetainedHistory, StorageClass, StoreError};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, OnceLock};

/// Actual retained transaction lifecycle, independent of provider stream position.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum TransactionState {
    /// Awaiting validation.
    Proposed,
    /// Accepted at a retained basis.
    Validated,
    /// Applied to an immutable revision.
    Committed,
    /// Refused with retained deterministic issues.
    Rejected,
    /// The canonical head moved before publication.
    Stale,
}
/// A transaction and its actual retained decisions. Reads never fabricate a missing state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct TransactionRecord {
    /// Exact original submitted input and host attribution.
    pub proposal: ProposalRecordV1,
    /// Payload-domain address of that proposal.
    pub proposal_record_hash: ContentHash,
    /// Its accepted validation, if one exists.
    pub validation: Option<ValidationReceiptV1>,
    /// Actual validation payload address.
    pub validation_record_hash: Option<ContentHash>,
    /// Its terminal deterministic refusal, if one exists.
    pub rejection: Option<RejectionRecordV1>,
    /// Its terminal committed receipt, if one exists.
    pub committed: Option<CommitReceiptV1>,
    /// Its terminal stale decision, if one exists.
    pub stale: Option<StaleRecordV1>,
}
impl TransactionRecord {
    /// The actual retained lifecycle state.
    #[must_use]
    pub const fn state(&self) -> TransactionState {
        if self.committed.is_some() {
            TransactionState::Committed
        } else if self.stale.is_some() {
            TransactionState::Stale
        } else if self.rejection.is_some() {
            TransactionState::Rejected
        } else if self.validation.is_some() {
            TransactionState::Validated
        } else {
            TransactionState::Proposed
        }
    }
}
/// The state a complete verified replay reaches after some prefix of the revision stream.
///
/// Cloning is cheap where it matters: every admitted revision, parsed document and sealed
/// validation is shared, and so are the retained records until a replay extending the clone
/// changes one ([`Self::transactions_mut`]). A read shares them and never copies them.
#[derive(Clone)]
pub(crate) struct ReplayState {
    pub(crate) upgraded_authority: Option<AuthorityStateV1>,
    pub(crate) transition: Option<ekr_core::contract_data::EkrKernelAuthorityTransitionRecord>,
    pub(crate) assessment_validators: BTreeMap<ekr_core::AssertionId, BTreeSet<AgentId>>,
    pub(crate) seed: SeedResultV1,
    pub(crate) revisions: BTreeMap<RevisionNumber, Revision>,
    pub(crate) transactions: Arc<BTreeMap<TransactionId, Arc<TransactionRecord>>>,
    /// The public owned-record snapshot, materialized only when a reader asks for it and shared
    /// by subsequent reads. Commands update individual shared records without copying old input.
    pub(crate) transaction_snapshot: OnceLock<Arc<BTreeMap<TransactionId, TransactionRecord>>>,
    pub(crate) version: u64,
    /// The prefix digest of the occurrences this state covers, when replay computed it.
    pub(crate) digest: Option<ContentHash>,
    /// The evidence payloads the admitted seed envelope requires.
    pub(crate) seed_payloads: BTreeSet<ContentHash>,
    /// Each retained proposal's document, parsed once from its verified bytes.
    pub(crate) documents: BTreeMap<TransactionId, Arc<TransactionDocument>>,
    /// Each accepted validation's sealed result, keyed by transaction. Validation is a pure
    /// function of the proposal, the basis revision and the lineage before it, all immutable, so
    /// a commit whose basis is still that revision reuses the result instead of recomputing it.
    pub(crate) validated: BTreeMap<TransactionId, Arc<ValidatedTransaction>>,
    pub(crate) revision_ids: BTreeSet<RevisionId>,
    pub(crate) event_ids: BTreeSet<EventId>,
    pub(crate) issue_ids: BTreeSet<IssueId>,
    /// Under a profile that keeps identities, every node and edge id a revision of this state
    /// held, from the revision that first held it; empty under every other profile. A state
    /// restored from a checkpoint holds it too, so validating against any revision reads it here
    /// rather than from that revision's graph.
    pub(crate) held: HeldIdentities,
    /// Each validation or rejection this state holds whose basis was not the head when it was
    /// recorded, as its occurrence's stream position and that basis. Replaying one needs its
    /// basis revision's graph, which a checkpoint of a later head does not hold (design § 99).
    pub(crate) earlier_bases: Vec<(u64, RevisionNumber)>,
}
/// The refusal a state restored from a checkpoint gives for a graph it does not hold. It is never
/// a verdict about the history: whoever meets it replays that history in full instead.
pub(crate) const GRAPH_NOT_HELD: &str = "replay-graph-not-held";

/// One committed revision as replay holds it: its verified coordinates always, its graph only
/// while the state keeps it. A state keeps the graph of its head and of the retained checkpoint's
/// head, the graph of a revision a later occurrence of the same replay is validated or rejected
/// against until that occurrence is replayed, and the graph of the revision a running read names
/// ([`KernelAuthority::keeping`]); every other graph is released when the head moves past it, and
/// reconstructed by a replay to its revision when a command or read asks for it
/// ([`KernelAuthority::graph_at`]). A state restored from a checkpoint holds only the head's graph.
#[derive(Clone, Debug)]
pub(crate) struct Revision {
    pub(crate) root: ekr_graph::Root,
    pub(crate) revision_id: RevisionId,
    pub(crate) event_id: EventId,
    pub(crate) record_hash: ContentHash,
    pub(crate) committed_at: Timestamp,
    /// The graph root every revision of the lineage hangs off.
    pub(crate) graph_root: ekr_core::GraphRootId,
    /// The schema in force at this revision.
    pub(crate) ontology: Arc<ekr_ontology::Ontology>,
    pub(crate) graph: Option<Arc<ekr_graph::CanonicalGraph>>,
    /// The index of this revision's assertions about edges, by edge, kept with its graph: built by
    /// the first validation that reads it and shared by every state holding this revision or a
    /// verified successor whose operations leave this lookup unchanged. A cold reconstruction
    /// starts with a fresh cell; sharing never crosses an unverified graph boundary.
    pub(crate) asserted_edges: AssertedEdgesCell,
    /// Mutable lookup scratch shared along a lineage; every use checks exact revision and roots.
    pub(crate) alias_holders: Arc<std::sync::Mutex<crate::validate::AliasCache>>,
}
impl Revision {
    pub(crate) fn replayed(admitted: AdmittedRevision) -> Self {
        Self {
            root: admitted.root,
            revision_id: admitted.revision_id,
            event_id: admitted.event_id,
            record_hash: admitted.record_hash,
            committed_at: admitted.committed_at,
            graph_root: admitted.graph.root.id,
            ontology: Arc::new(admitted.graph.ontology.clone()),
            graph: Some(Arc::new(admitted.graph)),
            asserted_edges: AssertedEdgesCell::default(),
            alias_holders: Default::default(),
        }
    }
    /// Releases this revision's graph and the index kept with it.
    fn release_graph(&mut self) {
        self.graph = None;
        self.asserted_edges = AssertedEdgesCell::default();
        self.alias_holders = Default::default();
    }
    /// This lookup records assertion identities by edge subject, including retracted assertions.
    /// Only adding an edge assertion changes it; node assertions, lifecycle changes and deleting
    /// an edge leave the lookup intact. A future operation must make an explicit choice here.
    fn asserted_edges_after(&self, tx: &GraphTransaction<CanonicalValue>) -> AssertedEdgesCell {
        let changes = tx.operations.iter().any(|operation| match operation {
            GraphOperation::AddAssertion(assertion) => {
                matches!(assertion.subject, ekr_graph::Subject::Edge(_))
            }
            GraphOperation::CreateNode(_)
            | GraphOperation::AddAlias(_)
            | GraphOperation::UpdateProperty(_)
            | GraphOperation::CreateEdge(_)
            | GraphOperation::DeleteEdge(_)
            | GraphOperation::RetractAssertion(_)
            | GraphOperation::DefineNodeType(_)
            | GraphOperation::DefineEdgeType(_)
            | GraphOperation::ModifyProperty(_)
            | GraphOperation::MergeEntity(_)
            | GraphOperation::Invoke { .. }
            | GraphOperation::SupersedeAssertion(_)
            | GraphOperation::AddEvidence(_)
            | GraphOperation::AttachEvidence(_)
            | GraphOperation::WidenEdgeType(_) => false,
        });
        if changes {
            AssertedEdgesCell::default()
        } else {
            Arc::clone(&self.asserted_edges)
        }
    }
    /// The graph at this revision, or [`GRAPH_NOT_HELD`].
    pub(crate) fn graph(&self) -> Result<&ekr_graph::CanonicalGraph, StoreError> {
        self.graph.as_deref().ok_or_else(|| refuse(GRAPH_NOT_HELD))
    }
    pub(crate) fn admitted(&self) -> Result<AdmittedRevision, StoreError> {
        Ok(AdmittedRevision {
            graph: self.graph()?.clone(),
            root: self.root,
            revision_id: self.revision_id,
            event_id: self.event_id,
            record_hash: self.record_hash,
            committed_at: self.committed_at,
        })
    }
}
/// Whether `error` is [`GRAPH_NOT_HELD`].
pub(crate) fn graph_not_held(error: &StoreError) -> bool {
    matches!(error, StoreError::Document(code) if code == GRAPH_NOT_HELD)
}

impl ReplayState {
    pub(crate) fn active_authority<'a>(
        &'a self,
        anchor: &'a AuthorityStateV1,
    ) -> &'a AuthorityStateV1 {
        self.upgraded_authority.as_ref().unwrap_or(anchor)
    }
    /// Notes that the occurrence at `version` was validated against `basis`, when that is not
    /// the head it was recorded at.
    pub(crate) fn note_basis(&mut self, version: u64, basis: RevisionNumber) {
        if basis < self.head().root.revision {
            self.earlier_bases.push((version, basis));
        }
    }
    pub(crate) fn head(&self) -> &Revision {
        self.revisions
            .last_key_value()
            .expect("state is constructed with verified seed")
            .1
    }
    /// Releases the graph of revision `number` unless it is the head's or `kept` keeps it.
    fn release(&mut self, number: RevisionNumber, kept: impl Fn(RevisionNumber) -> bool) {
        if number == self.head().root.revision || kept(number) {
            return;
        }
        if let Some(revision) = self.revisions.get_mut(&number) {
            revision.release_graph();
        }
    }
    /// Releases every graph but the head's and those `kept` keeps.
    fn release_all(&mut self, kept: impl Fn(RevisionNumber) -> bool) {
        let head = self.head().root.revision;
        for (number, revision) in &mut self.revisions {
            if *number != head && revision.graph.is_some() && !kept(*number) {
                revision.release_graph();
            }
        }
    }
    /// The retained record index to change. Copying the index shares each unchanged record.
    pub(crate) fn transactions_mut(
        &mut self,
    ) -> &mut BTreeMap<TransactionId, Arc<TransactionRecord>> {
        self.transaction_snapshot.take();
        Arc::make_mut(&mut self.transactions)
    }
    /// Changes only the selected record, leaving every prior state's records immutable.
    pub(crate) fn transaction_mut(&mut self, id: TransactionId) -> &mut TransactionRecord {
        Arc::make_mut(
            self.transactions_mut()
                .get_mut(&id)
                .expect("verified transaction"),
        )
    }
    /// The unchanged public record-map shape, shared for every read of this verified state.
    pub(crate) fn transaction_records(&self) -> Arc<BTreeMap<TransactionId, TransactionRecord>> {
        Arc::clone(self.transaction_snapshot.get_or_init(|| {
            Arc::new(
                self.transactions
                    .iter()
                    .map(|(id, record)| (*id, (**record).clone()))
                    .collect(),
            )
        }))
    }
    /// The retained proposal's parsed document, or a fresh parse of its verified bytes.
    pub(crate) fn document(
        &self,
        proposal: &ProposalRecordV1,
    ) -> Result<Arc<TransactionDocument>, StoreError> {
        if let Some(parsed) = self.documents.get(&proposal.transaction_id) {
            return Ok(Arc::clone(parsed));
        }
        TransactionDocument::parse(&proposal.document_bytes)
            .map(Arc::new)
            .map_err(|e| refuse(&format!("proposal-document: {e}")))
    }
}

/// The digest of a revision-stream prefix: a chain over each occurrence's stream position and
/// complete domain event, which carries the payload address of every record replay reads.
///
/// Two histories with one digest hold the same occurrences in the same order and, because
/// retained objects are content-addressed and verified on load, the same record bytes. Replay is
/// deterministic in exactly those inputs under one authority, so a state reached over one of them
/// is the state reached over the other. The provider's own event identity is not an input of
/// replay and is not bound, so a candidate replayed before publication and the same occurrence
/// read back after it share a digest.
fn extend_digest(
    previous: ContentHash,
    version: u64,
    event: &ekr_graph::RevisionEvent,
) -> ContentHash {
    #[cfg(test)]
    PREFIX_STEPS.with(|count| count.set(count.get() + 1));
    struct Step<'a>(ContentHash, u64, &'a ekr_graph::RevisionEvent);
    impl Canonical for Step<'_> {
        fn encode(&self, out: &mut Encoder) {
            "ekr.replay-prefix/1".encode(out);
            self.0.encode(out);
            self.1.encode(out);
            self.2.encode(out);
        }
    }
    ContentHash::of(&Step(previous, version, event))
}
#[cfg(test)]
thread_local! {
    static PREFIX_STEPS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
pub(crate) fn prefix_digests(occurrences: &[RecordedOccurrence]) -> Vec<ContentHash> {
    let mut digests = Vec::with_capacity(occurrences.len() + 1);
    let mut digest = ContentHash::of("ekr.replay-prefix/1");
    digests.push(digest);
    for occurrence in occurrences {
        digest = extend_digest(digest, occurrence.version, &occurrence.event);
        digests.push(digest);
    }
    digests
}

/// Verified replay states this authority has already reached, by the prefix they cover.
///
/// Process-local and never persisted: an entry is only ever a state this authority computed
/// itself from verified history, so reusing it is reusing its own result. A handful of entries
/// covers one command, which replays the head, then the head plus its candidate, several times.
#[derive(Default)]
pub(crate) struct ReplayCache {
    entries: Vec<(usize, ContentHash, Arc<ReplayState>)>,
    /// Private confirmed predecessor scratch. The authority owns it; the applying thread holds
    /// only a weak locator, so dropping the authority releases every retained record promptly.
    retired: Arc<std::sync::Mutex<Option<crate::apply::Retired>>>,
    /// Inputs and digests from the latest hashing pass. Every reuse compares the complete
    /// domain occurrence and its position; provider identities do not enter the digest.
    prefix_occurrences: Vec<RecordedOccurrence>,
    prefix_hashes: Arc<Vec<ContentHash>>,
    /// The seed envelope this authority admitted, the evidence payloads it names, and whether it
    /// names them (`ekr-seed-envelope/3`: read where held) rather than carrying them (`/2`:
    /// required).
    pub(crate) seed: Option<(ContentHash, BTreeSet<ContentHash>, bool)>,
    /// The seed envelope this authority decoded in full from verified retained bytes, by their
    /// address. It is a function of that address, so every path that verified the retained bytes
    /// at the same address takes it instead of decoding them again.
    pub(crate) envelope: Option<(ContentHash, Arc<crate::seed::SeedEnvelope>)>,
    /// The cell holding the [`AliasIndex`] of the newest head a read captured, by that head's
    /// revision identity and root. One slot: a read of a newer head replaces it, so this
    /// authority keeps the index of no head but the current one.
    pub(crate) aliases: Option<(RevisionId, ekr_graph::Root, AliasCell)>,
    /// The retained replay checkpoint this authority knows of — the one it admitted, or the last
    /// it wrote — as the occurrences it covers and its head revision; `None` before either. What
    /// decides whether a commit writes the next one (design § 99).
    pub(crate) retained: Option<(u64, RevisionNumber)>,
    /// The revision whose graph every replay keeps too while a read that asked for it runs
    /// ([`KernelAuthority::keeping`]), so that the replay verifying its history holds that graph
    /// rather than a second replay reconstructing it.
    keep: Option<RevisionNumber>,
    /// How many replays this authority began at the seed rather than at a state it had reached.
    pub(crate) seed_replays: u64,
    /// How many times this authority decoded a retained seed envelope's complete bytes.
    pub(crate) envelope_decodes: u64,
    /// Whether this authority is the one publishing a preserving migration into its store, which
    /// alone reads that store before the migration finished (design § 100.3).
    pub(crate) migrating: bool,
    /// Whether this authority has seen its store's history with no unfinished migration, after
    /// which it stops asking for the migration markers: a store with a history never gains one.
    pub(crate) migration_settled: bool,
}
/// While held, every replay of the authority that returned it keeps one more revision's graph
/// ([`KernelAuthority::keeping`]). Dropping it releases that graph from every state the cache
/// holds, unless it is that state's head or the retained checkpoint's; a caller that still needs
/// the graph holds its own reference.
pub(crate) struct Keeping<'a>(&'a KernelAuthority, RevisionNumber);
impl Drop for Keeping<'_> {
    fn drop(&mut self) {
        if let Ok(mut cache) = self.0.cache.lock() {
            cache.keep = None;
            cache.release(self.1);
        }
    }
}
/// Where one head's [`AliasIndex`] is built, by the first read that asks for it.
pub(crate) type AliasCell = Arc<OnceLock<Arc<AliasIndex>>>;
impl ReplayCache {
    const CAPACITY: usize = 4;
    /// Hash only the suffix after the first different occurrence. Comparing complete inputs
    /// keeps this a pure hash memo, including for a shorter history, a fork or tampered history;
    /// it confers no replay authority and never substitutes for retained-object verification.
    pub(crate) fn digests(&mut self, occurrences: &[RecordedOccurrence]) -> Arc<Vec<ContentHash>> {
        let common = self
            .prefix_occurrences
            .iter()
            .zip(occurrences)
            .take_while(|(left, right)| left.version == right.version && left.event == right.event)
            .count();
        if common != occurrences.len() || common != self.prefix_occurrences.len() {
            self.prefix_occurrences.truncate(common);
            let digests = Arc::make_mut(&mut self.prefix_hashes);
            digests.truncate(common + 1);
            if digests.is_empty() {
                digests.push(ContentHash::of("ekr.replay-prefix/1"));
            }
            let mut previous = digests[common];
            for occurrence in &occurrences[common..] {
                previous = extend_digest(previous, occurrence.version, &occurrence.event);
                digests.push(previous);
            }
            self.prefix_occurrences
                .extend_from_slice(&occurrences[common..]);
        } else if self.prefix_hashes.is_empty() {
            Arc::make_mut(&mut self.prefix_hashes).push(ContentHash::of("ekr.replay-prefix/1"));
        }
        Arc::clone(&self.prefix_hashes)
    }
    /// The alias-index cell of the head `revision_id` with `root`: the held one when it is that
    /// head's, and otherwise a new one, which replaces it.
    pub(crate) fn alias_cell(
        &mut self,
        revision_id: RevisionId,
        root: ekr_graph::Root,
    ) -> AliasCell {
        match &self.aliases {
            Some((held, at, cell)) if *held == revision_id && *at == root => Arc::clone(cell),
            _ => {
                let cell = AliasCell::default();
                self.aliases = Some((revision_id, root, Arc::clone(&cell)));
                cell
            }
        }
    }
    /// The longest cached prefix of the history whose digests are `digests`.
    pub(crate) fn longest(&self, digests: &[ContentHash]) -> Option<(usize, Arc<ReplayState>)> {
        self.entries
            .iter()
            .filter(|(covered, digest, _)| digests.get(*covered) == Some(digest))
            .max_by_key(|(covered, _, _)| *covered)
            .map(|(covered, _, state)| (*covered, Arc::clone(state)))
    }
    /// The state covering the most occurrences.
    pub(crate) fn newest(&self) -> Option<Arc<ReplayState>> {
        self.entries
            .iter()
            .max_by_key(|(covered, _, _)| *covered)
            .map(|(_, _, state)| Arc::clone(state))
    }
    /// Retire the predecessor only after the store confirms this exact publication. The cached
    /// candidate must extend that predecessor's digest with the confirmed occurrence, so neither
    /// the longest unrelated candidate nor a different history at the same version is proof.
    /// Before confirmation the predecessor stays available for CAS retries. Readers holding an
    /// old state keep their own Arc, and the successor already keeps checkpoint/historical graphs
    /// required by replay. An absent match merely leaves the normal bounded cache in place.
    pub(crate) fn confirmed(&mut self, publication: &ekr_store::Publication) {
        if self
            .entries
            .iter()
            .any(|(_, _, state)| state.transition.is_some())
        {
            return;
        }
        let Some(version) = publication.expected_version.checked_add(1) else {
            return;
        };
        let obsolete = self.entries.iter().find_map(|(covered, digest, state)| {
            if state.version != publication.expected_version {
                return None;
            }
            let next = extend_digest(*digest, version, &publication.event);
            self.entries
                .iter()
                .find(|(later, found, candidate)| {
                    *later == covered + 1 && *found == next && candidate.version == version
                })
                .map(|(_, _, candidate)| ((*covered, *digest), Arc::clone(candidate)))
        });
        if let Some((obsolete, candidate)) = obsolete {
            if let Some(index) = self
                .entries
                .iter()
                .position(|(covered, digest, _)| (*covered, *digest) == obsolete)
            {
                let (_, _, retired) = self.entries.remove(index);
                if let RevisionPayload::RevisionCommitted { transaction_id, .. } =
                    publication.event.payload
                {
                    crate::apply::retire(&self.retired, retired, candidate.head(), transaction_id);
                }
            }
        }
    }
    /// Releases the graph of revision `number` from every held state that holds it and does not
    /// keep it as its head, the retained checkpoint's head or an active historical read. Each
    /// state is replaced by a
    /// copy without it, so that a caller still sharing the old state keeps what it holds.
    pub(crate) fn release(&mut self, number: RevisionNumber) {
        let checkpointed = self.retained.map(|(_, revision)| revision);
        for entry in &mut self.entries {
            let state = &entry.2;
            let releases = Some(number) != checkpointed
                && Some(number) != self.keep
                && state.head().root.revision != number
                && state
                    .revisions
                    .get(&number)
                    .is_some_and(|revision| revision.graph.is_some());
            if releases {
                let mut released = (**state).clone();
                released.release(number, |_| false);
                entry.2 = Arc::new(released);
            }
        }
    }
    /// Replaces the held state whose prefix digest is `digest` with `state`, which covers the same
    /// prefix, if one is held.
    fn replace(&mut self, digest: ContentHash, state: Arc<ReplayState>) {
        if let Some(entry) = self
            .entries
            .iter_mut()
            .find(|(_, found, _)| *found == digest)
        {
            entry.2 = state;
        }
    }
    /// [`Self::insert`] of `state`, which covers the whole history whose prefix digests are
    /// `digests`, dropping every held state that covers a shorter prefix of that history than the
    /// longest one held.
    ///
    /// Revision streams only grow, and a state is reached over a verified history or over one
    /// with a single candidate occurrence after it. The longest held prefix of `state`'s history
    /// is therefore a prefix of a verified history, which every later history extends, so a
    /// shorter one is never again the longest a replay continues from. Each held state that a
    /// replay extended holds its own copy of the retained records, so a chain of them would hold
    /// that many copies.
    fn insert_extending(&mut self, digests: &[ContentHash], state: Arc<ReplayState>) {
        let covered = digests.len() - 1;
        let prefix =
            |held: usize, found: &ContentHash| held < covered && digests.get(held) == Some(found);
        if let Some(longest) = self
            .entries
            .iter()
            .filter(|(held, found, _)| prefix(*held, found))
            .map(|(held, _, _)| *held)
            .max()
        {
            self.entries
                .retain(|(held, found, _)| !prefix(*held, found) || *held == longest);
        }
        self.insert(covered, digests[covered], state);
    }
    pub(crate) fn insert(&mut self, covered: usize, digest: ContentHash, state: Arc<ReplayState>) {
        self.entries
            .retain(|(held, found, _)| (*held, *found) != (covered, digest));
        if self.entries.len() == Self::CAPACITY {
            self.entries.remove(0);
        }
        self.entries.push((covered, digest, state));
    }
}
pub(crate) fn refuse(code: &str) -> StoreError {
    StoreError::Document(code.into())
}
pub(crate) fn require(condition: bool, code: &str) -> Result<(), StoreError> {
    if condition {
        Ok(())
    } else {
        Err(refuse(code))
    }
}
pub(crate) fn registered(anchor: &AuthorityStateV1, actor: AgentId) -> Result<(), StoreError> {
    require(anchor.agents.contains_key(&actor), "unregistered-actor")
}
/// Whether every attribution `tx` writes is `actor`'s: its proposer, each assertion's
/// `proposed_by` and each added evidence entry's `extracted_by`. The host submitter is who
/// proposes, claims and extracts; a document naming anyone else is misattributed.
pub(crate) fn attributed(tx: &GraphTransaction, actor: AgentId) -> bool {
    tx.proposer == actor
        && tx.operations.iter().all(|op| match op {
            GraphOperation::AddAssertion(assertion) => assertion.proposed_by == actor,
            GraphOperation::AddEvidence(addition) => addition.evidence.extracted_by == actor,
            _ => true,
        })
}
pub(crate) fn proposal(
    bytes: &[u8],
    actor: AgentId,
    event_id: EventId,
    at: Timestamp,
    anchor: &AuthorityStateV1,
) -> Result<ProposalRecordV1, StoreError> {
    parsed_proposal(bytes, actor, event_id, at, anchor, ProposalRecordV1::FORMAT)
        .map(|(record, _)| record)
}
/// [`proposal`] in the record format a retained proposal was written in, returning the parsed
/// document with it so that one replay parses each retained document once.
pub(crate) fn parsed_proposal(
    bytes: &[u8],
    actor: AgentId,
    event_id: EventId,
    at: Timestamp,
    anchor: &AuthorityStateV1,
    format: &str,
) -> Result<(ProposalRecordV1, TransactionDocument), StoreError> {
    registered(anchor, actor)?;
    let document = TransactionDocument::parse(bytes)
        .map_err(|e| refuse(&format!("proposal-document: {e}")))?;
    let tx = document.transaction();
    require(attributed(tx, actor), "proposal-attribution-mismatch")?;
    let canonical = GraphTransaction::<CanonicalValue>::try_from(tx.clone()).ok();
    let record = ProposalRecordV1 {
        format: format.into(),
        event_id,
        submitted_at: at,
        submitter: actor,
        document_hash: document.hash(),
        document_bytes: bytes.to_vec(),
        transaction_id: tx.id,
        operation_count: tx.operations.len() as u64,
        evidence_hash: ContentHash::of(&tx.evidence),
        canonical_transaction_hash: canonical.as_ref().map(ContentHash::of),
        canonical_operations_hash: canonical.as_ref().map(|tx| ContentHash::of(&tx.operations)),
    };
    Ok((record, document))
}
pub(crate) fn basis(
    prior: &Revision,
    seed_hash: ContentHash,
    anchor: &AuthorityStateV1,
) -> ValidationBasisV1 {
    ValidationBasisV1 {
        format: ValidationBasisV1::FORMAT.into(),
        graph_root_id: prior.graph_root,
        previous_revision_id: prior.revision_id,
        previous_event_id: prior.event_id,
        previous_record_hash: prior.record_hash,
        previous_root: prior.root,
        previous_root_hash: ContentHash::of(&prior.root),
        seed_hash,
        ontology_root: prior.root.ontology_root,
        authority_root: prior.root.agent_root,
        validation_profile_hash: ContentHash::of(&anchor.validation_profile),
    }
}
/// Validates a retained proposal's parsed document against `prior` under the store's own profile.
///
/// `revisions` are the committed revisions retained so far; those up to `prior` are the lineage a
/// profile-v2 or v3 schema change is held new against. `held` is every node and edge identity
/// those revisions held, which profile v3 reads at `prior` and holds a new record against.
/// Profile v1 reads neither, and seals and refuses exactly as P1 did.
///
/// # Errors
///
/// The outer error is [`GRAPH_NOT_HELD`] for a restored state without `prior`'s graph; the inner
/// one is the validation verdict.
#[allow(clippy::type_complexity)]
pub(crate) fn validate(
    document: &TransactionDocument,
    revisions: &BTreeMap<RevisionNumber, Revision>,
    held: &HeldIdentities,
    prior: &Revision,
    anchor: &AuthorityStateV1,
    validator: AgentId,
) -> Result<Result<ValidatedTransaction, Vec<crate::ValidationIssue>>, StoreError> {
    let graph = prior.graph()?;
    let lineage = || {
        crate::validate::schema::lineage(
            revisions
                .range(..=prior.root.revision)
                .map(|(_, revision)| &*revision.ontology),
        )
    };
    let pipeline = if anchor.validation_profile.keeps_identities() {
        Pipeline::identity_keeping(validator, lineage(), held.at(prior.root.revision))
    } else if anchor.validation_profile.admits_schema_changes() {
        Pipeline::schema_evolving(validator, lineage())
    } else {
        Pipeline::deterministic(validator)
    };
    let mut aliases = prior
        .alias_holders
        .lock()
        .map_err(|_| refuse("replay-cache-poisoned"))?;
    Ok(pipeline.validate_kept(
        &GraphSnapshot::of(graph),
        document.transaction(),
        &prior.asserted_edges,
        aliases.at(prior.revision_id, prior.root, graph),
    ))
}

/// What [`validate`] answers: the sealed transaction, shared, or every issue raised.
pub(crate) type Verdict = Result<Arc<ValidatedTransaction>, Vec<crate::ValidationIssue>>;

/// The verdict the last validate decision on this thread reached, with every input it is a
/// function of, until the validate command that decided it ends.
///
/// A validate command validates the proposal when it decides the publication, and the store then
/// admits the staged candidate by replaying it through the same kernel, which validates the same
/// document against the same revision under the same profile and validator. The verdict is a pure
/// function of those inputs ([`validate`]): the revision's graph, which its root and allocation
/// name, and the lineage and held identities before it, which its root chains. So the replay reads
/// the verdict the decision reached instead of running the pipeline a second time
/// (`task:validate-builds-one-view-per-command`). The verdict is still the kernel's: this module
/// computed it, from the inputs the key names.
///
/// The prior graph is identified by its allocation, as the remembered root of
/// [`crate::apply`] is: the weak reference keeps that allocation from being reused while it is
/// held, so an equal address is the same graph.
struct Decided {
    prior: std::sync::Weak<ekr_graph::CanonicalGraph>,
    prior_root: ekr_graph::Root,
    document: ContentHash,
    profile: ContentHash,
    validator: AgentId,
    verdict: Verdict,
}
impl Decided {
    fn holds(
        &self,
        document: &TransactionDocument,
        prior: &Revision,
        anchor: &AuthorityStateV1,
        validator: AgentId,
    ) -> bool {
        prior
            .graph
            .as_ref()
            .is_some_and(|graph| std::ptr::eq(self.prior.as_ptr(), Arc::as_ptr(graph)))
            && self.prior_root == prior.root
            && self.document == document.hash()
            && self.validator == validator
            && self.profile == ContentHash::of(&anchor.validation_profile)
    }
}
thread_local! {
    static DECIDED: std::cell::RefCell<Option<Decided>> = const { std::cell::RefCell::new(None) };
}

/// [`validate`] for a validate command deciding its publication. The verdict is left for the
/// replay that admits the publication ([`verdict`] with the same inputs), and for each retry of
/// that admission, which read it rather than validating again.
pub(crate) fn decide_validation(
    document: &TransactionDocument,
    revisions: &BTreeMap<RevisionNumber, Revision>,
    held: &HeldIdentities,
    prior: &Revision,
    anchor: &AuthorityStateV1,
    validator: AgentId,
) -> Result<Verdict, StoreError> {
    let verdict = validate(document, revisions, held, prior, anchor, validator)?.map(Arc::new);
    let graph = prior.graph.as_ref().ok_or_else(|| refuse(GRAPH_NOT_HELD))?;
    DECIDED.with(|decided| {
        *decided.borrow_mut() = Some(Decided {
            prior: Arc::downgrade(graph),
            prior_root: prior.root,
            document: document.hash(),
            profile: ContentHash::of(&anchor.validation_profile),
            validator,
            verdict: verdict.clone(),
        });
    });
    Ok(verdict)
}

/// The verdict of `document` against `prior`: the one the last decision on this thread reached
/// when it had exactly these inputs, and otherwise [`validate`]'s now.
///
/// The decided verdict is shared, not taken: a publication refused as a conflict by an unrelated
/// append is admitted again by a second replay of the same occurrence (design § 91.6), which
/// reads the same verdict. It stays until the command ends ([`ReleaseDecidedValidation`]).
fn verdict(
    document: &TransactionDocument,
    revisions: &BTreeMap<RevisionNumber, Revision>,
    held: &HeldIdentities,
    prior: &Revision,
    anchor: &AuthorityStateV1,
    validator: AgentId,
) -> Result<Verdict, StoreError> {
    let decided = DECIDED.with(|decided| {
        decided
            .borrow()
            .as_ref()
            .filter(|decided| decided.holds(document, prior, anchor, validator))
            .map(|decided| decided.verdict.clone())
    });
    match decided {
        Some(verdict) => Ok(verdict),
        None => Ok(validate(document, revisions, held, prior, anchor, validator)?.map(Arc::new)),
    }
}

/// Releases the verdict a validate decision left for the replays that admit its publication:
/// dropped at the end of a validate command, so a verdict is held only while its publication is in
/// flight.
pub(crate) struct ReleaseDecidedValidation;
impl Drop for ReleaseDecidedValidation {
    fn drop(&mut self) {
        let _ = DECIDED.try_with(|decided| {
            if let Ok(mut decided) = decided.try_borrow_mut() {
                *decided = None;
            }
        });
    }
}
pub(crate) fn validation_record(
    proposal: &ProposalRecordV1,
    proposal_hash: ContentHash,
    validated: &ValidatedTransaction,
    basis: ValidationBasisV1,
    validator: AgentId,
    event_id: EventId,
    at: Timestamp,
) -> ValidationReceiptV1 {
    let tx = validated.transaction();
    let validators = BTreeSet::from([validator]);
    let validation_hash = ContentHash::of(&ValidationMaterialV1 {
        transaction: tx,
        basis: &basis,
        validators: &validators,
    });
    ValidationReceiptV1 {
        format: ValidationReceiptV1::FORMAT.into(),
        event_id,
        proposed_event_id: proposal.event_id,
        proposal_record_hash: proposal_hash,
        transaction_hash: ContentHash::of(tx),
        operations_hash: ContentHash::of(&tx.operations),
        evidence_hash: ContentHash::of(&tx.evidence),
        operation_count: tx.operations.len() as u64,
        basis,
        validators,
        validated_at: at,
        validation_hash,
    }
}
fn read_proposed(
    state: &ReplayState,
    id: TransactionId,
    expected: TransactionState,
) -> Result<&TransactionRecord, StoreError> {
    let tx = state
        .transactions
        .get(&id)
        .ok_or(StoreError::ProposalMissing { transaction_id: id })?;
    require(
        tx.state() == expected,
        "retained-transaction-state-conflict",
    )?;
    Ok(tx)
}
impl KernelAuthority {
    /// Replays `history`, continuing from the longest prefix this authority already reached.
    ///
    /// A reached state holds only the graphs it keeps (see [`Revision`]), and one restored from a
    /// checkpoint only its head's. Should the rest of the history need another earlier graph, the
    /// whole history is replayed from the seed instead, keeping each graph a later occurrence
    /// needs until that occurrence.
    pub(crate) fn reconstruct(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        selected: Option<RevisionNumber>,
    ) -> Result<Option<Arc<ReplayState>>, StoreError> {
        match self.replay_from(history, ontology, selected, true) {
            Err(error) if graph_not_held(&error) => {
                self.replay_from(history, ontology, selected, false)
            }
            result => result,
        }
    }
    /// [`Self::reconstruct`] from the seed, reusing nothing this authority reached before.
    pub(crate) fn reconstruct_in_full(
        &self,
        history: &RetainedHistory,
    ) -> Result<Option<Arc<ReplayState>>, StoreError> {
        self.replay_from(history, None, None, false)
    }
    /// Has every replay keep the graph of revision `number` too, where it passes it, until the
    /// returned guard is dropped. A state a replay continues from that no longer holds the graph
    /// does not regain it. Only the last of two overlapping requests is kept: the other's read
    /// reconstructs its graph instead, which costs a replay and changes no answer.
    pub(crate) fn keeping(&self, number: RevisionNumber) -> Keeping<'_> {
        if let Ok(mut cache) = self.cache.lock() {
            cache.keep = Some(number);
        }
        Keeping(self, number)
    }
    /// The graph of revision `number` of `state`, the state this authority reached over
    /// `history`: the state's own when it holds it, and otherwise reconstructed by a verified
    /// replay of `history` to that revision, which this authority does not keep.
    /// # Errors
    /// A revision `state` does not hold, or any refusal of the replay to it.
    pub(crate) fn graph_at(
        &self,
        history: &RetainedHistory,
        state: &ReplayState,
        number: RevisionNumber,
    ) -> Result<Arc<ekr_graph::CanonicalGraph>, StoreError> {
        let revision = state
            .revisions
            .get(&number)
            .ok_or(StoreError::NoMaterialisedState { requested: number })?;
        if let Some(graph) = &revision.graph {
            return Ok(Arc::clone(graph));
        }
        let reached = self
            .reconstruct(history, None, Some(number))?
            .ok_or(StoreError::NotSeeded)?;
        let head = reached.head();
        require(
            head.root == revision.root,
            "reconstructed-revision-disagrees",
        )?;
        Ok(Arc::clone(
            head.graph.as_ref().ok_or_else(|| refuse(GRAPH_NOT_HELD))?,
        ))
    }
    /// `state`, the state this authority reached over `history`, holding the graph of revision
    /// `number` too ([`Self::graph_at`]).
    ///
    /// The state holding it replaces `state` in this authority's cache, so that the replay
    /// verifying a command's publication against that revision continues from it rather than
    /// from the seed; that replay releases the graph again once no later occurrence needs it. A
    /// revision `state` does not hold, or whose graph it holds, returns `state` itself.
    /// # Errors
    /// Any refusal of the replay to `number`.
    pub(crate) fn holding(
        &self,
        history: &RetainedHistory,
        state: Arc<ReplayState>,
        number: RevisionNumber,
    ) -> Result<Arc<ReplayState>, StoreError> {
        if !state
            .revisions
            .get(&number)
            .is_some_and(|revision| revision.graph.is_none())
        {
            return Ok(state);
        }
        let graph = self.graph_at(history, &state, number)?;
        let mut holding = (*state).clone();
        holding
            .revisions
            .get_mut(&number)
            .expect("revision checked above")
            .graph = Some(graph);
        let holding = Arc::new(holding);
        if let Some(digest) = state.digest {
            self.cache
                .lock()
                .map_err(|_| refuse("replay-cache-poisoned"))?
                .replace(digest, Arc::clone(&holding));
        }
        Ok(holding)
    }
    /// The last stream position, among the occurrences of `history` after the first `start`, at
    /// which each revision is the basis of a validation or rejection: until replay reaches it,
    /// that revision's graph is kept. A record that does not read names nothing here; replay
    /// refuses it by its own name.
    pub(crate) fn bases(history: &RetainedHistory, start: usize) -> BTreeMap<RevisionNumber, u64> {
        let mut bases = BTreeMap::new();
        for occurrence in history.occurrences.iter().skip(start) {
            let basis = match occurrence.event.payload {
                RevisionPayload::TransactionValidated { against, .. } => Some(against),
                RevisionPayload::TransactionRejected { .. } => history
                    .content(occurrence.event.record_hash, StorageClass::Canonical)
                    .ok()
                    .and_then(|bytes| RejectionRecordV1::from_bytes(bytes).ok())
                    .map(|record| record.requested_basis.previous_root.revision),
                _ => None,
            };
            if let Some(basis) = basis {
                bases.insert(basis, occurrence.version);
            }
        }
        bases
    }
    fn replay_from(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        selected: Option<RevisionNumber>,
        reuse: bool,
    ) -> Result<Option<Arc<ReplayState>>, StoreError> {
        crate::migrate::finished(self, history)?;
        // Only the ordinary head replay is shared: a replay under a caller's ontology or to a
        // selected revision is computed in full, as before.
        let shared = ontology.is_none() && selected.is_none();
        let digests = if shared && !history.occurrences.is_empty() {
            self.cache()?.digests(&history.occurrences)
        } else {
            Arc::default()
        };
        let reached = if shared && reuse {
            self.cache
                .lock()
                .map_err(|_| refuse("replay-cache-poisoned"))?
                .longest(&digests)
        } else {
            None
        };
        let reused = reached.is_some();
        let (mut state, start) = if let Some((covered, reached)) = reached {
            // A state reached over exactly this history is this history's state, digest included:
            // it is shared, not copied.
            if covered == history.occurrences.len() && reached.digest == Some(digests[covered]) {
                return Ok(Some(reached));
            }
            // The seed checks bind the host context and anchor; this authority's are immutable.
            ((*reached).clone(), covered)
        } else {
            let Some((seed, seed_payloads)) = self.seed_state(history, ontology)? else {
                return Ok(None);
            };
            self.cache
                .lock()
                .map_err(|_| refuse("replay-cache-poisoned"))?
                .seed_replays += 1;
            let first = &history.occurrences[0];
            let seed_result = SeedResultV1::from_bytes(
                history.content(first.event.record_hash, StorageClass::Canonical)?,
            )?;
            let mut held = HeldIdentities::default();
            if self.anchor.validation_profile.keeps_identities() {
                held.record(
                    RevisionNumber::SEED,
                    seed.graph.nodes.keys().copied(),
                    seed.graph.edges.keys().copied(),
                );
            }
            let state = ReplayState {
                upgraded_authority: None,
                transition: None,
                assessment_validators: BTreeMap::new(),
                held,
                revision_ids: BTreeSet::from([seed_result.revision_id]),
                event_ids: BTreeSet::from([first.event.event_id]),
                issue_ids: BTreeSet::new(),
                seed: seed_result,
                revisions: BTreeMap::from([(RevisionNumber::SEED, Revision::replayed(seed))]),
                transactions: Arc::default(),
                transaction_snapshot: OnceLock::new(),
                documents: BTreeMap::new(),
                validated: BTreeMap::new(),
                version: first.version,
                digest: None,
                seed_payloads,
                earlier_bases: Vec::new(),
            };
            if selected == Some(RevisionNumber::SEED) {
                return Ok(Some(Arc::new(state)));
            }
            (state, 1)
        };
        // The graphs this replay keeps besides the head's: the one a running read asked to keep,
        // the retained checkpoint's, and each revision's a later occurrence is validated against,
        // until that occurrence.
        let (keep, checkpointed) = {
            let cache = self
                .cache
                .lock()
                .map_err(|_| refuse("replay-cache-poisoned"))?;
            (cache.keep, cache.retained.map(|(_, revision)| revision))
        };
        let bases = Self::bases(history, start);
        let kept = |number: RevisionNumber, version: u64| {
            Some(number) == keep
                || Some(number) == checkpointed
                || bases.get(&number).is_some_and(|last| *last > version)
        };
        let version = state.version;
        state.release_all(|number| kept(number, version));
        for occurrence in history.occurrences.iter().skip(start) {
            require(
                occurrence.version == state.version + 1
                    && state.event_ids.insert(occurrence.event.event_id),
                "occurrence-order-or-identity",
            )?;
            let event = &occurrence.event;
            let bytes = history.content(event.record_hash, StorageClass::Canonical)?;
            let active = state
                .upgraded_authority
                .as_ref()
                .unwrap_or(&self.anchor)
                .clone();
            match event.payload {
                RevisionPayload::AuthorityUpgraded { .. } => {
                    crate::upgrade::replay_transition(self, history, &mut state, occurrence)?;
                }
                RevisionPayload::Seeded { .. } => return Err(StoreError::SeedIsNotFirst),
                RevisionPayload::TransactionProposed {
                    transaction_id,
                    proposer,
                    operations_hash,
                } => {
                    let record = ProposalRecordV1::from_bytes(bytes)?;
                    let (expected, document) = parsed_proposal(
                        &record.document_bytes,
                        record.submitter,
                        event.event_id,
                        record.submitted_at,
                        &active,
                        &record.format,
                    )?;
                    require(
                        record == expected
                            && record.transaction_id == transaction_id
                            && record.submitter == proposer
                            && record.canonical_operations_hash == operations_hash,
                        "proposal-record-disagrees",
                    )?;
                    require(
                        !state.transactions.contains_key(&transaction_id),
                        "transaction-identity-reused",
                    )?;
                    state.documents.insert(transaction_id, Arc::new(document));
                    state.transactions_mut().insert(
                        transaction_id,
                        Arc::new(TransactionRecord {
                            proposal: record,
                            proposal_record_hash: event.record_hash,
                            validation: None,
                            validation_record_hash: None,
                            rejection: None,
                            committed: None,
                            stale: None,
                        }),
                    );
                }
                RevisionPayload::TransactionValidated {
                    transaction_id,
                    against,
                    validation_hash,
                } => {
                    let tx = read_proposed(&state, transaction_id, TransactionState::Proposed)?;
                    let record = ValidationReceiptV1::from_bytes(bytes)?;
                    let prior = state
                        .revisions
                        .get(&against)
                        .ok_or_else(|| refuse("validation-basis-absent"))?;
                    require(
                        record.validated_at >= tx.proposal.submitted_at
                            && record.validated_at >= prior.committed_at,
                        "validation-time-order",
                    )?;
                    let validated = verdict(
                        &*state.document(&tx.proposal)?,
                        &state.revisions,
                        &state.held,
                        prior,
                        &active,
                        self.context.validator,
                    )?
                    .map_err(|_| refuse("retained-validation-refused"))?;
                    let expected = validation_record(
                        &tx.proposal,
                        tx.proposal_record_hash,
                        &validated,
                        basis(prior, state.seed.seed_hash, &active),
                        self.context.validator,
                        event.event_id,
                        record.validated_at,
                    );
                    require(
                        record == expected && record.validation_hash == validation_hash,
                        "validation-record-disagrees",
                    )?;
                    state.validated.insert(transaction_id, validated);
                    state.note_basis(occurrence.version, against);
                    state.release(against, |number| kept(number, occurrence.version));
                    let tx = state.transaction_mut(transaction_id);
                    tx.validation = Some(record);
                    tx.validation_record_hash = Some(event.record_hash);
                }
                RevisionPayload::TransactionRejected {
                    transaction_id,
                    issues,
                } => {
                    let tx = read_proposed(&state, transaction_id, TransactionState::Proposed)?;
                    let record = RejectionRecordV1::from_bytes(bytes)?;
                    let prior = state
                        .revisions
                        .get(&record.requested_basis.previous_root.revision)
                        .ok_or_else(|| refuse("rejection-basis-absent"))?;
                    require(
                        record.event_id == event.event_id
                            && record.proposed_event_id == tx.proposal.event_id
                            && record.proposal_record_hash == tx.proposal_record_hash
                            && record.validator == self.context.validator
                            && record.requested_basis
                                == basis(prior, state.seed.seed_hash, &active)
                            && record.rejected_at >= tx.proposal.submitted_at
                            && record.rejected_at >= prior.committed_at,
                        "rejection-record-disagrees",
                    )?;
                    let actual = verdict(
                        &*state.document(&tx.proposal)?,
                        &state.revisions,
                        &state.held,
                        prior,
                        &active,
                        self.context.validator,
                    )?
                    .err()
                    .ok_or_else(|| refuse("rejection-of-valid-transaction"))?;
                    require(
                        !actual.is_empty()
                            && record.issues.len() == actual.len()
                            && usize::try_from(issues).ok() == Some(actual.len()),
                        "rejection-issue-count",
                    )?;
                    for (held, actual) in record.issues.iter().zip(&actual) {
                        require(
                            state.issue_ids.insert(held.id)
                                && held.transaction_id == actual.transaction_id
                                && held.validator == actual.validator
                                && held.code == actual.code
                                && held.message == actual.message,
                            "rejection-issue-disagrees",
                        )?;
                    }
                    let basis = record.requested_basis.previous_root.revision;
                    state.note_basis(occurrence.version, basis);
                    state.release(basis, |number| kept(number, occurrence.version));
                    state.transaction_mut(transaction_id).rejection = Some(record);
                    state.documents.remove(&transaction_id);
                }
                RevisionPayload::RevisionCommitted {
                    transaction_id,
                    revision_id,
                    number,
                    knowledge_root,
                } => {
                    if state.transactions.get(&transaction_id).is_some_and(|tx| {
                        matches!(
                            tx.state(),
                            TransactionState::Proposed
                                | TransactionState::Rejected
                                | TransactionState::Stale
                        )
                    }) {
                        return Err(StoreError::ValidationMissing { transaction_id });
                    }
                    let tx = read_proposed(&state, transaction_id, TransactionState::Validated)?;
                    let record = CommitReceiptV1::from_bytes(bytes)?;
                    let validation = tx.validation.as_ref().expect("validated state");
                    let prior = state.head();
                    require(
                        record.proposal == tx.proposal
                            && record.validation == *validation
                            && Some(record.validation_record_hash) == tx.validation_record_hash
                            && record.event_id == event.event_id
                            && record.revision_id == revision_id
                            && !state.revision_ids.contains(&revision_id),
                        "commit-record-linkage",
                    )?;
                    registered(&active, record.committer)?;
                    require(
                        validation.basis == basis(prior, state.seed.seed_hash, &active),
                        "commit-basis-is-stale",
                    )?;
                    require(
                        record.committed_at >= validation.validated_at,
                        "commit-time-order",
                    )?;
                    // The basis is the revision the retained validation was computed against, so
                    // its sealed result is this validation's result.
                    let validated = match state.validated.get(&transaction_id) {
                        Some(validated) => Arc::clone(validated),
                        None => Arc::new(
                            validate(
                                &*state.document(&tx.proposal)?,
                                &state.revisions,
                                &state.held,
                                prior,
                                &active,
                                self.context.validator,
                            )?
                            .map_err(|_| refuse("retained-commit-validation-refused"))?,
                        ),
                    };
                    // A `/3` receipt names the ids its transaction created (design § 99.5), and
                    // admitting a checkpoint reads them from it instead of parsing the proposal:
                    // replay holds the list to exactly what the transaction creates.
                    if let Some(created) = &record.created {
                        require(
                            *created == crate::CreatedIdentitiesV1::of(validated.transaction()),
                            "commit-created-identities",
                        )?;
                    }
                    let (mut graph, mut root) = crate::apply::apply(
                        prior,
                        &validated,
                        &validation.validators,
                        record.committed_at,
                    )?;
                    let mut assessment_validators = state.assessment_validators.clone();
                    if active.validation_profile.disputes() {
                        crate::disputes::recompute(&mut graph, &mut assessment_validators)?;
                        root.knowledge_root = ekr_store::knowledge_root(&graph);
                    }
                    // Evidence a commit added is retained admissible evidence only while its
                    // payload is retained, at Provenance strength, as the seed's payloads are.
                    for (hash, payload) in crate::commands::added_payloads(validated.transaction())
                    {
                        require(
                            history.content(hash, StorageClass::Provenance)? == payload.as_slice(),
                            "evidence-payload-mismatch",
                        )?;
                    }
                    // Each claim a named refusal describes is held against the recomputed root on
                    // both of its carriers, payload and receipt, before the generic comparison:
                    // a lie told consistently in both must still get its own name.
                    for found in [number, record.result.revision] {
                        if found != root.revision {
                            return Err(StoreError::RevisionOutOfOrder {
                                expected: root.revision,
                                found,
                            });
                        }
                    }
                    for published in [knowledge_root, record.result.knowledge_root] {
                        if published != root.knowledge_root {
                            return Err(StoreError::KnowledgeRootDisagrees {
                                revision: root.revision,
                                published,
                                folded: root.knowledge_root,
                            });
                        }
                    }
                    require(
                        record.result == root && record.result_hash == ContentHash::of(&root),
                        "commit-result-disagrees",
                    )?;
                    let ontology = if root.ontology_root == prior.root.ontology_root {
                        Arc::clone(&prior.ontology)
                    } else {
                        Arc::new(graph.ontology.clone())
                    };
                    let graph_root = prior.graph_root;
                    let asserted_edges = prior.asserted_edges_after(validated.transaction());
                    let alias_holders = Arc::clone(&prior.alias_holders);
                    alias_holders
                        .lock()
                        .map_err(|_| refuse("replay-cache-poisoned"))?
                        .advance(
                            (prior.revision_id, prior.root),
                            (revision_id, root),
                            &graph,
                            validated.transaction(),
                        );
                    let superseded = prior.root.revision;
                    if active.validation_profile.keeps_identities() {
                        state.held.hold(number, validated.transaction());
                    }
                    state.revision_ids.insert(revision_id);
                    state.assessment_validators = assessment_validators;
                    state.revisions.insert(
                        number,
                        Revision {
                            root,
                            revision_id,
                            event_id: event.event_id,
                            record_hash: event.record_hash,
                            committed_at: record.committed_at,
                            graph_root,
                            ontology,
                            graph: Some(Arc::new(graph)),
                            asserted_edges,
                            alias_holders,
                        },
                    );
                    state.release(superseded, |number| kept(number, occurrence.version));
                    state.validated.remove(&transaction_id);
                    state.documents.remove(&transaction_id);
                    state.transaction_mut(transaction_id).committed = Some(record);
                }
                RevisionPayload::TransactionStale {
                    transaction_id,
                    validated_against,
                    current,
                } => {
                    let tx = read_proposed(&state, transaction_id, TransactionState::Validated)?;
                    let record = StaleRecordV1::from_bytes(bytes)?;
                    let validation = tx.validation.as_ref().expect("validated state");
                    // A prepared stale decision can itself encounter later canonical publication.
                    // Its immutable observation must name an actual later retained revision, not
                    // whichever head happens to exist when a provider retry finally succeeds.
                    let observed = state
                        .revisions
                        .get(&record.observed_root.revision)
                        .ok_or_else(|| refuse("stale-observed-revision-absent"))?;
                    require(
                        record.event_id == event.event_id
                            && Some(record.validation_record_hash) == tx.validation_record_hash
                            && record.expected_basis == validation.basis
                            && observed.root.revision > validation.basis.previous_root.revision
                            && validated_against == validation.basis.previous_root.revision
                            && current == observed.root.revision
                            && record.observed_revision_id == observed.revision_id
                            && record.observed_event_id == observed.event_id
                            && record.observed_record_hash == observed.record_hash
                            && record.observed_root == observed.root
                            && record.observed_root_hash == ContentHash::of(&observed.root)
                            && record.stale_at >= validation.validated_at,
                        "stale-record-disagrees",
                    )?;
                    state.transaction_mut(transaction_id).stale = Some(record);
                    state.validated.remove(&transaction_id);
                    state.documents.remove(&transaction_id);
                }
            }
            state.version = occurrence.version;
            if selected.is_some_and(|number| state.head().root.revision == number) {
                return Ok(Some(Arc::new(state)));
            }
        }
        if let Some(requested) = selected {
            return Err(StoreError::NoMaterialisedState { requested });
        }
        let state = if shared {
            let covered = history.occurrences.len();
            state.digest = Some(digests[covered]);
            let state = Arc::new(state);
            if !reused || start < covered {
                self.cache
                    .lock()
                    .map_err(|_| refuse("replay-cache-poisoned"))?
                    .insert_extending(&digests, Arc::clone(&state));
            }
            state
        } else {
            Arc::new(state)
        };
        Ok(Some(state))
    }
}

#[cfg(test)]
mod tests {
    //! One handle serving every command, as `ekr session` does: the graphs its replay cache
    //! holds, and the reads and validations that need a graph it no longer holds.
    use super::ReplayState;
    use crate::{
        Agent, AuthorityStateV1, BootstrapContext, Commit, CommitCommandResult, GraphOperation,
        GraphTransaction, NodeDraft, SeedDocument, ValidationCommandResult, ValidationProfileV1,
    };
    use ekr_core::{NodeId, RevisionNumber, Timestamp, TransactionId, TypeId};
    use ekr_ontology::NodeType;
    use ekr_store::{FileStore, Initialize, Inventory, ObjectStore, RevisionLog, SqliteStore};
    use std::collections::{BTreeMap, BTreeSet};
    use std::path::Path;

    fn context() -> BootstrapContext {
        BootstrapContext {
            operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
            validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
        }
    }
    fn anchor() -> AuthorityStateV1 {
        let c = context();
        AuthorityStateV1 {
            format: "ekr.authority-state/1".into(),
            agents: [(c.operator, "operator"), (c.validator, "validator")]
                .into_iter()
                .map(|(id, name)| {
                    let agent = Agent {
                        id,
                        name: name.into(),
                        capabilities: BTreeSet::new(),
                    };
                    (id, agent)
                })
                .collect(),
            validation_profile: ValidationProfileV1::deterministic(c.validator),
        }
    }
    fn file(path: &Path, full: bool) -> Commit<FileStore> {
        Commit::over_with_authority(context(), anchor(), |authority| {
            let mut store = FileStore::file(path, "session", None)?.under(authority);
            store.set_full_replay(full);
            Ok(store)
        })
        .unwrap()
    }
    fn sqlite(path: &Path, full: bool) -> Commit<SqliteStore> {
        Commit::over_with_authority(context(), anchor(), |authority| {
            let mut store =
                SqliteStore::sqlite(&path.join("state.db"), "session", None)?.under(authority);
            store.set_full_replay(full);
            Ok(store)
        })
        .unwrap()
    }
    fn seed() -> SeedDocument {
        let mut seed =
            SeedDocument::from_yaml(include_str!("../tests/fixtures/seed-minimal-v2.yaml"))
                .unwrap();
        let type_id = "00000000-0000-4000-8000-000000000005".parse().unwrap();
        seed.ontology
            .node_types
            .push(NodeType::new(type_id, "Subject"));
        seed
    }
    /// A transaction creating one node of `type_id`.
    fn document(seed: &SeedDocument, n: u64, type_id: TypeId) -> (TransactionId, Vec<u8>) {
        #[derive(serde::Serialize)]
        struct Wire<'a> {
            format: &'static str,
            transaction: &'a GraphTransaction,
        }
        let tx = GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            operations: vec![GraphOperation::CreateNode(NodeDraft {
                id: NodeId::mint(),
                root_id: seed.graph.root.id,
                type_id,
                canonical_name: format!("subject {n}"),
                properties: BTreeMap::new(),
                aliases: vec![format!("subject {n}")],
            })],
            evidence: BTreeSet::new(),
            schema_version: None,
        };
        let wire = Wire {
            format: "ekr.transaction-document/1",
            transaction: &tx,
        };
        (tx.id, serde_yaml_ng::to_string(&wire).unwrap().into_bytes())
    }
    fn at(n: u64, step: i64) -> Timestamp {
        Timestamp::from_millis(i64::try_from(n * 100).unwrap() + step)
    }
    /// Proposes transaction `n` and validates it against `against`, through `kernel`.
    fn validated<S: RevisionLog + ObjectStore>(
        kernel: &Commit<S>,
        seed: &SeedDocument,
        n: u64,
        type_id: TypeId,
        against: u64,
    ) -> (TransactionId, ValidationCommandResult) {
        let (tx, bytes) = document(seed, n, type_id);
        kernel
            .propose(&bytes, context().operator, || at(n, 0))
            .unwrap();
        let verdict = kernel
            .validate(tx, RevisionNumber::new(against), || at(n, 1))
            .unwrap();
        (tx, verdict)
    }
    /// Commits revision `n`, validated against the head, through `kernel`.
    fn commit<S: RevisionLog + ObjectStore>(kernel: &Commit<S>, seed: &SeedDocument, n: u64) {
        let (tx, verdict) = validated(kernel, seed, n, seed.ontology.node_types[0].id, n - 1);
        assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
        let result = kernel.commit(tx, context().operator, || at(n, 2)).unwrap();
        assert!(matches!(result, CommitCommandResult::Committed(_)));
    }
    /// The revisions whose graph `state` holds.
    fn held(state: &ReplayState) -> Vec<u64> {
        state
            .revisions
            .iter()
            .filter(|(_, revision)| revision.graph.is_some())
            .map(|(number, _)| number.get())
            .collect()
    }
    /// The head of the checkpoint `kernel` knows is retained.
    fn retained<S: RevisionLog + ObjectStore>(kernel: &Commit<S>) -> Option<u64> {
        let retained = kernel.authority.cache.lock().unwrap().retained;
        retained.map(|(_, revision)| revision.get())
    }
    /// What a state `kernel` reached during a command may hold a graph of: its head, and the head
    /// of the checkpoint retained while the command ran, `before` it or after it.
    fn kept<S: RevisionLog + ObjectStore>(
        kernel: &Commit<S>,
        state: &ReplayState,
        before: Option<u64>,
    ) -> Vec<u64> {
        let mut kept: Vec<u64> = [before, retained(kernel)]
            .into_iter()
            .flatten()
            .chain([state.head().root.revision.get()])
            .collect();
        kept.sort_unstable();
        kept.dedup();
        kept
    }
    /// How many distinct graphs every state of `kernel`'s replay cache holds together.
    fn graphs<S: RevisionLog + ObjectStore>(kernel: &Commit<S>) -> usize {
        let cache = kernel.authority.cache.lock().unwrap();
        cache
            .entries
            .iter()
            .flat_map(|(_, _, state)| state.revisions.values())
            .filter_map(|revision| revision.graph.as_ref())
            .map(|graph| std::sync::Arc::as_ptr(graph) as usize)
            .collect::<BTreeSet<_>>()
            .len()
    }

    fn holds_the_head_graph<S: RevisionLog + ObjectStore + Initialize>(
        kernel: &Commit<S>,
        how: &str,
    ) {
        let seed = seed();
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        for n in 1..=10 {
            let before = retained(kernel);
            commit(kernel, &seed, n);
            let state = kernel.read_state().unwrap();
            let held = held(&state);
            assert!(
                held.contains(&n),
                "{how}: after commit {n} the head graph: {held:?}"
            );
            let kept = kept(kernel, &state, before);
            assert!(
                held.iter().all(|number| kept.contains(number)),
                "{how}: after commit {n} graphs held {held:?}, only {kept:?} kept"
            );
            let graphs = graphs(kernel);
            assert!(
                graphs <= 3,
                "{how}: after commit {n} the replay cache holds {graphs} graphs"
            );
        }
    }

    #[test]
    fn one_handle_holds_the_head_graph_and_not_one_graph_per_revision() {
        let directory = tempfile::tempdir().unwrap();
        holds_the_head_graph(&file(directory.path(), false), "file");
        let directory = tempfile::tempdir().unwrap();
        holds_the_head_graph(&sqlite(directory.path(), false), "sqlite");
    }

    #[test]
    fn confirmed_commit_releases_the_previous_head_before_the_next_command() {
        fn check<S: RevisionLog + ObjectStore + Initialize>(kernel: &Commit<S>) {
            let seed = seed();
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();
            commit(kernel, &seed, 1);
            let held = kernel.read_state().unwrap();
            let graph = std::sync::Arc::downgrade(held.head().graph.as_ref().unwrap());
            commit(kernel, &seed, 2);
            assert_ne!(
                retained(kernel),
                Some(1),
                "fixture must not checkpoint the old head"
            );
            assert!(
                graph.upgrade().is_some(),
                "a reader still holds its immutable graph"
            );
            assert_eq!(held.head().root.revision, RevisionNumber::new(1));
            drop(held);
            assert!(
                graph.upgrade().is_none(),
                "the confirmed commit still caches its obsolete head"
            );
        }
        let directory = tempfile::tempdir().unwrap();
        check(&file(directory.path(), false));
        let directory = tempfile::tempdir().unwrap();
        check(&sqlite(directory.path(), false));
    }

    #[test]
    fn a_new_checkpoint_releases_its_predecessors_graph_before_the_next_command() {
        fn check<S: RevisionLog + ObjectStore + Initialize>(kernel: &Commit<S>) {
            let seed = seed();
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();
            for n in 1..=5 {
                commit(kernel, &seed, n);
            }
            assert_eq!(retained(kernel), Some(5));
            let held = kernel.read_state().unwrap();
            let graph = std::sync::Arc::downgrade(held.head().graph.as_ref().unwrap());
            for n in 6..=10 {
                commit(kernel, &seed, n);
            }
            assert_eq!(retained(kernel), Some(10));
            assert_eq!(held.head().root.revision, RevisionNumber::new(5));
            drop(held);
            assert!(
                graph.upgrade().is_none(),
                "the new checkpoint still caches its predecessor's graph"
            );
        }
        let directory = tempfile::tempdir().unwrap();
        check(&file(directory.path(), false));
        let directory = tempfile::tempdir().unwrap();
        check(&sqlite(directory.path(), false));
    }

    #[test]
    fn warm_validation_hashes_no_more_prefix_occurrences_as_history_grows() {
        fn count<S: RevisionLog + ObjectStore + Initialize>(kernel: &Commit<S>, size: u64) -> u64 {
            let seed = seed();
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();
            for n in 1..=size {
                commit(kernel, &seed, n);
            }
            let (id, bytes) = document(&seed, size + 1, seed.ontology.node_types[0].id);
            kernel
                .propose(&bytes, context().operator, || at(size + 1, 0))
                .unwrap();
            let before = super::PREFIX_STEPS.with(std::cell::Cell::get);
            let verdict = kernel
                .validate(id, RevisionNumber::new(size), || at(size + 1, 1))
                .unwrap();
            assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
            super::PREFIX_STEPS.with(std::cell::Cell::get) - before
        }
        for use_file in [false, true] {
            let counts: Vec<_> = [4, 16]
                .into_iter()
                .map(|size| {
                    let directory = tempfile::tempdir().unwrap();
                    if use_file {
                        count(&file(directory.path(), false), size)
                    } else {
                        count(&sqlite(directory.path(), false), size)
                    }
                })
                .collect();
            assert_eq!(
                counts[0], counts[1],
                "file={use_file}: prefix hashing grows with history: {counts:?}"
            );
            assert_eq!(
                counts,
                [1, 1],
                "only the new validation occurrence is hashed"
            );
        }
    }

    fn assertion_seed() -> SeedDocument {
        use ekr_graph::{Confidence, Evidence, EvidenceSource};
        use ekr_ontology::{PropertyDefinition, ValueType};
        let mut seed = seed();
        let property = "00000000-0000-4000-8000-000000000006".parse().unwrap();
        seed.ontology.node_types[0].properties.insert(
            property,
            PropertyDefinition::new(property, "label", ValueType::String),
        );
        let bytes = b"retained assertion provenance".to_vec();
        let hash = ekr_core::ContentHash::of_bytes(&bytes);
        let entry = Evidence {
            id: "00000000-0000-4000-8000-000000000007".parse().unwrap(),
            source: EvidenceSource::HumanStatement {
                identity: Some("operator".into()),
            },
            content_hash: hash,
            extracted_by: context().operator,
            observed_at: Timestamp::EPOCH,
            confidence: Confidence::from_basis_points(10000).unwrap(),
        };
        let mut second = entry.clone();
        second.id = "00000000-0000-4000-8000-000000000008".parse().unwrap();
        seed.graph.evidence.insert(second.id, second);
        seed.graph.evidence.insert(entry.id, entry);
        seed.evidence_payloads.insert(hash, bytes.into());
        seed
    }

    fn assertion_commit<S: RevisionLog + ObjectStore>(
        kernel: &Commit<S>,
        seed: &SeedDocument,
        n: u64,
    ) -> std::sync::Arc<crate::ValidatedTransaction> {
        use ekr_graph::{
            Assertion, AssertionLifecycle, Assessment, Object, Predicate, Subject, TemporalRange,
            TransactionTime,
        };
        let (_, bytes) = document(seed, n, seed.ontology.node_types[0].id);
        let wire: serde_yaml_ng::Value = serde_yaml_ng::from_slice(&bytes).unwrap();
        let mut tx: GraphTransaction =
            serde_yaml_ng::from_value(wire["transaction"].clone()).unwrap();
        let GraphOperation::CreateNode(node) = &tx.operations[0] else {
            unreachable!()
        };
        let evidence = *seed.graph.evidence.keys().next().unwrap();
        tx.operations
            .push(GraphOperation::AddAssertion(Box::new(Assertion {
                id: ekr_core::AssertionId::mint(),
                root_id: seed.graph.root.id,
                subject: Subject::Node(node.id),
                predicate: Predicate::Property(
                    *seed.ontology.node_types[0]
                        .properties
                        .keys()
                        .next()
                        .unwrap(),
                ),
                object: Object::Value(ekr_ontology::Value::String(format!("value {n}"))),
                evidence: BTreeSet::from([evidence]),
                proposed_by: context().operator,
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time: TemporalRange::UNBOUNDED,
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            })));
        tx.evidence.insert(evidence);
        commit_transaction(kernel, tx, n)
    }

    fn commit_transaction<S: RevisionLog + ObjectStore>(
        kernel: &Commit<S>,
        tx: GraphTransaction,
        n: u64,
    ) -> std::sync::Arc<crate::ValidatedTransaction> {
        #[derive(serde::Serialize)]
        struct Wire<'a> {
            format: &'static str,
            transaction: &'a GraphTransaction,
        }
        let wire = Wire {
            format: "ekr.transaction-document/1",
            transaction: &tx,
        };
        kernel
            .propose(
                serde_yaml_ng::to_string(&wire).unwrap().as_bytes(),
                context().operator,
                || at(n, 0),
            )
            .unwrap();
        assert!(matches!(
            kernel
                .validate(tx.id, RevisionNumber::new(n - 1), || at(n, 1))
                .unwrap(),
            ValidationCommandResult::Validated(_)
        ));
        let validated = std::sync::Arc::clone(&kernel.read_state().unwrap().validated[&tx.id]);
        assert!(matches!(
            kernel
                .commit(tx.id, context().operator, || at(n, 2))
                .unwrap(),
            CommitCommandResult::Committed(_)
        ));
        validated
    }

    #[test]
    fn append_commits_copy_no_more_assertions_as_the_graph_grows() {
        fn count<S: RevisionLog + ObjectStore + Initialize>(
            kernel: &Commit<S>,
            size: u64,
        ) -> (usize, usize) {
            let seed = assertion_seed();
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();
            for n in 1..=size {
                assertion_commit(kernel, &seed, n);
            }
            let prior = kernel.read_state().unwrap();
            let assertions = prior.head().graph().unwrap().assertions.len();
            let before = crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get);
            let applied = crate::apply::graphs_applied();
            let validated = assertion_commit(kernel, &seed, size + 1);
            let copied = crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get) - before;
            assert_eq!(crate::apply::graphs_applied() - applied, 1);
            let result = kernel.read_state().unwrap();
            let before_oracle = crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get);
            let (graph, root) = crate::apply::forced_clone(
                prior.head(),
                &validated,
                &BTreeSet::from([context().validator]),
                at(size + 1, 2),
            )
            .unwrap();
            assert_eq!(
                crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get) - before_oracle,
                assertions
            );
            assert_eq!(&graph, result.head().graph().unwrap());
            assert_eq!(root, result.head().root);
            (assertions, copied)
        }
        for use_file in [false, true] {
            let counts: Vec<_> = [4, 24]
                .into_iter()
                .map(|size| {
                    let directory = tempfile::tempdir().unwrap();
                    if use_file {
                        count(&file(directory.path(), false), size)
                    } else {
                        count(&sqlite(directory.path(), false), size)
                    }
                })
                .collect();
            assert!(
                counts[1].0 >= 4 * counts[0].0,
                "actual assertion totals: {counts:?}"
            );
            assert_eq!(
                counts[0].1, counts[1].1,
                "file={use_file}: full graph clone grows: {counts:?}"
            );
            assert_eq!(
                counts[0].1, 0,
                "exclusive confirmed predecessor is reusable"
            );
        }
    }

    #[test]
    fn reusable_graph_belongs_to_the_authority_not_the_thread() {
        let directory = tempfile::tempdir().unwrap();
        let kernel = sqlite(directory.path(), false);
        let seed = assertion_seed();
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        for n in 1..=4 {
            assertion_commit(&kernel, &seed, n);
        }
        let owner = {
            let cache = kernel.authority.cache.lock().unwrap();
            assert!(cache.retired.lock().unwrap().is_some());
            std::sync::Arc::downgrade(&cache.retired)
        };
        drop(kernel);
        assert!(
            owner.upgrade().is_none(),
            "thread locator must not keep retired graph alive"
        );
    }

    #[test]
    fn readers_and_checkpoint_graphs_prevent_reuse_extraction() {
        fn check<S: RevisionLog + ObjectStore + Initialize>(kernel: &Commit<S>) {
            let seed = assertion_seed();
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();
            for n in 1..=2 {
                assertion_commit(kernel, &seed, n);
            }
            let held = kernel.read_state().unwrap();
            let frozen = held.head().graph().unwrap().clone();
            assertion_commit(kernel, &seed, 3);
            assert!(kernel
                .authority
                .cache
                .lock()
                .unwrap()
                .retired
                .lock()
                .unwrap()
                .is_none());
            let before = crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get);
            assertion_commit(kernel, &seed, 4);
            assert_eq!(
                crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get) - before,
                3
            );
            assert_eq!(held.head().graph().unwrap(), &frozen);
            drop(held);
            for n in 5..=6 {
                assertion_commit(kernel, &seed, n);
            }
            assert_eq!(retained(kernel), Some(5));
            assert!(kernel
                .authority
                .cache
                .lock()
                .unwrap()
                .retired
                .lock()
                .unwrap()
                .is_none());
            let before = crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get);
            assertion_commit(kernel, &seed, 7);
            assert_eq!(
                crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get) - before,
                6
            );
        }
        let directory = tempfile::tempdir().unwrap();
        check(&file(directory.path(), false));
        let directory = tempfile::tempdir().unwrap();
        check(&sqlite(directory.path(), false));
    }

    #[test]
    fn reused_graph_preserves_inherited_attachments_and_matches_forced_clone() {
        fn check<S: RevisionLog + ObjectStore + Initialize>(kernel: &Commit<S>) {
            let seed = assertion_seed();
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();
            assertion_commit(kernel, &seed, 1);
            let assertion = *kernel
                .read_state()
                .unwrap()
                .head()
                .graph()
                .unwrap()
                .assertions
                .keys()
                .next()
                .unwrap();
            let evidence = *seed.graph.evidence.keys().last().unwrap();
            commit_transaction(
                kernel,
                GraphTransaction {
                    id: TransactionId::mint(),
                    proposer: context().operator,
                    operations: vec![GraphOperation::AttachEvidence(crate::EvidenceAttachment {
                        assertion,
                        evidence,
                    })],
                    evidence: BTreeSet::from([evidence]),
                    schema_version: None,
                },
                2,
            );
            assert!(
                kernel
                    .authority
                    .cache
                    .lock()
                    .unwrap()
                    .retired
                    .lock()
                    .unwrap()
                    .is_none(),
                "nonappend publication cannot seed reuse"
            );
            for n in 3..=4 {
                assertion_commit(kernel, &seed, n);
            }
            let prior = kernel.read_state().unwrap();
            let before = crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get);
            let validated = assertion_commit(kernel, &seed, 5);
            assert_eq!(
                crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get) - before,
                0
            );
            let result = kernel.read_state().unwrap();
            assert_eq!(result.head().graph().unwrap().attachments.len(), 1);
            let (graph, root) = crate::apply::forced_clone(
                prior.head(),
                &validated,
                &BTreeSet::from([context().validator]),
                at(5, 2),
            )
            .unwrap();
            assert_eq!(&graph, result.head().graph().unwrap());
            assert_eq!(root, result.head().root);
        }
        let directory = tempfile::tempdir().unwrap();
        check(&file(directory.path(), false));
        let directory = tempfile::tempdir().unwrap();
        check(&sqlite(directory.path(), false));
    }

    #[test]
    fn an_unpublished_candidate_cannot_seed_reuse() {
        fn check<S: RevisionLog + ObjectStore + Initialize>(kernel: &Commit<S>) {
            let seed = assertion_seed();
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();
            for n in 1..=4 {
                assertion_commit(kernel, &seed, n);
            }
            let (id, verdict) = validated(kernel, &seed, 5, seed.ontology.node_types[0].id, 4);
            assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
            let prior = kernel.read_state().unwrap();
            let before = crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get);
            crate::apply::decide(
                prior.head(),
                &prior.validated[&id],
                &BTreeSet::from([context().validator]),
                at(5, 2),
            )
            .unwrap();
            assert_eq!(
                crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get) - before,
                0
            );
            // The command ends before publication: no confirmed prefix can authorize this graph.
            drop(crate::apply::ReleaseDecided);
            assert!(!crate::apply::decided_graph_held());
            assert!(kernel
                .authority
                .cache
                .lock()
                .unwrap()
                .retired
                .lock()
                .unwrap()
                .is_none());
            let before = crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get);
            assert!(matches!(
                kernel.commit(id, context().operator, || at(5, 2)).unwrap(),
                CommitCommandResult::Committed(_)
            ));
            assert_eq!(
                crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get) - before,
                4,
                "retry must clone after an unconfirmed candidate consumed scratch"
            );
        }
        let directory = tempfile::tempdir().unwrap();
        check(&file(directory.path(), false));
        let directory = tempfile::tempdir().unwrap();
        check(&sqlite(directory.path(), false));
    }

    #[test]
    fn an_equal_root_replayed_by_another_authority_falls_back_to_clone() {
        for use_file in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            fn check<S: RevisionLog + ObjectStore + Initialize>(
                kernel: &Commit<S>,
                peer: &Commit<S>,
            ) {
                let seed = assertion_seed();
                kernel
                    .seed(seed.clone(), || Timestamp::from_millis(10))
                    .unwrap();
                for n in 1..=4 {
                    assertion_commit(kernel, &seed, n);
                }
                let (id, verdict) = validated(kernel, &seed, 5, seed.ontology.node_types[0].id, 4);
                assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
                let original = kernel.read_state().unwrap();
                // Preserve real confirmed scratch across the diagnostic full replay so the
                // equal-root allocation guard itself, rather than an empty slot, is exercised.
                let saved = kernel
                    .authority
                    .cache
                    .lock()
                    .unwrap()
                    .retired
                    .lock()
                    .unwrap()
                    .take();
                assert!(saved.is_some());
                let replayed = peer.read_state().unwrap();
                *kernel
                    .authority
                    .cache
                    .lock()
                    .unwrap()
                    .retired
                    .lock()
                    .unwrap() = saved;
                assert_eq!(original.head().root, replayed.head().root);
                assert!(!std::sync::Arc::ptr_eq(
                    original.head().graph.as_ref().unwrap(),
                    replayed.head().graph.as_ref().unwrap()
                ));
                let before = crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get);
                let result = crate::apply::apply(
                    replayed.head(),
                    &original.validated[&id],
                    &BTreeSet::from([context().validator]),
                    at(5, 2),
                )
                .unwrap();
                assert_eq!(
                    crate::apply::ASSERTIONS_COPIED.with(std::cell::Cell::get) - before,
                    4
                );
                let oracle = crate::apply::forced_clone(
                    replayed.head(),
                    &original.validated[&id],
                    &BTreeSet::from([context().validator]),
                    at(5, 2),
                )
                .unwrap();
                assert_eq!(result, oracle);
            }
            if use_file {
                check(
                    &file(directory.path(), false),
                    &file(directory.path(), true),
                );
            } else {
                check(
                    &sqlite(directory.path(), false),
                    &sqlite(directory.path(), true),
                );
            }
        }
    }

    #[test]
    fn prefix_hash_memo_compares_every_occurrence_and_preserves_held_digest_vectors() {
        let directory = tempfile::tempdir().unwrap();
        let kernel = sqlite(directory.path(), false);
        let seed = seed();
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        for n in 1..=3 {
            commit(&kernel, &seed, n);
        }
        let mut occurrences = kernel.store.history().unwrap().occurrences;
        let expected = super::prefix_digests(&occurrences);
        let mut cache = super::ReplayCache::default();
        let held = cache.digests(&occurrences);
        assert_eq!(*held, expected);
        for occurrence in &mut occurrences {
            occurrence.provider_event_id.push_str(" native identity");
        }
        let before = super::PREFIX_STEPS.with(std::cell::Cell::get);
        assert_eq!(*cache.digests(&occurrences), expected);
        assert_eq!(super::PREFIX_STEPS.with(std::cell::Cell::get) - before, 0);
        for index in [0, occurrences.len() / 2, occurrences.len() - 1] {
            let prior = occurrences[index].event.record_hash;
            occurrences[index].event.record_hash =
                ekr_core::ContentHash::of("changed retained record");
            let actual = cache.digests(&occurrences);
            assert_eq!(*actual, super::prefix_digests(&occurrences));
            assert_ne!(*actual, expected);
            assert_eq!(
                *held, expected,
                "a later memo update changed a held digest vector"
            );
            occurrences[index].event.record_hash = prior;
            assert_eq!(*cache.digests(&occurrences), expected);
            let prior_version = occurrences[index].version;
            occurrences[index].version += 1;
            let actual = cache.digests(&occurrences);
            assert_eq!(*actual, super::prefix_digests(&occurrences));
            assert_ne!(*actual, expected, "stream position is independently bound");
            occurrences[index].version = prior_version;
            assert_eq!(*cache.digests(&occurrences), expected);
        }
        let shorter = occurrences.len() / 2;
        assert_eq!(
            *cache.digests(&occurrences[..shorter]),
            expected[..=shorter]
        );
        assert_eq!(*cache.digests(&occurrences), expected);
        assert_eq!(*cache.digests(&[]), super::prefix_digests(&[]));
        assert_eq!(*held, expected);
    }

    #[test]
    fn node_only_commits_share_the_unchanged_assertion_edge_index() {
        fn check<S: RevisionLog + ObjectStore + Initialize>(kernel: &Commit<S>) {
            let seed = seed();
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();
            commit(kernel, &seed, 1);
            let before = crate::validate::edge_indexes_built();
            let (_, verdict) = validated(kernel, &seed, 2, seed.ontology.node_types[0].id, 1);
            assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
            assert_eq!(
                crate::validate::edge_indexes_built() - before,
                0,
                "a node-only commit rebuilt an unchanged edge-assertion index"
            );
        }
        let directory = tempfile::tempdir().unwrap();
        check(&file(directory.path(), false));
        let directory = tempfile::tempdir().unwrap();
        check(&sqlite(directory.path(), false));
    }

    #[test]
    fn retirement_requires_the_confirmed_occurrence_and_matching_prefix() {
        let directory = tempfile::tempdir().unwrap();
        let kernel = sqlite(directory.path(), false);
        let seed = seed();
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let (id, _) = validated(&kernel, &seed, 1, seed.ontology.node_types[0].id, 0);
        let before = kernel.read_state().unwrap();
        kernel.commit(id, context().operator, || at(1, 2)).unwrap();
        let after = kernel.read_state().unwrap();
        let history = kernel.store.history().unwrap();
        let publication = ekr_store::Publication {
            expected_version: before.version,
            event: history.occurrences.last().unwrap().event.clone(),
            objects: BTreeMap::new(),
        };
        let mut cache = super::ReplayCache::default();
        let prior = (before.version as usize, before.digest.unwrap());
        cache.insert(prior.0, prior.1, before.clone());
        cache.insert(after.version as usize, after.digest.unwrap(), after.clone());
        // Another branch at the same stream version is not a prefix of this publication.
        let unrelated = ekr_core::ContentHash::of("another verified history");
        cache.insert(prior.0, unrelated, before);
        let mut different = publication.clone();
        different.event.event_id = ekr_core::EventId::mint();
        cache.confirmed(&different);
        assert_eq!(
            cache.entries.len(),
            3,
            "different occurrence retired a predecessor"
        );
        different = publication.clone();
        different.expected_version += 1;
        cache.confirmed(&different);
        assert_eq!(
            cache.entries.len(),
            3,
            "different position retired a predecessor"
        );
        cache.confirmed(&publication);
        assert_eq!(cache.entries.len(), 2);
        assert!(cache
            .entries
            .iter()
            .all(|(covered, digest, _)| (*covered, *digest) != prior));
        assert!(cache
            .entries
            .iter()
            .any(|(_, digest, _)| *digest == unrelated));
        assert!(cache
            .entries
            .iter()
            .any(|(_, digest, _)| Some(*digest) == after.digest));
    }

    #[test]
    fn validating_one_transaction_does_not_copy_prior_document_buffers() {
        fn copied<S: RevisionLog + ObjectStore + Initialize>(
            kernel: &Commit<S>,
            count: u64,
        ) -> usize {
            let seed = seed();
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();
            for n in 1..=count {
                commit(kernel, &seed, n);
            }
            let before = kernel.read_state().unwrap();
            let (id, bytes) = document(&seed, count + 1, seed.ontology.node_types[0].id);
            kernel
                .propose(&bytes, context().operator, || at(count + 1, 0))
                .unwrap();
            assert_eq!(
                kernel.transaction_states([id]).unwrap()[&id],
                super::TransactionState::Proposed
            );
            let verdict = kernel
                .validate(id, RevisionNumber::new(count), || at(count + 1, 1))
                .unwrap();
            assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
            assert_eq!(
                kernel.transaction_states([id]).unwrap()[&id],
                super::TransactionState::Validated
            );
            let after = kernel.read_state().unwrap();
            assert!(
                after.transaction_snapshot.get().is_none(),
                "selected state reads materialized all retained transaction records"
            );
            assert_eq!(
                after.transactions.get(&id).unwrap().state(),
                super::TransactionState::Validated
            );
            before
                .transactions
                .iter()
                .filter(|(id, record)| {
                    let later = after.transactions.get(id).unwrap();
                    assert_eq!(*record, later);
                    record.proposal.document_bytes.as_ptr()
                        != later.proposal.document_bytes.as_ptr()
                })
                .count()
        }
        for file_provider in [false, true] {
            let mut counts = Vec::new();
            for size in [4, 16] {
                let directory = tempfile::tempdir().unwrap();
                let count = if file_provider {
                    copied(&file(directory.path(), false), size)
                } else {
                    copied(&sqlite(directory.path(), false), size)
                };
                counts.push(count);
            }
            assert_eq!(
                counts,
                [0, 0],
                "file={file_provider}: prior document buffers copied"
            );
        }
    }

    #[test]
    fn warm_alias_checks_visit_no_unchanged_nodes() {
        let mut visits = Vec::new();
        for size in [4, 16] {
            let directory = tempfile::tempdir().unwrap();
            let kernel = sqlite(directory.path(), false);
            let seed = seed();
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();
            for n in 1..=size {
                commit(&kernel, &seed, n);
            }
            let before = crate::validate::ALIAS_NODES_VISITED.with(std::cell::Cell::get);
            let (_, result) = validated(
                &kernel,
                &seed,
                size + 1,
                seed.ontology.node_types[0].id,
                size,
            );
            assert!(matches!(result, ValidationCommandResult::Validated(_)));
            visits.push(crate::validate::ALIAS_NODES_VISITED.with(std::cell::Cell::get) - before);
        }
        assert_eq!(
            visits,
            [0, 0],
            "unchanged canonical nodes scanned for aliases"
        );
    }

    /// Seeds and commits six revisions through one handle, then validates one transaction against
    /// revision 2 and rejects another against revision 1, both earlier than the head, and commits
    /// the first, which is stale. Every decision equals what a full replay from the seed derives,
    /// the handle again holds only the graphs it keeps, and the store migrates.
    fn validates_against_an_earlier_revision<S, D>(
        kernel: &Commit<S>,
        in_full: impl Fn() -> Commit<S>,
        destination: impl FnOnce() -> Commit<D>,
        how: &str,
    ) where
        S: RevisionLog + ObjectStore + Initialize + Inventory,
        D: RevisionLog + ObjectStore + Initialize + Inventory,
    {
        let seed = seed();
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        for n in 1..=6 {
            commit(kernel, &seed, n);
        }
        let subject = seed.ontology.node_types[0].id;
        let (tx, verdict) = validated(kernel, &seed, 7, subject, 2);
        let ValidationCommandResult::Validated(receipt) = verdict else {
            panic!("{how}: {verdict:?}");
        };
        assert_eq!(receipt.basis.previous_root.revision, RevisionNumber::new(2));
        let unknown = "00000000-0000-4000-8000-0000000000ff".parse().unwrap();
        let (_, verdict) = validated(kernel, &seed, 8, unknown, 1);
        let ValidationCommandResult::Rejected(rejection) = verdict else {
            panic!("{how}: {verdict:?}");
        };
        assert_eq!(
            rejection.requested_basis.previous_root.revision,
            RevisionNumber::new(1)
        );
        let result = kernel.commit(tx, context().operator, || at(9, 0)).unwrap();
        assert!(matches!(result, CommitCommandResult::Stale(_)), "{how}");

        let state = kernel.read_state().unwrap();
        let (held, kept) = (held(&state), kept(kernel, &state, retained(kernel)));
        assert!(
            held.iter().all(|number| kept.contains(number)),
            "{how}: graphs held {held:?}, only {kept:?} kept"
        );
        let in_full = in_full();
        let replayed = in_full.read_state().unwrap();
        assert_eq!(state.transactions, replayed.transactions, "{how}");
        assert_eq!(state.head().root, replayed.head().root, "{how}");
        for number in 0..=6 {
            let number = RevisionNumber::new(number);
            assert_eq!(
                kernel.schema_history(number).unwrap(),
                in_full.schema_history(number).unwrap(),
                "{how}: revision {number}"
            );
        }
        kernel.migrate_into(&destination()).unwrap();
    }

    #[test]
    fn one_handle_validates_against_an_earlier_revision_whose_graph_it_released() {
        let directory = tempfile::tempdir().unwrap();
        let (path, into) = (directory.path(), tempfile::tempdir().unwrap());
        validates_against_an_earlier_revision(
            &file(path, false),
            || file(path, true),
            || file(into.path(), false),
            "file",
        );
        let directory = tempfile::tempdir().unwrap();
        let (path, into) = (directory.path(), tempfile::tempdir().unwrap());
        validates_against_an_earlier_revision(
            &sqlite(path, false),
            || sqlite(path, true),
            || sqlite(into.path(), false),
            "sqlite",
        );
    }
}
