//! Generated proposal obligations over the native kernel admission path.
use super::{schema_proposal_projection as p, schema_proposals::error};
use crate::Commit;
use ekr_core::contracts::{
    integrate::obligations::{ShowSchemaProposalBehavior, SubmitSchemaProposalBehavior},
    obligation::UnmetObligation,
    primitives::Uuid,
};
use ekr_core::{contract_data as w, contracts::integrate as m, Timestamp};
use ekr_store::{
    IncubationRetention, ObjectStore, ObservationRetention, RevisionLog, SchemaProposalRetention,
    StoreError,
};

struct Behavior<'a, S: RevisionLog + ObjectStore> {
    commit: &'a Commit<S>,
    at: Timestamp,
    fault: Option<StoreError>,
    held: Option<w::EkrIntegrateSchemaProposalRead>,
}
impl<'a, S: RevisionLog + ObjectStore> Behavior<'a, S> {
    fn new(commit: &'a Commit<S>, at: Timestamp) -> Self {
        Self {
            commit,
            at,
            fault: None,
            held: None,
        }
    }
    fn finish<T>(&mut self, result: Result<T, UnmetObligation>) -> Result<T, StoreError> {
        if let Some(fault) = self.fault.take() {
            return Err(fault);
        }
        result.map_err(error)
    }
    fn refusal(&mut self, fault: StoreError) -> m::KnowledgeRefused {
        let result = m::KnowledgeRefused {
            code: "schema-proposal-refused".into(),
            reason: fault.to_string(),
        };
        self.fault = Some(fault);
        result
    }
    fn failed(&mut self, fault: StoreError, source: &'static str) -> UnmetObligation {
        self.fault = Some(fault);
        UnmetObligation {
            capability: "available proposal persistence",
            source,
        }
    }
}
impl<
        S: RevisionLog
            + ObjectStore
            + ObservationRetention
            + IncubationRetention
            + SchemaProposalRetention,
    > SubmitSchemaProposalBehavior for Behavior<'_, S>
{
    fn submit_schema_proposal(
        &mut self,
        input: m::SubmitSchemaProposal,
    ) -> Result<m::SubmitSchemaProposalOutcome, UnmetObligation> {
        let result = (|| {
            let proposal: w::EkrIntegrateSchemaProposalDocument =
                serde_json::from_slice(&input.payload).map_err(error)?;
            if p::document(&proposal)? != input.proposal {
                return Err(error("typed proposal differs from exact supplied bytes"));
            }
            let held = self.commit.retain_schema_proposal(
                &w::EkrIntegrateSchemaProposalImport {
                    proposal: Box::new(proposal),
                    payload: ekr_core::bytes::encode(&input.payload),
                },
                self.at,
            )?;
            let result = p::submitted(&held)?;
            self.held = Some(held);
            Ok(m::SubmitSchemaProposalOutcome::Answered {
                submit_schema_proposal_result: result,
            })
        })();
        match result {
            Ok(outcome) => Ok(outcome),
            Err(fault @ (StoreError::Document(_) | StoreError::PublicationInputConflict)) => {
                Ok(m::SubmitSchemaProposalOutcome::Refused {
                    error: self.refusal(fault),
                })
            }
            Err(fault) => Err(self.failed(fault, "ekr.integrate.SubmitSchemaProposal")),
        }
    }
}
impl<
        S: RevisionLog
            + ObjectStore
            + ObservationRetention
            + IncubationRetention
            + SchemaProposalRetention,
    > ShowSchemaProposalBehavior for Behavior<'_, S>
{
    fn show_schema_proposal(
        &mut self,
        input: m::ShowSchemaProposal,
    ) -> Result<m::ShowSchemaProposalOutcome, UnmetObligation> {
        let result = self
            .commit
            .retained_schema_proposal(&w::EkrIntegrateSchemaProposalId(input.proposal_id.0 .0))
            .and_then(|held| {
                let result = p::shown(&held)?;
                self.held = Some(held);
                Ok(m::ShowSchemaProposalOutcome::Answered {
                    show_schema_proposal_result: result,
                })
            });
        match result {
            Ok(outcome) => Ok(outcome),
            Err(fault @ StoreError::Document(_)) => Ok(m::ShowSchemaProposalOutcome::Refused {
                error: self.refusal(fault),
            }),
            Err(fault) => Err(self.failed(fault, "ekr.integrate.ShowSchemaProposal")),
        }
    }
}
pub(super) fn submit<
    S: RevisionLog
        + ObjectStore
        + ObservationRetention
        + IncubationRetention
        + SchemaProposalRetention,
>(
    commit: &Commit<S>,
    input: &w::EkrIntegrateSchemaProposalImport,
    at: Timestamp,
) -> Result<w::EkrIntegrateSchemaProposalRead, StoreError> {
    let mut behavior = Behavior::new(commit, at);
    let result = behavior.submit_schema_proposal(m::SubmitSchemaProposal {
        proposal: p::document(&input.proposal)?,
        payload: ekr_core::bytes::decode(&input.payload).map_err(error)?,
    });
    match behavior.finish(result)? {
        m::SubmitSchemaProposalOutcome::Answered {
            submit_schema_proposal_result,
        } => {
            let held = behavior
                .held
                .take()
                .ok_or_else(|| error("missing generated submission result"))?;
            if p::submitted(&held)? != submit_schema_proposal_result {
                return Err(error("generated submission result mismatch"));
            }
            Ok(held)
        }
        m::SubmitSchemaProposalOutcome::Refused { error: refusal } => Err(error(refusal.reason)),
    }
}
pub(super) fn show<
    S: RevisionLog
        + ObjectStore
        + ObservationRetention
        + IncubationRetention
        + SchemaProposalRetention,
>(
    commit: &Commit<S>,
    id: &w::EkrIntegrateSchemaProposalId,
) -> Result<w::EkrIntegrateSchemaProposalRead, StoreError> {
    let mut behavior = Behavior::new(commit, Timestamp::EPOCH);
    let result = behavior.show_schema_proposal(m::ShowSchemaProposal {
        proposal_id: m::SchemaProposalId(Uuid(id.0.clone())),
    });
    match behavior.finish(result)? {
        m::ShowSchemaProposalOutcome::Answered {
            show_schema_proposal_result,
        } => {
            let held = behavior
                .held
                .take()
                .ok_or_else(|| error("missing generated read result"))?;
            if p::shown(&held)? != show_schema_proposal_result {
                return Err(error("generated read result mismatch"));
            }
            Ok(held)
        }
        m::ShowSchemaProposalOutcome::Refused { error: refusal } => Err(error(refusal.reason)),
    }
}
