//! Complete retained decision verification; the same pure authority checks new publications.
use crate::{
    AuthorityStateV1, CommitReceiptV1, GraphOperation, GraphTransaction, KernelAuthority, Pipeline,
    ProposalRecordV1, RejectionRecordV1, SeedResultV1, StaleRecordV1, TransactionDocument,
    ValidatedTransaction, ValidationBasisV1, ValidationMaterialV1, ValidationReceiptV1,
};
use ekr_core::{
    AgentId, ContentHash, EventId, IssueId, RevisionId, RevisionNumber, Timestamp, TransactionId,
};
use ekr_graph::{CanonicalValue, GraphSnapshot, RevisionPayload};
use ekr_store::{AdmittedRevision, RetainedHistory, StorageClass, StoreError};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

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
pub(crate) struct ReplayState {
    pub(crate) seed: SeedResultV1,
    pub(crate) revisions: BTreeMap<RevisionNumber, AdmittedRevision>,
    pub(crate) transactions: BTreeMap<TransactionId, TransactionRecord>,
    pub(crate) version: u64,
}
impl ReplayState {
    pub(crate) fn head(&self) -> &AdmittedRevision {
        self.revisions
            .last_key_value()
            .expect("state is constructed with verified seed")
            .1
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
    Ok(ProposalRecordV1 {
        format: ProposalRecordV1::FORMAT.into(),
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
    })
}
pub(crate) fn basis(
    prior: &AdmittedRevision,
    seed_hash: ContentHash,
    anchor: &AuthorityStateV1,
) -> ValidationBasisV1 {
    ValidationBasisV1 {
        format: ValidationBasisV1::FORMAT.into(),
        graph_root_id: prior.graph.root.id,
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
pub(crate) fn validate(
    proposal: &ProposalRecordV1,
    prior: &AdmittedRevision,
    validator: AgentId,
) -> Result<ValidatedTransaction, Vec<crate::ValidationIssue>> {
    // Proposal verification has already parsed these exact retained bytes under their frozen profile.
    let parsed =
        TransactionDocument::parse(&proposal.document_bytes).expect("verified proposal document");
    Pipeline::deterministic(validator)
        .validate(&GraphSnapshot::of(&prior.graph), parsed.transaction())
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
    pub(crate) fn reconstruct(
        &self,
        history: &RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        selected: Option<RevisionNumber>,
    ) -> Result<Option<ReplayState>, StoreError> {
        let Some(seed) = self.seed_state(history, ontology)? else {
            return Ok(None);
        };
        let first = &history.occurrences[0];
        let seed_result = SeedResultV1::from_bytes(
            history.content(first.event.record_hash, StorageClass::Canonical)?,
        )?;
        let mut state = ReplayState {
            seed: seed_result,
            revisions: BTreeMap::from([(RevisionNumber::SEED, seed)]),
            transactions: BTreeMap::new(),
            version: first.version,
        };
        if selected == Some(RevisionNumber::SEED) {
            return Ok(Some(state));
        }
        let mut revisions = BTreeSet::<RevisionId>::from([state.seed.revision_id]);
        let mut events = BTreeSet::from([first.event.event_id]);
        let mut issue_ids = BTreeSet::<IssueId>::new();
        for occurrence in history.occurrences.iter().skip(1) {
            require(
                occurrence.version == state.version + 1 && events.insert(occurrence.event.event_id),
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
                    let expected = proposal(
                        &record.document_bytes,
                        record.submitter,
                        event.event_id,
                        record.submitted_at,
                        &self.anchor,
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
                    let validated = validate(&tx.proposal, prior, self.context.validator)
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
                    let actual = validate(&tx.proposal, prior, self.context.validator)
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
                            issue_ids.insert(held.id)
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
                            && revisions.insert(revision_id),
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
                    let validated = validate(&tx.proposal, prior, self.context.validator)
                        .map_err(|_| refuse("retained-commit-validation-refused"))?;
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
                    state.revisions.insert(
                        number,
                        AdmittedRevision {
                            graph,
                            root,
                            revision_id,
                            event_id: event.event_id,
                            record_hash: event.record_hash,
                            committed_at: record.committed_at,
                        },
                    );
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
        Ok(Some(state))
    }
}
