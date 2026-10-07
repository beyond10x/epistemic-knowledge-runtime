// generated from ekr v1
// model digest 1e0b3811627bd0ec3f1694ea3bbfe72b0d3a639819a331e0ca2e6b3c6f1bab6c
// contract digest 81bc26adc457d90b34775fbb51b22228c911c951ed1a48fe6047b7dc22a76819
// do not edit: regenerate with `ess synthesize`

//! ekr-observe — the `ekr-observe` component of `ekr` v1.
//!
//! The observation layer's vocabulary: source units, checkpoints, the health of one poll and the idempotency key of an observation. Supplied observations are append-only through ekr-store; polling, checkpoint and source-unit links remain held open by decision-blockers.
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
    /// `ekr.observe.ImportObservationResult`.
    ImportObservationResult(ekr_types::observe::ImportObservationResult),
    /// `ekr.observe.ObservationShown`.
    ObservationShown(ekr_types::observe::ObservationShown),
    /// `ekr.observe.ObservationsListed`.
    ObservationsListed(ekr_types::observe::ObservationsListed),
}

/// ekr-observe — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct EkrObserve<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> EkrObserve<B> {
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

impl<B> EkrObserve<B>
where
    B: ekr_types::observe::obligations::ImportObservationBehavior + ekr_types::observe::obligations::ListObservationsBehavior + ekr_types::observe::obligations::ShowObservationBehavior + ekr_types::observe::obligations::RetainedObservationRecordsQuery,
{
    /// Accepts `ekr.observe.ImportObservation`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn import_observation(&mut self, input: ekr_types::observe::ImportObservation) -> Result<ekr_types::observe::ImportObservationOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.import_observation(input)?;
        match &outcome {
            ekr_types::observe::ImportObservationOutcome::Answered { import_observation_result, .. } => {
                self.outbox.push(PublishedEvent::ImportObservationResult(import_observation_result.clone()));
            }
            ekr_types::observe::ImportObservationOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `ekr.observe.ListObservations`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn list_observations(&mut self, input: ekr_types::observe::ListObservations) -> Result<ekr_types::observe::ListObservationsOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.list_observations(input)?;
        match &outcome {
            ekr_types::observe::ListObservationsOutcome::Listed { observations_listed, .. } => {
                self.outbox.push(PublishedEvent::ObservationsListed(observations_listed.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `ekr.observe.ShowObservation`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn show_observation(&mut self, input: ekr_types::observe::ShowObservation) -> Result<ekr_types::observe::ShowObservationOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.show_observation(input)?;
        match &outcome {
            ekr_types::observe::ShowObservationOutcome::Shown { observation_shown, .. } => {
                self.outbox.push(PublishedEvent::ObservationShown(observation_shown.clone()));
            }
            ekr_types::observe::ShowObservationOutcome::Refused { .. } => {}
        }
        Ok(outcome)
    }

    /// Serves `ekr.observe.RetainedObservationRecords` at `read_your_writes` consistency, from the owed projection.
    pub fn retained_observation_records(&self) -> Result<Vec<ekr_types::observe::RetainedObservationRecords>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.retained_observation_records()
    }
}
