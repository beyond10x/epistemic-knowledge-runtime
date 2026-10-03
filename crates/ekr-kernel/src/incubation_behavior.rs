//! Generated interpretation obligations over the native retention workflow.
use super::incubation_document::error;
use super::incubation_projection as projection;
use crate::Commit;
use ekr_core::contract_data as wire;
use ekr_core::contracts::integrate as model;
use ekr_core::contracts::integrate::obligations::{
    ImportInterpretationBehavior, ListInterpretationsBehavior, ShowInterpretationBehavior,
};
use ekr_core::contracts::obligation::UnmetObligation;
use ekr_core::Timestamp;
use ekr_store::{IncubationRetention, ObjectStore, ObservationRetention, RevisionLog, StoreError};

struct Behavior<'a, S: RevisionLog + ObjectStore> {
    commit: &'a Commit<S>,
    at: Timestamp,
    fault: Option<StoreError>,
    imported: Option<wire::EkrIntegrateIncubationImportReceipt>,
    shown: Option<wire::EkrIntegrateInterpretationRead>,
}
impl<'a, S: RevisionLog + ObjectStore> Behavior<'a, S> {
    fn new(commit: &'a Commit<S>, at: Timestamp) -> Self {
        Self {
            commit,
            at,
            fault: None,
            imported: None,
            shown: None,
        }
    }
    fn finish<T>(&mut self, result: Result<T, UnmetObligation>) -> Result<T, StoreError> {
        if let Some(error) = self.fault.take() {
            return Err(error);
        }
        result.map_err(error)
    }
    fn failed(&mut self, error: StoreError, source: &'static str) -> UnmetObligation {
        self.fault = Some(error);
        UnmetObligation {
            capability: "available interpretation persistence",
            source,
        }
    }
    fn refusal(&mut self, error: StoreError) -> model::KnowledgeRefused {
        let refusal = model::KnowledgeRefused {
            code: "incubation-refused".into(),
            reason: error.to_string(),
        };
        self.fault = Some(error);
        refusal
    }
}
impl<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention>
    ImportInterpretationBehavior for Behavior<'_, S>
{
    fn import_interpretation(
        &mut self,
        input: model::ImportInterpretation,
    ) -> Result<model::ImportInterpretationOutcome, UnmetObligation> {
        let result = (|| {
            let document: wire::EkrIntegrateInterpretationDocument =
                serde_json::from_slice(&input.payload).map_err(error)?;
            if projection::document(&document)? != input.document {
                return Err(error("typed document differs from exact supplied bytes"));
            }
            let receipt = self.commit.retain_interpretation(
                &wire::EkrIntegrateInterpretationImport {
                    document: Box::new(document),
                    payload: ekr_core::bytes::encode(&input.payload),
                },
                self.at,
            )?;
            let projected = projection::receipt(&receipt)?;
            self.imported = Some(receipt);
            Ok(model::ImportInterpretationOutcome::Answered {
                import_interpretation_result: model::ImportInterpretationResult {
                    receipt: projected,
                },
            })
        })();
        match result {
            Ok(outcome) => Ok(outcome),
            Err(error @ (StoreError::Document(_) | StoreError::PublicationInputConflict)) => {
                Ok(model::ImportInterpretationOutcome::Refused {
                    error: self.refusal(error),
                })
            }
            Err(error) => Err(self.failed(error, "ekr.integrate.ImportInterpretation")),
        }
    }
}
impl<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention>
    ListInterpretationsBehavior for Behavior<'_, S>
{
    fn list_interpretations(
        &mut self,
        _: model::ListInterpretations,
    ) -> Result<model::ListInterpretationsOutcome, UnmetObligation> {
        let result = self
            .commit
            .retained_interpretation_versions()
            .and_then(|versions| {
                versions
                    .iter()
                    .map(projection::version)
                    .collect::<Result<Vec<_>, _>>()
            });
        match result {
            Ok(interpretations) => Ok(model::ListInterpretationsOutcome::Answered {
                list_interpretations_result: model::ListInterpretationsResult { interpretations },
            }),
            Err(error) => Err(self.failed(error, "ekr.integrate.ListInterpretations")),
        }
    }
}
impl<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention>
    ShowInterpretationBehavior for Behavior<'_, S>
{
    fn show_interpretation(
        &mut self,
        input: model::ShowInterpretation,
    ) -> Result<model::ShowInterpretationOutcome, UnmetObligation> {
        let result = self
            .commit
            .retained_interpretation(&projection::wire_version(input.version))
            .and_then(|held| {
                let projected = projection::read(&held)?;
                self.shown = Some(held);
                Ok(model::ShowInterpretationOutcome::Answered {
                    show_interpretation_result: projected,
                })
            });
        match result {
            Ok(outcome) => Ok(outcome),
            Err(error @ StoreError::Document(_)) => Ok(model::ShowInterpretationOutcome::Refused {
                error: self.refusal(error),
            }),
            Err(error) => Err(self.failed(error, "ekr.integrate.ShowInterpretation")),
        }
    }
}

pub(super) fn import<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention>(
    commit: &Commit<S>,
    input: &wire::EkrIntegrateInterpretationImport,
    at: Timestamp,
) -> Result<wire::EkrIntegrateIncubationImportReceipt, StoreError> {
    let mut behavior = Behavior::new(commit, at);
    let result = behavior.import_interpretation(model::ImportInterpretation {
        document: projection::document(&input.document)?,
        payload: projection::bytes(&input.payload)?,
    });
    match behavior.finish(result)? {
        model::ImportInterpretationOutcome::Answered {
            import_interpretation_result,
        } => {
            let held = behavior
                .imported
                .take()
                .ok_or_else(|| error("missing generated import result"))?;
            if projection::receipt(&held)? != import_interpretation_result.receipt {
                return Err(error("generated import result mismatch"));
            }
            Ok(held)
        }
        model::ImportInterpretationOutcome::Refused { error: refusal } => {
            Err(error(refusal.reason))
        }
    }
}
pub(super) fn list<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention>(
    commit: &Commit<S>,
) -> Result<Vec<wire::EkrIntegrateInterpretationVersion>, StoreError> {
    let mut behavior = Behavior::new(commit, Timestamp::EPOCH);
    let result = behavior.list_interpretations(model::ListInterpretations {});
    let model::ListInterpretationsOutcome::Answered {
        list_interpretations_result,
    } = behavior.finish(result)?;
    Ok(list_interpretations_result
        .interpretations
        .into_iter()
        .map(projection::wire_version)
        .collect())
}
pub(super) fn show<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention>(
    commit: &Commit<S>,
    version: &wire::EkrIntegrateInterpretationVersion,
) -> Result<wire::EkrIntegrateInterpretationRead, StoreError> {
    let mut behavior = Behavior::new(commit, Timestamp::EPOCH);
    let result = behavior.show_interpretation(model::ShowInterpretation {
        version: projection::version(version)?,
    });
    match behavior.finish(result)? {
        model::ShowInterpretationOutcome::Answered {
            show_interpretation_result,
        } => {
            let held = behavior
                .shown
                .take()
                .ok_or_else(|| error("missing generated read result"))?;
            if projection::read(&held)? != show_interpretation_result {
                return Err(error("generated read result mismatch"));
            }
            Ok(held)
        }
        model::ShowInterpretationOutcome::Refused { error: refusal } => Err(error(refusal.reason)),
    }
}
