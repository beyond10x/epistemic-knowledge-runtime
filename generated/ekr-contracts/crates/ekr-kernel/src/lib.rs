// generated from ekr v1
// model digest 95e0cc738a4105082ac236fd226f432bb61b31bd4c0c3f8bafd2688d0a257609
// contract digest db51800daf271146ca5948dce9aaf371fac263e98eae1f4dd09a51739c94022c
// do not edit: regenerate with `ess synthesize`

//! ekr-kernel — the `ekr-kernel` component of `ekr` v1.
//!
//! Identity and reference rules, the transaction boundary from proposal through validation to an immutable committed revision, snapshot reads and the explain chain. The only component that constructs a validated transaction. Implemented by two crates: ekr-core holds the domain's types, ekr-kernel its commands.
//!
//! The component's outer surface exactly as the specification declares it: accepted commands as
//! handlers, declared views as queries, published events as a typed outbox. The behaviour behind
//! every handler is an implementation obligation — see the `PLAN.md` beside this workspace — and
//! until one is satisfied, its stub answers with a typed refusal naming what is owed.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event this component declares it publishes, on its way to the system's transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishedEvent {
    /// `ekr.kernel.AnswerAttentionResult`.
    AnswerAttentionResult(ekr_types::kernel::AnswerAttentionResult),
    /// `ekr.kernel.ApplyUpgradeResult`.
    ApplyUpgradeResult(ekr_types::kernel::ApplyUpgradeResult),
    /// `ekr.kernel.AttentionAnswered`.
    AttentionAnswered(ekr_types::kernel::AttentionAnswered),
    /// `ekr.kernel.Explained`.
    Explained(ekr_types::kernel::Explained),
    /// `ekr.kernel.ListAnswersResult`.
    ListAnswersResult(ekr_types::kernel::ListAnswersResult),
    /// `ekr.kernel.ListAttentionResult`.
    ListAttentionResult(ekr_types::kernel::ListAttentionResult),
    /// `ekr.kernel.PreviewUpgradeResult`.
    PreviewUpgradeResult(ekr_types::kernel::PreviewUpgradeResult),
    /// `ekr.kernel.RevisionCommitted`.
    RevisionCommitted(ekr_types::kernel::RevisionCommitted),
    /// `ekr.kernel.Seeded`.
    Seeded(ekr_types::kernel::Seeded),
    /// `ekr.kernel.ShowAttentionResult`.
    ShowAttentionResult(ekr_types::kernel::ShowAttentionResult),
    /// `ekr.kernel.SnapshotTaken`.
    SnapshotTaken(ekr_types::kernel::SnapshotTaken),
    /// `ekr.kernel.TransactionProposed`.
    TransactionProposed(ekr_types::kernel::TransactionProposed),
    /// `ekr.kernel.TransactionRejected`.
    TransactionRejected(ekr_types::kernel::TransactionRejected),
    /// `ekr.kernel.TransactionStale`.
    TransactionStale(ekr_types::kernel::TransactionStale),
    /// `ekr.kernel.TransactionValidated`.
    TransactionValidated(ekr_types::kernel::TransactionValidated),
    /// `ekr.store.CheckpointWritten`.
    CheckpointWritten(ekr_types::store::CheckpointWritten),
    /// `ekr.store.ObjectStored`.
    ObjectStored(ekr_types::store::ObjectStored),
    /// `ekr.store.PublicationPrepared`.
    PublicationPrepared(ekr_types::store::PublicationPrepared),
}

/// ekr-kernel — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct EkrKernel<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> EkrKernel<B> {
    /// A new port over the given obligation implementations.
    pub fn new(behaviors: B) -> Self {
        Self {
            behaviors,
            outbox: Vec::new(),
        }
    }

    /// Hands over everything published since the last drain, in publication order.
    ///
    /// The system's transport calls this; anything else reading it is taking events the transport
    /// will then never deliver.
    pub fn drain_outbox(&mut self) -> Vec<PublishedEvent> {
        core::mem::take(&mut self.outbox)
    }
}

impl<B> EkrKernel<B>
where
    B: ekr_types::kernel::obligations::AnswerAttentionBehavior + ekr_types::kernel::obligations::ApplyUpgradeBehavior + ekr_types::kernel::obligations::CommitBehavior + ekr_types::kernel::obligations::ExplainBehavior + ekr_types::kernel::obligations::ListAnswersBehavior + ekr_types::kernel::obligations::ListAttentionBehavior + ekr_types::kernel::obligations::PreviewUpgradeBehavior + ekr_types::kernel::obligations::ProposeBehavior + ekr_types::kernel::obligations::SeedBehavior + ekr_types::kernel::obligations::ShowAttentionBehavior + ekr_types::kernel::obligations::SnapshotBehavior + ekr_types::kernel::obligations::ValidateBehavior + ekr_types::kernel::obligations::AuthorityTransitionRecordsQuery + ekr_types::kernel::obligations::CurrentRevisionQuery + ekr_types::kernel::obligations::DisputeClaimRecordsQuery + ekr_types::kernel::obligations::DisputeRecordsQuery + ekr_types::kernel::obligations::HumanAnswerRecordsQuery + ekr_types::kernel::obligations::HumanDecisionRecordsQuery + ekr_types::kernel::obligations::PendingTransactionsQuery + ekr_types::kernel::obligations::RejectionsQuery + ekr_types::kernel::obligations::RetainedEvidenceQuery + ekr_types::kernel::obligations::RevisionsQuery + ekr_types::kernel::obligations::SchemaTransactionEvidenceRecordsQuery + ekr_types::kernel::obligations::TransactionsQuery + ekr_types::kernel::obligations::ValidationIssuesQuery,
{
    /// Accepts `ekr.kernel.AnswerAttention`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn answer_attention(&mut self, input: ekr_types::kernel::AnswerAttention) -> Result<ekr_types::kernel::AnswerAttentionOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.answer_attention(input)?;
        match &outcome {
            ekr_types::kernel::AnswerAttentionOutcome::Answered { answer_attention_result, .. } => {
                self.outbox.push(PublishedEvent::AnswerAttentionResult(answer_attention_result.clone()));
            }
            ekr_types::kernel::AnswerAttentionOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.kernel.ApplyUpgrade`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn apply_upgrade(&mut self, input: ekr_types::kernel::ApplyUpgrade) -> Result<ekr_types::kernel::ApplyUpgradeOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.apply_upgrade(input)?;
        match &outcome {
            ekr_types::kernel::ApplyUpgradeOutcome::Answered { apply_upgrade_result, .. } => {
                self.outbox.push(PublishedEvent::ApplyUpgradeResult(apply_upgrade_result.clone()));
            }
            ekr_types::kernel::ApplyUpgradeOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.kernel.Commit`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn commit(&mut self, input: ekr_types::kernel::Commit) -> Result<ekr_types::kernel::CommitOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.commit(input)?;
        match &outcome {
            ekr_types::kernel::CommitOutcome::Committed { revision_committed, publication_prepared, object_stored, checkpoint_written, .. } => {
                self.outbox.push(PublishedEvent::RevisionCommitted(revision_committed.clone()));
                self.outbox.push(PublishedEvent::PublicationPrepared(publication_prepared.clone()));
                self.outbox.push(PublishedEvent::ObjectStored(object_stored.clone()));
                self.outbox.push(PublishedEvent::CheckpointWritten(checkpoint_written.clone()));
            }
            ekr_types::kernel::CommitOutcome::Stale { transaction_stale, publication_prepared, object_stored, .. } => {
                self.outbox.push(PublishedEvent::TransactionStale(transaction_stale.clone()));
                self.outbox.push(PublishedEvent::PublicationPrepared(publication_prepared.clone()));
                self.outbox.push(PublishedEvent::ObjectStored(object_stored.clone()));
            }
            ekr_types::kernel::CommitOutcome::RetainedCommit { .. } => {}
            ekr_types::kernel::CommitOutcome::TransactionNotFound { .. } => {}
            ekr_types::kernel::CommitOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.kernel.Explain`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn explain(&mut self, input: ekr_types::kernel::Explain) -> Result<ekr_types::kernel::ExplainOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.explain(input)?;
        match &outcome {
            ekr_types::kernel::ExplainOutcome::Explained { explained, .. } => {
                self.outbox.push(PublishedEvent::Explained(explained.clone()));
            }
            ekr_types::kernel::ExplainOutcome::NotFound { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.kernel.ListAnswers`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn list_answers(&mut self, input: ekr_types::kernel::ListAnswers) -> Result<ekr_types::kernel::ListAnswersOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.list_answers(input)?;
        match &outcome {
            ekr_types::kernel::ListAnswersOutcome::Answered { list_answers_result, .. } => {
                self.outbox.push(PublishedEvent::ListAnswersResult(list_answers_result.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `ekr.kernel.ListAttention`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn list_attention(&mut self, input: ekr_types::kernel::ListAttention) -> Result<ekr_types::kernel::ListAttentionOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.list_attention(input)?;
        match &outcome {
            ekr_types::kernel::ListAttentionOutcome::Answered { list_attention_result, .. } => {
                self.outbox.push(PublishedEvent::ListAttentionResult(list_attention_result.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `ekr.kernel.PreviewUpgrade`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn preview_upgrade(&mut self, input: ekr_types::kernel::PreviewUpgrade) -> Result<ekr_types::kernel::PreviewUpgradeOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.preview_upgrade(input)?;
        match &outcome {
            ekr_types::kernel::PreviewUpgradeOutcome::Answered { preview_upgrade_result, .. } => {
                self.outbox.push(PublishedEvent::PreviewUpgradeResult(preview_upgrade_result.clone()));
            }
            ekr_types::kernel::PreviewUpgradeOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.kernel.Propose`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn propose(&mut self, input: ekr_types::kernel::Propose) -> Result<ekr_types::kernel::ProposeOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.propose(input)?;
        match &outcome {
            ekr_types::kernel::ProposeOutcome::Proposed { transaction_proposed, publication_prepared, object_stored, .. } => {
                self.outbox.push(PublishedEvent::TransactionProposed(transaction_proposed.clone()));
                self.outbox.push(PublishedEvent::PublicationPrepared(publication_prepared.clone()));
                self.outbox.push(PublishedEvent::ObjectStored(object_stored.clone()));
            }
            ekr_types::kernel::ProposeOutcome::Malformed { .. } => {}
            ekr_types::kernel::ProposeOutcome::Misattributed { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.kernel.Seed`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn seed(&mut self, input: ekr_types::kernel::Seed) -> Result<ekr_types::kernel::SeedOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.seed(input)?;
        match &outcome {
            ekr_types::kernel::SeedOutcome::Seeded { seeded, publication_prepared, object_stored, checkpoint_written, .. } => {
                self.outbox.push(PublishedEvent::Seeded(seeded.clone()));
                self.outbox.push(PublishedEvent::PublicationPrepared(publication_prepared.clone()));
                self.outbox.push(PublishedEvent::ObjectStored(object_stored.clone()));
                self.outbox.push(PublishedEvent::CheckpointWritten(checkpoint_written.clone()));
            }
            ekr_types::kernel::SeedOutcome::AlreadySeeded { .. } => {}
            ekr_types::kernel::SeedOutcome::RetainedSeed { .. } => {}
            ekr_types::kernel::SeedOutcome::InvalidSeed { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.kernel.ShowAttention`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn show_attention(&mut self, input: ekr_types::kernel::ShowAttention) -> Result<ekr_types::kernel::ShowAttentionOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.show_attention(input)?;
        match &outcome {
            ekr_types::kernel::ShowAttentionOutcome::Answered { show_attention_result, .. } => {
                self.outbox.push(PublishedEvent::ShowAttentionResult(show_attention_result.clone()));
            }
            ekr_types::kernel::ShowAttentionOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.kernel.Snapshot`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn snapshot(&mut self, input: ekr_types::kernel::Snapshot) -> Result<ekr_types::kernel::SnapshotOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.snapshot(input)?;
        match &outcome {
            ekr_types::kernel::SnapshotOutcome::Taken { snapshot_taken, .. } => {
                self.outbox.push(PublishedEvent::SnapshotTaken(snapshot_taken.clone()));
            }
            ekr_types::kernel::SnapshotOutcome::NotFound { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.kernel.Validate`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn validate(&mut self, input: ekr_types::kernel::Validate) -> Result<ekr_types::kernel::ValidateOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.validate(input)?;
        match &outcome {
            ekr_types::kernel::ValidateOutcome::Validated { transaction_validated, publication_prepared, object_stored, .. } => {
                self.outbox.push(PublishedEvent::TransactionValidated(transaction_validated.clone()));
                self.outbox.push(PublishedEvent::PublicationPrepared(publication_prepared.clone()));
                self.outbox.push(PublishedEvent::ObjectStored(object_stored.clone()));
            }
            ekr_types::kernel::ValidateOutcome::Rejected { transaction_rejected, publication_prepared, object_stored, .. } => {
                self.outbox.push(PublishedEvent::TransactionRejected(transaction_rejected.clone()));
                self.outbox.push(PublishedEvent::PublicationPrepared(publication_prepared.clone()));
                self.outbox.push(PublishedEvent::ObjectStored(object_stored.clone()));
            }
            ekr_types::kernel::ValidateOutcome::RevisionNotFound { .. } => {}
            ekr_types::kernel::ValidateOutcome::TransactionNotFound { .. } => {}
            ekr_types::kernel::ValidateOutcome::WrongState { .. } => {}
            ekr_types::kernel::ValidateOutcome::WrongStateUnknownInstance => {}
        }
        Ok(outcome)
    }

    /// Serves `ekr.kernel.AuthorityTransitionRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn authority_transition_records(&self) -> Result<Vec<ekr_types::kernel::AuthorityTransitionRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.authority_transition_records()
    }

    /// Serves `ekr.kernel.CurrentRevision` at `read_your_writes` consistency, from the owed projection.
    pub fn current_revision(&self) -> Result<Vec<ekr_types::kernel::CurrentRevision>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.current_revision()
    }

    /// Serves `ekr.kernel.DisputeClaimRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn dispute_claim_records(&self) -> Result<Vec<ekr_types::kernel::DisputeClaimRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.dispute_claim_records()
    }

    /// Serves `ekr.kernel.DisputeRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn dispute_records(&self) -> Result<Vec<ekr_types::kernel::DisputeRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.dispute_records()
    }

    /// Serves `ekr.kernel.HumanAnswerRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn human_answer_records(&self) -> Result<Vec<ekr_types::kernel::HumanAnswerRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.human_answer_records()
    }

    /// Serves `ekr.kernel.HumanDecisionRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn human_decision_records(&self) -> Result<Vec<ekr_types::kernel::HumanDecisionRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.human_decision_records()
    }

    /// Serves `ekr.kernel.PendingTransactions` at `read_your_writes` consistency, from the owed projection.
    pub fn pending_transactions(&self) -> Result<Vec<ekr_types::kernel::PendingTransactions>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.pending_transactions()
    }

    /// Serves `ekr.kernel.Rejections` at `read_your_writes` consistency, from the owed projection.
    pub fn rejections(&self, from: Option<ekr_types::kernel::RevisionNumber>, to: Option<ekr_types::kernel::RevisionNumber>) -> Result<Vec<ekr_types::kernel::Rejections>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.rejections(from, to)
    }

    /// Serves `ekr.kernel.RetainedEvidence` at `read_your_writes` consistency, from the owed projection.
    pub fn retained_evidence(&self) -> Result<Vec<ekr_types::kernel::RetainedEvidence>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.retained_evidence()
    }

    /// Serves `ekr.kernel.Revisions` at `read_your_writes` consistency, from the owed projection.
    pub fn revisions(&self) -> Result<Vec<ekr_types::kernel::Revisions>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.revisions()
    }

    /// Serves `ekr.kernel.SchemaTransactionEvidenceRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn schema_transaction_evidence_records(&self) -> Result<Vec<ekr_types::kernel::SchemaTransactionEvidenceRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.schema_transaction_evidence_records()
    }

    /// Serves `ekr.kernel.Transactions` at `read_your_writes` consistency, from the owed projection.
    pub fn transactions(&self) -> Result<Vec<ekr_types::kernel::Transactions>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.transactions()
    }

    /// Serves `ekr.kernel.ValidationIssues` at `read_your_writes` consistency, from the owed projection.
    pub fn validation_issues(&self) -> Result<Vec<ekr_types::kernel::ValidationIssues>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.validation_issues()
    }
}
