//! Complete retained decision verification; the same pure authority checks new publications.
use crate::{
    AuthorityStateV1, CommitReceiptV1, GraphOperation, GraphTransaction, KernelAuthority, Pipeline,
    ProposalRecordV1, RejectionRecordV1, SeedResultV1, StaleRecordV1, TransactionDocument,
    ValidatedTransaction, ValidationBasisV1, ValidationMaterialV1, ValidationReceiptV1,
};
use ekr_core::{
    AgentId, Canonical, ContentHash, Encoder, EventId, IssueId, RevisionId, RevisionNumber,
    Timestamp, TransactionId,
};
use ekr_graph::{CanonicalValue, GraphSnapshot, RevisionPayload};
use ekr_store::{AdmittedRevision, RecordedOccurrence, RetainedHistory, StorageClass, StoreError};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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
/// validation is shared, and only the retained records themselves are copied.
#[derive(Clone)]
pub(crate) struct ReplayState {
    pub(crate) seed: SeedResultV1,
    pub(crate) revisions: BTreeMap<RevisionNumber, Revision>,
    pub(crate) transactions: BTreeMap<TransactionId, TransactionRecord>,
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
}
/// The refusal a state restored from a checkpoint gives for a graph it does not hold. It is never
/// a verdict about the history: whoever meets it replays that history in full instead.
pub(crate) const GRAPH_NOT_HELD: &str = "replay-graph-not-held";

/// One committed revision as replay holds it: its verified coordinates always, its graph when the
/// state reached it by replay. A state restored from a checkpoint holds only the head's graph.
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
    pub(crate) fn head(&self) -> &Revision {
        self.revisions
            .last_key_value()
            .expect("state is constructed with verified seed")
            .1
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
pub(crate) fn prefix_digests(occurrences: &[RecordedOccurrence]) -> Vec<ContentHash> {
    struct Step<'a>(ContentHash, &'a RecordedOccurrence);
    impl Canonical for Step<'_> {
        fn encode(&self, out: &mut Encoder) {
            "ekr.replay-prefix/1".encode(out);
            self.0.encode(out);
            self.1.version.encode(out);
            self.1.event.encode(out);
        }
    }
    let mut digests = Vec::with_capacity(occurrences.len() + 1);
    let mut digest = ContentHash::of("ekr.replay-prefix/1");
    digests.push(digest);
    for occurrence in occurrences {
        digest = ContentHash::of(&Step(digest, occurrence));
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
    /// The seed envelope this authority admitted, and the evidence payloads it requires.
    pub(crate) seed: Option<(ContentHash, BTreeSet<ContentHash>)>,
}
impl ReplayCache {
    const CAPACITY: usize = 4;
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
    require(
        tx.proposer == actor
            && tx
                .operations
                .iter()
                .all(|op| !matches!(op,GraphOperation::AddAssertion(a) if a.proposed_by!=actor)),
        "proposal-attribution-mismatch",
    )?;
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
/// profile-v2 schema change is held new against. Profile v1 reads none of them, and seals and
/// refuses exactly as P1 did.
///
/// # Errors
///
/// The outer error is [`GRAPH_NOT_HELD`] for a restored state without `prior`'s graph; the inner
/// one is the validation verdict.
#[allow(clippy::type_complexity)]
pub(crate) fn validate(
    document: &TransactionDocument,
    revisions: &BTreeMap<RevisionNumber, Revision>,
    prior: &Revision,
    anchor: &AuthorityStateV1,
    validator: AgentId,
) -> Result<Result<ValidatedTransaction, Vec<crate::ValidationIssue>>, StoreError> {
    let graph = prior.graph()?;
    let pipeline = if anchor.validation_profile.admits_schema_changes() {
        Pipeline::schema_evolving(
            validator,
            crate::validate::schema::lineage(
                revisions
                    .range(..=prior.root.revision)
                    .map(|(_, revision)| &*revision.ontology),
            ),
        )
    } else {
        Pipeline::deterministic(validator)
    };
    Ok(pipeline.validate(&GraphSnapshot::of(graph), document.transaction()))
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
    /// A reached state restored from a checkpoint holds only its head's graph. Should the rest of
    /// the history need an earlier graph, the whole history is replayed from the seed instead.
    pub(crate) fn reconstruct(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        selected: Option<RevisionNumber>,
    ) -> Result<Option<ReplayState>, StoreError> {
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
    ) -> Result<Option<ReplayState>, StoreError> {
        self.replay_from(history, None, None, false)
    }
    fn replay_from(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        selected: Option<RevisionNumber>,
        reuse: bool,
    ) -> Result<Option<ReplayState>, StoreError> {
        // Only the ordinary head replay is shared: a replay under a caller's ontology or to a
        // selected revision is computed in full, as before.
        let shared = ontology.is_none() && selected.is_none();
        let digests = if shared && !history.occurrences.is_empty() {
            prefix_digests(&history.occurrences)
        } else {
            Vec::new()
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
            // The seed checks bind the host context and anchor; this authority's are immutable.
            ((*reached).clone(), covered)
        } else {
            let Some((seed, seed_payloads)) = self.seed_state(history, ontology)? else {
                return Ok(None);
            };
            let first = &history.occurrences[0];
            let seed_result = SeedResultV1::from_bytes(
                history.content(first.event.record_hash, StorageClass::Canonical)?,
            )?;
            let state = ReplayState {
                revision_ids: BTreeSet::from([seed_result.revision_id]),
                event_ids: BTreeSet::from([first.event.event_id]),
                issue_ids: BTreeSet::new(),
                seed: seed_result,
                revisions: BTreeMap::from([(RevisionNumber::SEED, Revision::replayed(seed))]),
                transactions: BTreeMap::new(),
                documents: BTreeMap::new(),
                validated: BTreeMap::new(),
                version: first.version,
                digest: None,
                seed_payloads,
            };
            if selected == Some(RevisionNumber::SEED) {
                return Ok(Some(state));
            }
            (state, 1)
        };
        for occurrence in history.occurrences.iter().skip(start) {
            require(
                occurrence.version == state.version + 1
                    && state.event_ids.insert(occurrence.event.event_id),
                "occurrence-order-or-identity",
            )?;
            let event = &occurrence.event;
            let bytes = history.content(event.record_hash, StorageClass::Canonical)?;
            match event.payload {
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
                        &self.anchor,
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
                    state.transactions.insert(
                        transaction_id,
                        TransactionRecord {
                            proposal: record,
                            proposal_record_hash: event.record_hash,
                            validation: None,
                            validation_record_hash: None,
                            rejection: None,
                            committed: None,
                            stale: None,
                        },
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
                    let validated = validate(
                        &*state.document(&tx.proposal)?,
                        &state.revisions,
                        prior,
                        &self.anchor,
                        self.context.validator,
                    )?
                    .map_err(|_| refuse("retained-validation-refused"))?;
                    let expected = validation_record(
                        &tx.proposal,
                        tx.proposal_record_hash,
                        &validated,
                        basis(prior, state.seed.seed_hash, &self.anchor),
                        self.context.validator,
                        event.event_id,
                        record.validated_at,
                    );
                    require(
                        record == expected && record.validation_hash == validation_hash,
                        "validation-record-disagrees",
                    )?;
                    state.validated.insert(transaction_id, Arc::new(validated));
                    let tx = state
                        .transactions
                        .get_mut(&transaction_id)
                        .expect("verified transaction");
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
                                == basis(prior, state.seed.seed_hash, &self.anchor)
                            && record.rejected_at >= tx.proposal.submitted_at
                            && record.rejected_at >= prior.committed_at,
                        "rejection-record-disagrees",
                    )?;
                    let actual = validate(
                        &*state.document(&tx.proposal)?,
                        &state.revisions,
                        prior,
                        &self.anchor,
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
                    state
                        .transactions
                        .get_mut(&transaction_id)
                        .expect("verified transaction")
                        .rejection = Some(record);
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
                    registered(&self.anchor, record.committer)?;
                    require(
                        validation.basis == basis(prior, state.seed.seed_hash, &self.anchor),
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
                                prior,
                                &self.anchor,
                                self.context.validator,
                            )?
                            .map_err(|_| refuse("retained-commit-validation-refused"))?,
                        ),
                    };
                    let (graph, root) = crate::apply::apply(
                        prior,
                        &validated,
                        &validation.validators,
                        record.committed_at,
                    )?;
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
                    state.revision_ids.insert(revision_id);
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
                        },
                    );
                    state.validated.remove(&transaction_id);
                    state.documents.remove(&transaction_id);
                    state
                        .transactions
                        .get_mut(&transaction_id)
                        .expect("verified transaction")
                        .committed = Some(record);
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
                    state
                        .transactions
                        .get_mut(&transaction_id)
                        .expect("verified transaction")
                        .stale = Some(record);
                    state.validated.remove(&transaction_id);
                    state.documents.remove(&transaction_id);
                }
            }
            state.version = occurrence.version;
            if selected.is_some_and(|number| state.head().root.revision == number) {
                return Ok(Some(state));
            }
        }
        if let Some(requested) = selected {
            return Err(StoreError::NoMaterialisedState { requested });
        }
        if shared {
            let covered = history.occurrences.len();
            state.digest = Some(digests[covered]);
            if !reused || start < covered {
                self.cache
                    .lock()
                    .map_err(|_| refuse("replay-cache-poisoned"))?
                    .insert(covered, digests[covered], Arc::new(state.clone()));
            }
        }
        Ok(Some(state))
    }
}
