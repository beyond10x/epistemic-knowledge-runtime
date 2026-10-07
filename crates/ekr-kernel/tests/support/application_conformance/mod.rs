//! Test-only semantic target. Commands execute Runtime; rows come from verified retention.
mod values;
// The unchanged shared native fixture contains helpers for other review cases as well.
#[allow(dead_code, unused_imports)]
mod fixture {
    include!("../schema_proposal_fixture.rs");
    include!("../schema_application_history.rs");
    include!("fixture.rs");
}

use ekr_core::{contract_data as w, ContentHash, Timestamp};
use ekr_kernel::{ExplanationLink, Runtime};
use ess_conformance::target::*;
use ess_primitives::node::Node;
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
};
use values::{fail, input, map, node, value};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Normal,
    Inert,
    DropProvenance,
    DuplicateWrite,
}
impl Control {
    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "native",
            Self::Inert => "inert",
            Self::DropProvenance => "dropped-provenance",
            Self::DuplicateWrite => "duplicate-write",
        }
    }
}
pub struct Target {
    work: PathBuf,
    sqlite: bool,
    control: Control,
    state: RefCell<Option<fixture::Prepared>>,
    events: RefCell<Vec<ObservedEvent>>,
    integrity: RefCell<Vec<String>>,
}
impl Target {
    pub fn new(work: &Path, sqlite: bool, control: Control) -> Self {
        Self {
            work: work.into(),
            sqlite,
            control,
            state: RefCell::new(None),
            events: RefCell::new(vec![]),
            integrity: RefCell::new(vec![]),
        }
    }
    pub fn integrity_observations(&self) -> Vec<String> {
        self.integrity.borrow().clone()
    }
    fn event(
        &self,
        request: &SemanticCommandRequest,
        name: &str,
        payload: Value,
    ) -> Result<ObservedEvent, TargetError> {
        let mut event = ObservedEvent::new(name.parse().map_err(|e| fail("event name", e))?)
            .in_activity(request.correlation.clone())
            .at(self.events.borrow().len().try_into().unwrap());
        event.payload = map(payload)?;
        self.events.borrow_mut().push(event.clone());
        Ok(event)
    }
    fn answer(
        &self,
        request: &SemanticCommandRequest,
        outcome: &str,
        event: &str,
        payload: Value,
    ) -> Result<SemanticCommandResult, TargetError> {
        let mut result = SemanticCommandResult::undeclared();
        result.outcome = Some(
            serde_json::from_value(
                json!({"command":request.command.to_string(),"outcome":outcome}),
            )
            .map_err(|e| fail("outcome", e))?,
        );
        if self.control != Control::Inert {
            let observed = self.event(request, event, payload)?;
            result.response = Some(observed.payload.clone());
            result.direct_events.push(observed);
        }
        Ok(result)
    }
}
impl ConformanceTarget for Target {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            format!(
                "EKR native application sqlite={} control={:?}",
                self.sqlite, self.control
            ),
            env!("CARGO_PKG_VERSION"),
        ))
    }
    fn fixture_values(
        &self,
        scenario: &ScenarioContext,
        contract: &ess_conformance::fixtures::Contract,
    ) -> Result<BTreeMap<String, Node>, TargetError> {
        let prepared =
            fixture::Prepared::new(&self.work, self.sqlite, &scenario.scenario.to_string())?;
        let values = prepared
            .fixtures
            .iter()
            .filter(|(name, _)| {
                contract
                    .fields
                    .iter()
                    .any(|f| f.name.as_str() == name.as_str())
            })
            .map(|(k, v)| Ok((k.clone(), node(v.clone())?)))
            .collect::<Result<_, _>>()?;
        *self.state.borrow_mut() = Some(prepared);
        Ok(values)
    }
    fn begin_scenario(&self, _: &ScenarioContext) -> Result<(), TargetError> {
        self.events.borrow_mut().clear();
        if self.state.borrow().is_none() {
            return Err(fail("scenario", "fixture setup absent"));
        }
        Ok(())
    }
    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let mut state = self.state.borrow_mut();
        let state = state
            .as_mut()
            .ok_or_else(|| fail("command", "fixture absent"))?;
        let command = request.command.to_string();
        let supplied = input(&Node::Map(request.input.clone()))?;
        if self.control == Control::Inert {
            // Inert control reports no events and changes no state. Expected outcomes remain
            // command-shaped; positive suite event/retained assertions must reject it.
            let outcome = if command.ends_with("Snapshot") {
                "taken"
            } else if command.ends_with("Explain") {
                "explained"
            } else {
                "answered"
            };
            return self.answer(&request, outcome, "", Value::Null);
        }
        if command == "ekr.kernel.Explain" {
            let id: ekr_core::AssertionId =
                serde_json::from_value(supplied["assertion_id"].clone())
                    .map_err(|e| fail("assertion input", e))?;
            let explanation = state.explain(id)?;
            let result = self.answer(
                &request,
                "explained",
                "ekr.kernel.Explained",
                json!({"assertion_id":explanation.assertion_id,"links":explanation.links.len()}),
            )?;
            self.integrity
                .borrow_mut()
                .push("Explain: detached verified capture unchanged".into());
            return Ok(result);
        }
        let runtime = state.open()?;
        let before = state.boundary(&runtime)?;
        let result = match command.as_str() {
            "ekr.integrate.ApproveSchemaProposal" => {
                let mut supplied = supplied;
                if supplied["human_proof"]["intent"]["format"] != "HumanDecision1" {
                    return Err(fail("approval input", "unknown semantic format"));
                }
                supplied["human_proof"]["intent"]["format"] = json!("ekr.human-decision/1");
                let document: w::EkrIntegrateSchemaProposalReviewApplication =
                    serde_json::from_value(supplied).map_err(|e| fail("approval input", e))?;
                let returned =
                    runtime.approve_schema_proposal(&document, Timestamp::from_millis(10));
                let after = state.boundary(&runtime)?;
                if before.canonical != after.canonical {
                    return Err(fail("approval integrity", "canonical occurrences changed"));
                }
                match returned {
                    Ok(review) => {
                        let retained = runtime
                            .schema_proposal_reviews(&state.proposal_id)
                            .map_err(|e| fail("review read", e))?;
                        if !retained.iter().any(|r| *r.review == review) {
                            return Err(fail("review integrity", "returned decision not retained"));
                        }
                        self.answer(
                            &request,
                            "answered",
                            "ekr.integrate.ApproveSchemaProposalResult",
                            json!({"review_id":review.review_id}),
                        )?
                    }
                    Err(ekr_kernel::PersistenceError::Document(reason))
                        if reason.starts_with("schema-proposal-refused: ") =>
                    {
                        if before != after {
                            return Err(fail(
                                "refused approval integrity",
                                "retained state changed",
                            ));
                        }
                        let mut result = SemanticCommandResult::undeclared();
                        result.outcome = Some(
                            serde_json::from_value(json!({"command":command,"outcome":"refused"}))
                                .unwrap(),
                        );
                        result.error = Some(
                            DeclaredErrorValue::new(
                                "ekr.integrate.KnowledgeRefused".parse().unwrap(),
                            )
                            .with("code", Node::Text("schema-proposal".into()))
                            .with("reason", Node::Text(reason)),
                        );
                        result
                    }
                    Err(error) => return Err(fail("approve", error)),
                }
            }
            "ekr.integrate.ShowSchemaProposal" => {
                let id = serde_json::from_value(supplied["proposal_id"].clone())
                    .map_err(|e| fail("proposal input", e))?;
                let shown = runtime
                    .schema_proposal(&id)
                    .map_err(|e| fail("show proposal", e))?;
                self.answer(
                    &request,
                    "answered",
                    "ekr.integrate.ShowSchemaProposalResult",
                    value(shown)?,
                )?
            }
            "ekr.kernel.Snapshot" => {
                if !supplied.as_object().is_some_and(serde_json::Map::is_empty) {
                    return Err(fail(
                        "snapshot input",
                        "only current snapshot is admitted here",
                    ));
                }
                let read = runtime.read(None).map_err(|e| fail("snapshot", e))?;
                self.answer(&request,"taken","ekr.kernel.SnapshotTaken",json!({"number":read.root.revision.get(),"knowledge_root":read.root.knowledge_root.to_string()}))?
            }
            "ekr.integrate.ApplySchemaProposal" => {
                let id = serde_json::from_value(supplied["proposal_id"].clone())
                    .map_err(|e| fail("proposal input", e))?;
                let review = serde_json::from_value(supplied["review_id"].clone())
                    .map_err(|e| fail("review input", e))?;
                let digest = serde_json::from_value(supplied["proposal_digest"].clone())
                    .map_err(|e| fail("digest input", e))?;
                let repeat = state.applied;
                let report = runtime
                    .apply_schema_proposal(&id, &review, &digest, Timestamp::from_millis(20))
                    .map_err(|e| fail("apply", e))?;
                if repeat && self.control == Control::DuplicateWrite {
                    state.duplicate_write(&runtime)?;
                }
                let after = state.boundary(&runtime)?;
                if repeat && before != after {
                    self.integrity
                        .borrow_mut()
                        .push("duplicate-write: repeat changed retained history".into());
                    return Err(fail(
                        "repeat application integrity",
                        "retained history changed",
                    ));
                }
                state.verify_report(&runtime, &report, repeat)?;
                state.applied = true;
                self.integrity.borrow_mut().push(if repeat { "Apply repeat: complete report and full retained state unchanged" } else { "Apply resume: complete report matches independently verified commits and receipts" }.into());
                self.answer(
                    &request,
                    "answered",
                    "ekr.integrate.ApplySchemaProposalResult",
                    json!({"receipt":report}),
                )?
            }
            _ => {
                return Err(TargetError::unsupported(
                    command,
                    "not an authored application command",
                ))
            }
        };
        if matches!(
            command.as_str(),
            "ekr.integrate.ShowSchemaProposal" | "ekr.kernel.Snapshot"
        ) && state.boundary(&runtime)? != before
        {
            return Err(fail("read command integrity", "retained state changed"));
        }
        Ok(result)
    }
    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        if !request.params.is_empty() {
            return Err(TargetError::unsupported(
                request.view.to_string(),
                "no view parameters",
            ));
        }
        let state = self.state.borrow();
        let state = state
            .as_ref()
            .ok_or_else(|| fail("view", "fixture absent"))?;
        let rows = state.rows(&request.view.to_string(), &request.consistency)?;
        if self.control == Control::Inert
            || (self.control == Control::DropProvenance
                && matches!(
                    request.view.to_string().as_str(),
                    "ekr.integrate.MappingRecordRecords"
                        | "ekr.integrate.CanonicalDerivationRecords"
                ))
        {
            return Ok(SemanticViewResult::default());
        }
        Ok(SemanticViewResult::of(
            rows.into_iter().map(map).collect::<Result<_, _>>()?,
        ))
    }
    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        Ok(self
            .events
            .borrow()
            .iter()
            .filter(|e| {
                e.event == request.event && e.correlation.as_ref() == Some(&request.correlation)
            })
            .cloned()
            .collect())
    }
}
