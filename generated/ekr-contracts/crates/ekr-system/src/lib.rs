// generated from ekr v1
// model digest d93593588d2611a3d25542c06ca23a3ac2992d57a37c6785cf8b89f0846dc087
// contract digest ea2e8b0cc8f708c0b41768fcc389f371d3eddc8554b05b10d5d26385cc30a130
// do not edit: regenerate with `ess synthesize`

//! The `ekr` system, v1: its components assembled, its bindings wired, and its one transport.
//!
//! The transport is derived from the specification, not chosen: `at_least_once` is the only
//! delivery guarantee the model declares, so published events land on an append-only log and a
//! pump delivers each to every binding that reacts to it. The log is the system's observable
//! record, and so is the record of what each binding invoked. What no specification determines
//! — how an escalation event is filled, behaviour behind the ports — stays an obligation; see
//! the `PLAN.md` beside this workspace.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event on the system's log: everything any component publishes, and everything a binding
/// escalates into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemEvent {
    /// `ekr.integrate.ApplySchemaProposalResult`.
    ApplySchemaProposalResult(ekr_types::integrate::ApplySchemaProposalResult),
    /// `ekr.integrate.ApproveSchemaProposalResult`.
    ApproveSchemaProposalResult(ekr_types::integrate::ApproveSchemaProposalResult),
    /// `ekr.integrate.DiscoverSchemaGapsResult`.
    DiscoverSchemaGapsResult(ekr_types::integrate::DiscoverSchemaGapsResult),
    /// `ekr.integrate.ExtractionApplied`.
    ExtractionApplied(ekr_types::integrate::ExtractionApplied),
    /// `ekr.integrate.ImportInterpretationResult`.
    ImportInterpretationResult(ekr_types::integrate::ImportInterpretationResult),
    /// `ekr.integrate.ListInterpretationsResult`.
    ListInterpretationsResult(ekr_types::integrate::ListInterpretationsResult),
    /// `ekr.integrate.RejectSchemaProposalResult`.
    RejectSchemaProposalResult(ekr_types::integrate::RejectSchemaProposalResult),
    /// `ekr.integrate.ShowInterpretationResult`.
    ShowInterpretationResult(ekr_types::integrate::ShowInterpretationResult),
    /// `ekr.integrate.ShowSchemaProposalResult`.
    ShowSchemaProposalResult(ekr_types::integrate::ShowSchemaProposalResult),
    /// `ekr.integrate.SubmitSchemaProposalResult`.
    SubmitSchemaProposalResult(ekr_types::integrate::SubmitSchemaProposalResult),
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
    /// `ekr.observe.ImportObservationResult`.
    ImportObservationResult(ekr_types::observe::ImportObservationResult),
    /// `ekr.observe.ObservationShown`.
    ObservationShown(ekr_types::observe::ObservationShown),
    /// `ekr.observe.ObservationsListed`.
    ObservationsListed(ekr_types::observe::ObservationsListed),
    /// `ekr.store.CheckpointWritten`.
    CheckpointWritten(ekr_types::store::CheckpointWritten),
    /// `ekr.store.ObjectStored`.
    ObjectStored(ekr_types::store::ObjectStored),
    /// `ekr.store.PublicationPrepared`.
    PublicationPrepared(ekr_types::store::PublicationPrepared),
    /// `ekr.views.GraphProjected`.
    GraphProjected(ekr_types::views::GraphProjected),
}

impl SystemEvent {
    /// The qualified name the specification declares this event under.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ApplySchemaProposalResult(_) => "ekr.integrate.ApplySchemaProposalResult",
            Self::ApproveSchemaProposalResult(_) => "ekr.integrate.ApproveSchemaProposalResult",
            Self::DiscoverSchemaGapsResult(_) => "ekr.integrate.DiscoverSchemaGapsResult",
            Self::ExtractionApplied(_) => "ekr.integrate.ExtractionApplied",
            Self::ImportInterpretationResult(_) => "ekr.integrate.ImportInterpretationResult",
            Self::ListInterpretationsResult(_) => "ekr.integrate.ListInterpretationsResult",
            Self::RejectSchemaProposalResult(_) => "ekr.integrate.RejectSchemaProposalResult",
            Self::ShowInterpretationResult(_) => "ekr.integrate.ShowInterpretationResult",
            Self::ShowSchemaProposalResult(_) => "ekr.integrate.ShowSchemaProposalResult",
            Self::SubmitSchemaProposalResult(_) => "ekr.integrate.SubmitSchemaProposalResult",
            Self::AnswerAttentionResult(_) => "ekr.kernel.AnswerAttentionResult",
            Self::ApplyUpgradeResult(_) => "ekr.kernel.ApplyUpgradeResult",
            Self::AttentionAnswered(_) => "ekr.kernel.AttentionAnswered",
            Self::Explained(_) => "ekr.kernel.Explained",
            Self::ListAnswersResult(_) => "ekr.kernel.ListAnswersResult",
            Self::ListAttentionResult(_) => "ekr.kernel.ListAttentionResult",
            Self::PreviewUpgradeResult(_) => "ekr.kernel.PreviewUpgradeResult",
            Self::RevisionCommitted(_) => "ekr.kernel.RevisionCommitted",
            Self::Seeded(_) => "ekr.kernel.Seeded",
            Self::ShowAttentionResult(_) => "ekr.kernel.ShowAttentionResult",
            Self::SnapshotTaken(_) => "ekr.kernel.SnapshotTaken",
            Self::TransactionProposed(_) => "ekr.kernel.TransactionProposed",
            Self::TransactionRejected(_) => "ekr.kernel.TransactionRejected",
            Self::TransactionStale(_) => "ekr.kernel.TransactionStale",
            Self::TransactionValidated(_) => "ekr.kernel.TransactionValidated",
            Self::ImportObservationResult(_) => "ekr.observe.ImportObservationResult",
            Self::ObservationShown(_) => "ekr.observe.ObservationShown",
            Self::ObservationsListed(_) => "ekr.observe.ObservationsListed",
            Self::CheckpointWritten(_) => "ekr.store.CheckpointWritten",
            Self::ObjectStored(_) => "ekr.store.ObjectStored",
            Self::PublicationPrepared(_) => "ekr.store.PublicationPrepared",
            Self::GraphProjected(_) => "ekr.views.GraphProjected",
        }
    }
}

impl From<ekr_graph::PublishedEvent> for SystemEvent {
    fn from(event: ekr_graph::PublishedEvent) -> Self {
        match event {
        }
    }
}

impl From<ekr_integrate::PublishedEvent> for SystemEvent {
    fn from(event: ekr_integrate::PublishedEvent) -> Self {
        match event {
            ekr_integrate::PublishedEvent::ApplySchemaProposalResult(event) => Self::ApplySchemaProposalResult(event),
            ekr_integrate::PublishedEvent::ApproveSchemaProposalResult(event) => Self::ApproveSchemaProposalResult(event),
            ekr_integrate::PublishedEvent::DiscoverSchemaGapsResult(event) => Self::DiscoverSchemaGapsResult(event),
            ekr_integrate::PublishedEvent::ExtractionApplied(event) => Self::ExtractionApplied(event),
            ekr_integrate::PublishedEvent::ImportInterpretationResult(event) => Self::ImportInterpretationResult(event),
            ekr_integrate::PublishedEvent::ListInterpretationsResult(event) => Self::ListInterpretationsResult(event),
            ekr_integrate::PublishedEvent::RejectSchemaProposalResult(event) => Self::RejectSchemaProposalResult(event),
            ekr_integrate::PublishedEvent::ShowInterpretationResult(event) => Self::ShowInterpretationResult(event),
            ekr_integrate::PublishedEvent::ShowSchemaProposalResult(event) => Self::ShowSchemaProposalResult(event),
            ekr_integrate::PublishedEvent::SubmitSchemaProposalResult(event) => Self::SubmitSchemaProposalResult(event),
        }
    }
}

impl From<ekr_kernel::PublishedEvent> for SystemEvent {
    fn from(event: ekr_kernel::PublishedEvent) -> Self {
        match event {
            ekr_kernel::PublishedEvent::AnswerAttentionResult(event) => Self::AnswerAttentionResult(event),
            ekr_kernel::PublishedEvent::ApplyUpgradeResult(event) => Self::ApplyUpgradeResult(event),
            ekr_kernel::PublishedEvent::AttentionAnswered(event) => Self::AttentionAnswered(event),
            ekr_kernel::PublishedEvent::Explained(event) => Self::Explained(event),
            ekr_kernel::PublishedEvent::ListAnswersResult(event) => Self::ListAnswersResult(event),
            ekr_kernel::PublishedEvent::ListAttentionResult(event) => Self::ListAttentionResult(event),
            ekr_kernel::PublishedEvent::PreviewUpgradeResult(event) => Self::PreviewUpgradeResult(event),
            ekr_kernel::PublishedEvent::RevisionCommitted(event) => Self::RevisionCommitted(event),
            ekr_kernel::PublishedEvent::Seeded(event) => Self::Seeded(event),
            ekr_kernel::PublishedEvent::ShowAttentionResult(event) => Self::ShowAttentionResult(event),
            ekr_kernel::PublishedEvent::SnapshotTaken(event) => Self::SnapshotTaken(event),
            ekr_kernel::PublishedEvent::TransactionProposed(event) => Self::TransactionProposed(event),
            ekr_kernel::PublishedEvent::TransactionRejected(event) => Self::TransactionRejected(event),
            ekr_kernel::PublishedEvent::TransactionStale(event) => Self::TransactionStale(event),
            ekr_kernel::PublishedEvent::TransactionValidated(event) => Self::TransactionValidated(event),
            ekr_kernel::PublishedEvent::CheckpointWritten(event) => Self::CheckpointWritten(event),
            ekr_kernel::PublishedEvent::ObjectStored(event) => Self::ObjectStored(event),
            ekr_kernel::PublishedEvent::PublicationPrepared(event) => Self::PublicationPrepared(event),
        }
    }
}

impl From<ekr_observe::PublishedEvent> for SystemEvent {
    fn from(event: ekr_observe::PublishedEvent) -> Self {
        match event {
            ekr_observe::PublishedEvent::ImportObservationResult(event) => Self::ImportObservationResult(event),
            ekr_observe::PublishedEvent::ObservationShown(event) => Self::ObservationShown(event),
            ekr_observe::PublishedEvent::ObservationsListed(event) => Self::ObservationsListed(event),
        }
    }
}

impl From<ekr_ontology::PublishedEvent> for SystemEvent {
    fn from(event: ekr_ontology::PublishedEvent) -> Self {
        match event {
        }
    }
}

impl From<ekr_store::PublishedEvent> for SystemEvent {
    fn from(event: ekr_store::PublishedEvent) -> Self {
        match event {
        }
    }
}

impl From<ekr_views::PublishedEvent> for SystemEvent {
    fn from(event: ekr_views::PublishedEvent) -> Self {
        match event {
            ekr_views::PublishedEvent::GraphProjected(event) => Self::GraphProjected(event),
        }
    }
}

/// The `ekr` system: every component behind its port, and the transport between them.
///
/// The component fields are public because commands enter the system through a component's own
/// port; the log and its delivery cursor are not, because publishing happens by pumping, not by
/// writing history directly.
pub struct System<EkrGraphBehaviors, EkrIntegrateBehaviors, EkrKernelBehaviors, EkrObserveBehaviors, EkrOntologyBehaviors, EkrStoreBehaviors, EkrViewsBehaviors> {
    /// The `ekr-graph` component.
    pub ekr_graph: ekr_graph::EkrGraph<EkrGraphBehaviors>,
    /// The `ekr-integrate` component.
    pub ekr_integrate: ekr_integrate::EkrIntegrate<EkrIntegrateBehaviors>,
    /// The `ekr-kernel` component.
    pub ekr_kernel: ekr_kernel::EkrKernel<EkrKernelBehaviors>,
    /// The `ekr-observe` component.
    pub ekr_observe: ekr_observe::EkrObserve<EkrObserveBehaviors>,
    /// The `ekr-ontology` component.
    pub ekr_ontology: ekr_ontology::EkrOntology<EkrOntologyBehaviors>,
    /// The `ekr-store` component.
    pub ekr_store: ekr_store::EkrStore<EkrStoreBehaviors>,
    /// The `ekr-views` component.
    pub ekr_views: ekr_views::EkrViews<EkrViewsBehaviors>,
    published: Vec<SystemEvent>,
    cursor: usize,
}

impl<EkrGraphBehaviors, EkrIntegrateBehaviors, EkrKernelBehaviors, EkrObserveBehaviors, EkrOntologyBehaviors, EkrStoreBehaviors, EkrViewsBehaviors> System<EkrGraphBehaviors, EkrIntegrateBehaviors, EkrKernelBehaviors, EkrObserveBehaviors, EkrOntologyBehaviors, EkrStoreBehaviors, EkrViewsBehaviors> {
    /// Assembles the system from its components.
    pub fn new(ekr_graph: ekr_graph::EkrGraph<EkrGraphBehaviors>, ekr_integrate: ekr_integrate::EkrIntegrate<EkrIntegrateBehaviors>, ekr_kernel: ekr_kernel::EkrKernel<EkrKernelBehaviors>, ekr_observe: ekr_observe::EkrObserve<EkrObserveBehaviors>, ekr_ontology: ekr_ontology::EkrOntology<EkrOntologyBehaviors>, ekr_store: ekr_store::EkrStore<EkrStoreBehaviors>, ekr_views: ekr_views::EkrViews<EkrViewsBehaviors>) -> Self {
        Self {
            ekr_graph,
            ekr_integrate,
            ekr_kernel,
            ekr_observe,
            ekr_ontology,
            ekr_store,
            ekr_views,
            published: Vec::new(),
            cursor: 0,
        }
    }

    /// Everything published so far, in publication order — the system's observable record.
    pub fn published(&self) -> &[SystemEvent] {
        &self.published
    }

    /// Takes every event the pump has already delivered off the log, in publication order.
    ///
    /// A long-running shell calls this after each `pump`, or the log holds every event the
    /// process ever published. A `pump` returns with every logged event delivered: each
    /// reacting binding has had its attempt, and a binding whose attempt stopped holds the event in
    /// its own held-back list, not on the log. Events published since the last `pump` stay on the
    /// log, so the next `pump` still delivers them; taking never skips a binding.
    pub fn take_published(&mut self) -> Vec<SystemEvent> {
        let delivered: Vec<SystemEvent> = self.published.drain(..self.cursor).collect();
        self.cursor = 0;
        delivered
    }
}

impl<EkrGraphBehaviors, EkrIntegrateBehaviors, EkrKernelBehaviors, EkrObserveBehaviors, EkrOntologyBehaviors, EkrStoreBehaviors, EkrViewsBehaviors> System<EkrGraphBehaviors, EkrIntegrateBehaviors, EkrKernelBehaviors, EkrObserveBehaviors, EkrOntologyBehaviors, EkrStoreBehaviors, EkrViewsBehaviors>
where
    EkrGraphBehaviors: ekr_types::graph::obligations::AssertionsQuery + ekr_types::graph::obligations::SettledAssertionsQuery,
    EkrIntegrateBehaviors: ekr_types::integrate::obligations::ApplyExtractionBehavior + ekr_types::integrate::obligations::ApplySchemaProposalBehavior + ekr_types::integrate::obligations::ApproveSchemaProposalBehavior + ekr_types::integrate::obligations::DiscoverSchemaGapsBehavior + ekr_types::integrate::obligations::ImportInterpretationBehavior + ekr_types::integrate::obligations::ListInterpretationsBehavior + ekr_types::integrate::obligations::RejectSchemaProposalBehavior + ekr_types::integrate::obligations::ShowInterpretationBehavior + ekr_types::integrate::obligations::ShowSchemaProposalBehavior + ekr_types::integrate::obligations::SubmitSchemaProposalBehavior + ekr_types::integrate::obligations::ApplicationPublicationRecordsQuery + ekr_types::integrate::obligations::ApplicationReceiptRecordsQuery + ekr_types::integrate::obligations::CanonicalDerivationRecordsQuery + ekr_types::integrate::obligations::IntegrationBlockerRecordsQuery + ekr_types::integrate::obligations::InterpretationObservationRecordsQuery + ekr_types::integrate::obligations::InterpretationRecordsQuery + ekr_types::integrate::obligations::MappingRecordRecordsQuery + ekr_types::integrate::obligations::ProcessingReceiptRecordsQuery + ekr_types::integrate::obligations::ProposalCoordinationOccurrenceRecordsQuery + ekr_types::integrate::obligations::ProposalCoordinationRecordsQuery + ekr_types::integrate::obligations::ProposalEvidenceRecordsQuery + ekr_types::integrate::obligations::ProposalObservationRecordsQuery + ekr_types::integrate::obligations::ProposalReviewRecordsQuery + ekr_types::integrate::obligations::ProposalSourceBindingRecordsQuery + ekr_types::integrate::obligations::SchemaProposalRecordsQuery,
    EkrKernelBehaviors: ekr_types::kernel::obligations::AnswerAttentionBehavior + ekr_types::kernel::obligations::ApplyUpgradeBehavior + ekr_types::kernel::obligations::CommitBehavior + ekr_types::kernel::obligations::ExplainBehavior + ekr_types::kernel::obligations::ListAnswersBehavior + ekr_types::kernel::obligations::ListAttentionBehavior + ekr_types::kernel::obligations::PreviewUpgradeBehavior + ekr_types::kernel::obligations::ProposeBehavior + ekr_types::kernel::obligations::SeedBehavior + ekr_types::kernel::obligations::ShowAttentionBehavior + ekr_types::kernel::obligations::SnapshotBehavior + ekr_types::kernel::obligations::ValidateBehavior + ekr_types::kernel::obligations::AuthorityTransitionRecordsQuery + ekr_types::kernel::obligations::CurrentRevisionQuery + ekr_types::kernel::obligations::DisputeClaimRecordsQuery + ekr_types::kernel::obligations::DisputeRecordsQuery + ekr_types::kernel::obligations::HumanAnswerRecordsQuery + ekr_types::kernel::obligations::HumanDecisionRecordsQuery + ekr_types::kernel::obligations::PendingTransactionsQuery + ekr_types::kernel::obligations::RejectionsQuery + ekr_types::kernel::obligations::RetainedEvidenceQuery + ekr_types::kernel::obligations::RevisionsQuery + ekr_types::kernel::obligations::SchemaTransactionEvidenceRecordsQuery + ekr_types::kernel::obligations::TransactionsQuery + ekr_types::kernel::obligations::ValidationIssuesQuery,
    EkrObserveBehaviors: ekr_types::observe::obligations::ImportObservationBehavior + ekr_types::observe::obligations::ListObservationsBehavior + ekr_types::observe::obligations::ShowObservationBehavior + ekr_types::observe::obligations::RetainedObservationRecordsQuery,
    EkrOntologyBehaviors: ekr_types::ontology::obligations::NodeTypesByVersionQuery,
    EkrViewsBehaviors: ekr_types::views::obligations::ProjectGraphBehavior,
{
    /// Delivers until quiescent: collects every component's outbox onto the log. No binding
    /// reacts to anything this specification publishes, so collecting is the whole delivery.
    pub fn pump(&mut self) -> Result<(), ekr_types::obligation::UnmetObligation> {
        loop {
            self.collect();
            if self.cursor == self.published.len() {
                return Ok(());
            }
            self.cursor += 1;
        }
    }

    /// Moves every component's outbox onto the log, in component order.
    fn collect(&mut self) {
        for event in self.ekr_graph.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.ekr_integrate.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.ekr_kernel.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.ekr_observe.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.ekr_ontology.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.ekr_store.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.ekr_views.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
    }
}
