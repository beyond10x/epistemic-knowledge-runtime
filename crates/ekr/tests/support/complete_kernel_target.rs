//! Dispatch the complete kernel inventory to real native adapters by scenario.
#[path = "knowledge_target.rs"]
#[allow(dead_code)]
mod knowledge_target;
#[path = "upgrade_target.rs"]
#[allow(dead_code)]
mod upgrade_target;

use ekr::conformance::{KernelTarget, Provider};
use ess_conformance::target::*;
use ess_primitives::node::Node;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub struct CompleteKernelTarget {
    kernel: KernelTarget,
    knowledge: RefCell<Option<upgrade_target::UpgradeTarget>>,
    provider: Provider,
    work: PathBuf,
}
impl CompleteKernelTarget {
    pub fn new(provider: Provider, fixtures: &Path, work: &Path) -> Result<Self, TargetError> {
        Ok(Self {
            kernel: KernelTarget::new(provider, fixtures, work)
                .map_err(|e| TargetError::unavailable("opening kernel fixtures", e))?,
            knowledge: RefCell::new(None),
            provider,
            work: work.into(),
        })
    }
    fn prepare(&self, scenario: &ScenarioContext) {
        let id = scenario.scenario.to_string();
        let command = id
            .strip_prefix("ekr.kernel.")
            .and_then(|id| id.split_once('/'))
            .map(|(command, _)| command);
        *self.knowledge.borrow_mut() = match command {
            Some("AnswerAttention" | "ShowAttention" | "ListAttention" | "ListAnswers") => Some(
                upgrade_target::UpgradeTarget::generated(self.provider, &self.work, true),
            ),
            Some("PreviewUpgrade" | "ApplyUpgrade") => Some(
                upgrade_target::UpgradeTarget::generated(self.provider, &self.work, false),
            ),
            _ => None,
        };
    }
    fn with<T>(
        &self,
        f: impl FnOnce(&dyn ConformanceTarget) -> Result<T, TargetError>,
    ) -> Result<T, TargetError> {
        match &*self.knowledge.borrow() {
            Some(target) => f(target),
            None => f(&self.kernel),
        }
    }
}
impl ConformanceTarget for CompleteKernelTarget {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        self.kernel.identity()
    }
    fn fixture_values(
        &self,
        scenario: &ScenarioContext,
        contract: &ess_conformance::fixtures::Contract,
    ) -> Result<BTreeMap<String, Node>, TargetError> {
        self.prepare(scenario);
        self.with(|t| t.fixture_values(scenario, contract))
    }
    fn begin_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        if self.knowledge.borrow().is_none() {
            self.prepare(scenario);
        }
        self.with(|t| t.begin_scenario(scenario))
    }
    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        self.with(|t| t.execute_command(request))
    }
    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        self.with(|t| t.query_view(request))
    }
    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        self.with(|t| t.observe_events(request))
    }
    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        self.with(|t| t.configure_external_outcome(request))
    }
    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        self.with(|t| t.redeliver_event(request))
    }
    fn end_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        let result = self.with(|t| t.end_scenario(scenario));
        *self.knowledge.borrow_mut() = None;
        result
    }
}
