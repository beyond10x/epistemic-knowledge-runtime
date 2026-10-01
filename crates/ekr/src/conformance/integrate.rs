//! The ESS conformance target over `ekr.integrate`'s one command, `ekr.integrate.ApplyExtraction`
//! (`story:extraction-verb-shares-the-sdk-path`).
//!
//! [`IntegrateTarget`] answers the `ekr-integrate` component's suite by running `ekr
//! apply-extraction` itself, through [`crate::cli::run`], against an isolated store the
//! [`KernelTarget`] it wraps seeds: the same verb, the same in-process SDK routine and the same
//! reader a user runs. The component publishes one event, `ekr.integrate.ExtractionApplied`, which
//! the target observes from the report the verb printed: how many transactions committed, how many
//! parts were rejected, how many named things were ambiguous and how many facts the store already
//! held. The transactions the verb proposes publish the kernel's own events; those belong to the
//! `ekr-kernel` component and its suite, and are not this component's to report.
//!
//! * **Lineage.** `applied` runs on the manifest's seed and revision 1, as a kernel command does.
//!   The external `refused` branch establishes the state it declares — a head whose ontology the
//!   reader refuses the document against — by seeding the manifest's `extraction.refusing_seed`
//!   instead, in which `Person` has a subtype, so every reference to a `Person` is refused as
//!   `reference-type-has-subtypes` before anything is written. It never selects the outcome: the
//!   verb's own reader does.
//! * **Documents.** `extraction_document` is staged as a kernel document is, from the manifest's
//!   `documents`.

use std::cell::{Cell, RefCell};
use std::path::Path;

use ess_conformance::target::{
    ConformanceTarget, EventObservationRequest, ExternalOutcomeControl, ImplementationIdentity,
    ObservedEvent, RedeliveryRequest, ScenarioContext, SemanticCommandRequest,
    SemanticCommandResult, SemanticViewRequest, SemanticViewResult, TargetError,
};

use super::{
    error, event_ref, integer, outcome_ref, text, text_input, unavailable, KernelTarget, Provider,
};
use crate::exit::Failure;

const APPLY_EXTRACTION: &str = "ekr.integrate.ApplyExtraction";
const APPLIER: &str = "ekr.integrate.Applier";

/// The conformance target over `ekr apply-extraction` on one native provider.
pub struct IntegrateTarget {
    kernel: KernelTarget,
    /// Whether the open scenario forces the external `refused` branch.
    refusing: Cell<bool>,
    /// Every event this component published in the open scenario.
    observed: RefCell<Vec<ObservedEvent>>,
}

impl IntegrateTarget {
    /// Opens a target over `provider`, reading the kernel target's `manifest.json` and host
    /// documents from `fixtures` and creating one isolated provider root per scenario below `work`.
    ///
    /// # Errors
    ///
    /// An unreadable or malformed manifest or host document, or a manifest that names no
    /// `extraction` fixtures.
    pub fn new(provider: Provider, fixtures: &Path, work: &Path) -> Result<Self, String> {
        let kernel = KernelTarget::new(provider, fixtures, work)?;
        if kernel.manifest.extraction.is_none() {
            return Err("the fixture manifest names no `extraction` fixtures".to_owned());
        }
        Ok(Self {
            kernel,
            refusing: Cell::new(false),
            observed: RefCell::new(Vec::new()),
        })
    }

    /// Seeds the lineage the branch declares, then runs `ekr apply-extraction` on the staged
    /// document and reports what it answered.
    fn apply(
        &self,
        request: &SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        if let Some(actor) = &request.actor {
            if actor.to_string() != APPLIER {
                return Err(TargetError::unsupported(
                    format!("`{APPLY_EXTRACTION}` as `{actor}`"),
                    format!("the host binds `{APPLY_EXTRACTION}` to `{APPLIER}`"),
                ));
            }
        }
        if self.refusing.replace(false) {
            let seed = self
                .kernel
                .manifest
                .extraction
                .as_ref()
                .map(|fixtures| fixtures.refusing_seed.clone())
                .ok_or_else(|| unavailable("establishing a refusing head", "no refusing seed"))?;
            if let Some(scenario) = self.kernel.scenario.borrow_mut().as_mut() {
                scenario.prepared = true;
            }
            self.kernel.establish_seed(&seed)?;
        } else {
            self.kernel.prepare(APPLY_EXTRACTION)?;
        }
        let document = self
            .kernel
            .stage(&text_input(request, "extraction_document")?)?;
        let directory = self.kernel.directory()?;
        let (backend, store) = match self.kernel.provider {
            Provider::File => ("file", directory.join("store")),
            Provider::Sqlite => ("sqlite", directory.join("store.sqlite")),
        };
        let host = {
            let scenario = self.kernel.scenario.borrow();
            let id = scenario.as_ref().map(|s| s.id.as_str()).unwrap_or_default();
            let name = self
                .kernel
                .manifest
                .setups
                .get(id)
                .map_or(self.kernel.manifest.host.as_str(), |setup| {
                    setup.host.as_str()
                });
            self.kernel.fixtures.join(name)
        };
        let argv = [
            "ekr".into(),
            "--host".into(),
            host.into_os_string(),
            "--store".into(),
            store.into_os_string(),
            "--backend".into(),
            backend.into(),
            "apply-extraction".into(),
            document.into_os_string(),
        ];
        let mut stdin = std::io::empty();
        let mut result = SemanticCommandResult::undeclared();
        match crate::cli::run(argv, &|| self.kernel.tick(), &mut stdin) {
            Ok(printed) => {
                let report: serde_json::Value = serde_json::from_str(&printed)
                    .map_err(|e| unavailable("reading the extraction report", e))?;
                let count = |field: &str| {
                    report[field]
                        .as_array()
                        .ok_or_else(|| {
                            unavailable("reading the extraction report", format!("no `{field}`"))
                        })
                        .and_then(|rows| integer(rows.len() as u64))
                };
                let event = ObservedEvent::new(event_ref("ekr.integrate.ExtractionApplied")?)
                    .with("committed", count("committed")?)
                    .with("rejected", count("rejected")?)
                    .with("ambiguous", count("ambiguous")?)
                    .with("held", count("held")?);
                self.observed.borrow_mut().push(event.clone());
                result.outcome = Some(outcome_ref(APPLY_EXTRACTION, "applied")?);
                result.direct_events = vec![event];
            }
            Err(Failure::Refused { name, message }) => {
                result.outcome = Some(outcome_ref(APPLY_EXTRACTION, "refused")?);
                result.error = Some(
                    error("ekr.integrate.ExtractionRefused")?
                        .with("code", text(name))
                        .with("name", text(message)),
                );
            }
            Err(other) => return Err(unavailable("applying the extraction document", other)),
        }
        Ok(result)
    }
}

impl ConformanceTarget for IntegrateTarget {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        let provider = match self.kernel.provider {
            Provider::File => "file",
            Provider::Sqlite => "sqlite",
        };
        Ok(ImplementationIdentity::new(
            format!("ekr-integrate ({provider} provider)"),
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn begin_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        self.refusing.set(false);
        self.observed.borrow_mut().clear();
        self.kernel.begin_scenario(scenario)
    }

    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let command = request.command.to_string();
        if command != APPLY_EXTRACTION {
            return Err(TargetError::unsupported(
                format!("the command `{command}`"),
                "the integrate target answers only ekr-integrate's commands",
            ));
        }
        self.apply(&request)
    }

    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        Err(TargetError::unsupported(
            format!("the view `{}`", request.view),
            "ekr-integrate declares no views",
        ))
    }

    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        Ok(self
            .observed
            .borrow()
            .iter()
            .filter(|event| event.event == request.event)
            .cloned()
            .collect())
    }

    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        if request.force.command.to_string() == APPLY_EXTRACTION
            && request.force.outcome.to_string() == "refused"
        {
            self.refusing.set(true);
            return Ok(());
        }
        Err(TargetError::unsupported(
            format!("establishing `{}`", request.force),
            "the integrate target knows no precondition for that branch",
        ))
    }

    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        Err(TargetError::unsupported(
            format!("delivering `{}` again", request.event),
            "ekr-integrate declares no bindings",
        ))
    }

    fn end_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        self.observed.borrow_mut().clear();
        self.kernel.end_scenario(scenario)
    }
}
