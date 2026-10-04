// generated from ekr v1
// model digest 2289ab6b66263f553d91e328b960af9f6934eef272c699c1b9ad967900abb2f4
// contract digest a468fb767edcd11ae49e6758907a635f9a371e456b8690b65bee6007ab27151d
// do not edit: regenerate with `ess synthesize`

//! ekr-integrate — the `ekr-integrate` component of `ekr` v1.
//!
//! Entity resolution of typed references against canonical nodes, and the merge and split lineage, and applying an extraction document. Holds no writer of its own; a resolution or an application proposes, and every change arrives as a committed transaction.
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
}

/// ekr-integrate — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct EkrIntegrate<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> EkrIntegrate<B> {
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

impl<B> EkrIntegrate<B>
where
    B: ekr_types::integrate::obligations::ApplyExtractionBehavior + ekr_types::integrate::obligations::ApplySchemaProposalBehavior + ekr_types::integrate::obligations::ApproveSchemaProposalBehavior + ekr_types::integrate::obligations::DiscoverSchemaGapsBehavior + ekr_types::integrate::obligations::ImportInterpretationBehavior + ekr_types::integrate::obligations::ListInterpretationsBehavior + ekr_types::integrate::obligations::RejectSchemaProposalBehavior + ekr_types::integrate::obligations::ShowInterpretationBehavior + ekr_types::integrate::obligations::ShowSchemaProposalBehavior + ekr_types::integrate::obligations::SubmitSchemaProposalBehavior + ekr_types::integrate::obligations::ApplicationPublicationRecordsQuery + ekr_types::integrate::obligations::ApplicationReceiptRecordsQuery + ekr_types::integrate::obligations::CanonicalDerivationRecordsQuery + ekr_types::integrate::obligations::IntegrationBlockerRecordsQuery + ekr_types::integrate::obligations::InterpretationObservationRecordsQuery + ekr_types::integrate::obligations::InterpretationRecordsQuery + ekr_types::integrate::obligations::MappingRecordRecordsQuery + ekr_types::integrate::obligations::ProcessingReceiptRecordsQuery + ekr_types::integrate::obligations::ProposalCoordinationOccurrenceRecordsQuery + ekr_types::integrate::obligations::ProposalCoordinationRecordsQuery + ekr_types::integrate::obligations::ProposalEvidenceRecordsQuery + ekr_types::integrate::obligations::ProposalObservationRecordsQuery + ekr_types::integrate::obligations::ProposalReviewRecordsQuery + ekr_types::integrate::obligations::ProposalSourceBindingRecordsQuery + ekr_types::integrate::obligations::SchemaProposalRecordsQuery,
{
    /// Accepts `ekr.integrate.ApplyExtraction`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn apply_extraction(&mut self, input: ekr_types::integrate::ApplyExtraction) -> Result<ekr_types::integrate::ApplyExtractionOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.apply_extraction(input)?;
        match &outcome {
            ekr_types::integrate::ApplyExtractionOutcome::Applied { extraction_applied, .. } => {
                self.outbox.push(PublishedEvent::ExtractionApplied(extraction_applied.clone()));
            }
            ekr_types::integrate::ApplyExtractionOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.integrate.ApplySchemaProposal`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn apply_schema_proposal(&mut self, input: ekr_types::integrate::ApplySchemaProposal) -> Result<ekr_types::integrate::ApplySchemaProposalOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.apply_schema_proposal(input)?;
        match &outcome {
            ekr_types::integrate::ApplySchemaProposalOutcome::Answered { apply_schema_proposal_result, .. } => {
                self.outbox.push(PublishedEvent::ApplySchemaProposalResult(apply_schema_proposal_result.clone()));
            }
            ekr_types::integrate::ApplySchemaProposalOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.integrate.ApproveSchemaProposal`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn approve_schema_proposal(&mut self, input: ekr_types::integrate::ApproveSchemaProposal) -> Result<ekr_types::integrate::ApproveSchemaProposalOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.approve_schema_proposal(input)?;
        match &outcome {
            ekr_types::integrate::ApproveSchemaProposalOutcome::Answered { approve_schema_proposal_result, .. } => {
                self.outbox.push(PublishedEvent::ApproveSchemaProposalResult(approve_schema_proposal_result.clone()));
            }
            ekr_types::integrate::ApproveSchemaProposalOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.integrate.DiscoverSchemaGaps`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn discover_schema_gaps(&mut self, input: ekr_types::integrate::DiscoverSchemaGaps) -> Result<ekr_types::integrate::DiscoverSchemaGapsOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.discover_schema_gaps(input)?;
        match &outcome {
            ekr_types::integrate::DiscoverSchemaGapsOutcome::Answered { discover_schema_gaps_result, .. } => {
                self.outbox.push(PublishedEvent::DiscoverSchemaGapsResult(discover_schema_gaps_result.clone()));
            }
            ekr_types::integrate::DiscoverSchemaGapsOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.integrate.ImportInterpretation`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn import_interpretation(&mut self, input: ekr_types::integrate::ImportInterpretation) -> Result<ekr_types::integrate::ImportInterpretationOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.import_interpretation(input)?;
        match &outcome {
            ekr_types::integrate::ImportInterpretationOutcome::Answered { import_interpretation_result, .. } => {
                self.outbox.push(PublishedEvent::ImportInterpretationResult(import_interpretation_result.clone()));
            }
            ekr_types::integrate::ImportInterpretationOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.integrate.ListInterpretations`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn list_interpretations(&mut self, input: ekr_types::integrate::ListInterpretations) -> Result<ekr_types::integrate::ListInterpretationsOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.list_interpretations(input)?;
        match &outcome {
            ekr_types::integrate::ListInterpretationsOutcome::Answered { list_interpretations_result, .. } => {
                self.outbox.push(PublishedEvent::ListInterpretationsResult(list_interpretations_result.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `ekr.integrate.RejectSchemaProposal`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn reject_schema_proposal(&mut self, input: ekr_types::integrate::RejectSchemaProposal) -> Result<ekr_types::integrate::RejectSchemaProposalOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.reject_schema_proposal(input)?;
        match &outcome {
            ekr_types::integrate::RejectSchemaProposalOutcome::Answered { reject_schema_proposal_result, .. } => {
                self.outbox.push(PublishedEvent::RejectSchemaProposalResult(reject_schema_proposal_result.clone()));
            }
            ekr_types::integrate::RejectSchemaProposalOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.integrate.ShowInterpretation`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn show_interpretation(&mut self, input: ekr_types::integrate::ShowInterpretation) -> Result<ekr_types::integrate::ShowInterpretationOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.show_interpretation(input)?;
        match &outcome {
            ekr_types::integrate::ShowInterpretationOutcome::Answered { show_interpretation_result, .. } => {
                self.outbox.push(PublishedEvent::ShowInterpretationResult(show_interpretation_result.clone()));
            }
            ekr_types::integrate::ShowInterpretationOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.integrate.ShowSchemaProposal`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn show_schema_proposal(&mut self, input: ekr_types::integrate::ShowSchemaProposal) -> Result<ekr_types::integrate::ShowSchemaProposalOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.show_schema_proposal(input)?;
        match &outcome {
            ekr_types::integrate::ShowSchemaProposalOutcome::Answered { show_schema_proposal_result, .. } => {
                self.outbox.push(PublishedEvent::ShowSchemaProposalResult(show_schema_proposal_result.clone()));
            }
            ekr_types::integrate::ShowSchemaProposalOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.integrate.SubmitSchemaProposal`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn submit_schema_proposal(&mut self, input: ekr_types::integrate::SubmitSchemaProposal) -> Result<ekr_types::integrate::SubmitSchemaProposalOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.submit_schema_proposal(input)?;
        match &outcome {
            ekr_types::integrate::SubmitSchemaProposalOutcome::Answered { submit_schema_proposal_result, .. } => {
                self.outbox.push(PublishedEvent::SubmitSchemaProposalResult(submit_schema_proposal_result.clone()));
            }
            ekr_types::integrate::SubmitSchemaProposalOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Serves `ekr.integrate.ApplicationPublicationRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn application_publication_records(&self) -> Result<Vec<ekr_types::integrate::ApplicationPublicationRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.application_publication_records()
    }

    /// Serves `ekr.integrate.ApplicationReceiptRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn application_receipt_records(&self) -> Result<Vec<ekr_types::integrate::ApplicationReceiptRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.application_receipt_records()
    }

    /// Serves `ekr.integrate.CanonicalDerivationRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn canonical_derivation_records(&self) -> Result<Vec<ekr_types::integrate::CanonicalDerivationRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.canonical_derivation_records()
    }

    /// Serves `ekr.integrate.IntegrationBlockerRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn integration_blocker_records(&self) -> Result<Vec<ekr_types::integrate::IntegrationBlockerRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.integration_blocker_records()
    }

    /// Serves `ekr.integrate.InterpretationObservationRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn interpretation_observation_records(&self) -> Result<Vec<ekr_types::integrate::InterpretationObservationRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.interpretation_observation_records()
    }

    /// Serves `ekr.integrate.InterpretationRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn interpretation_records(&self) -> Result<Vec<ekr_types::integrate::InterpretationRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.interpretation_records()
    }

    /// Serves `ekr.integrate.MappingRecordRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn mapping_record_records(&self) -> Result<Vec<ekr_types::integrate::MappingRecordRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.mapping_record_records()
    }

    /// Serves `ekr.integrate.ProcessingReceiptRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn processing_receipt_records(&self) -> Result<Vec<ekr_types::integrate::ProcessingReceiptRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.processing_receipt_records()
    }

    /// Serves `ekr.integrate.ProposalCoordinationOccurrenceRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn proposal_coordination_occurrence_records(&self) -> Result<Vec<ekr_types::integrate::ProposalCoordinationOccurrenceRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.proposal_coordination_occurrence_records()
    }

    /// Serves `ekr.integrate.ProposalCoordinationRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn proposal_coordination_records(&self) -> Result<Vec<ekr_types::integrate::ProposalCoordinationRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.proposal_coordination_records()
    }

    /// Serves `ekr.integrate.ProposalEvidenceRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn proposal_evidence_records(&self) -> Result<Vec<ekr_types::integrate::ProposalEvidenceRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.proposal_evidence_records()
    }

    /// Serves `ekr.integrate.ProposalObservationRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn proposal_observation_records(&self) -> Result<Vec<ekr_types::integrate::ProposalObservationRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.proposal_observation_records()
    }

    /// Serves `ekr.integrate.ProposalReviewRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn proposal_review_records(&self) -> Result<Vec<ekr_types::integrate::ProposalReviewRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.proposal_review_records()
    }

    /// Serves `ekr.integrate.ProposalSourceBindingRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn proposal_source_binding_records(&self) -> Result<Vec<ekr_types::integrate::ProposalSourceBindingRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.proposal_source_binding_records()
    }

    /// Serves `ekr.integrate.SchemaProposalRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn schema_proposal_records(&self) -> Result<Vec<ekr_types::integrate::SchemaProposalRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.schema_proposal_records()
    }
}
