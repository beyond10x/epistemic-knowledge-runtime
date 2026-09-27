//! The ESS conformance target over `ekr_views::project`, on one native provider.
//!
//! * **Isolation.** Every scenario gets a fresh directory below the caller's work directory, and
//!   every store a scenario names is a fresh provider root inside it.
//! * **Stores.** A `store` input names a [`Fixture`]; the first command naming it in a scenario
//!   builds it through the real kernel handlers, and every later command reads the same store.
//! * **External outcomes.** A control establishes the state its branch declares and never selects
//!   the reported outcome: `not-found` builds the named store as the seed alone, so the requested
//!   revision does not exist, and `not-seeded` opens the named store's provider without seeding
//!   it. The renderer then answers whatever it answers.
//! * **Observations.** A command reports the `GraphProjected` event built from the summary the
//!   renderer returned, and every event the provider log gained while it ran, by its logged name —
//!   so a render that wrote anything is caught by the suite's `expect_no_event` steps.
//! * **Responses.** None: no admitted views scenario observes a command response, so a projection
//!   here would be checked by nothing in the suite. The document reaches the suite through
//!   `projection_hash` and the counts.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::path::PathBuf;

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use ekr_views::{GraphProjected, ProjectError};
use ess_conformance::target::{
    ConformanceTarget, DeclaredErrorValue, EventObservationRequest, ExternalOutcomeControl,
    ImplementationIdentity, ObservedEvent, RedeliveryRequest, ScenarioContext,
    SemanticCommandRequest, SemanticCommandResult, SemanticViewRequest, SemanticViewResult,
    TargetError,
};
use ess_primitives::facts::Number;
use ess_primitives::node::Node;

use super::fixtures::{self, Fixture, Provider};

const PROJECT_GRAPH: &str = "ekr.views.ProjectGraph";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    RevisionAbsent,
    Unseeded,
}

struct Scenario {
    directory: PathBuf,
    control: Option<Control>,
    stores: BTreeMap<String, Runtime>,
    events: Vec<ObservedEvent>,
}

/// One projection the target returned: which store, which revision, and its summary.
#[derive(Clone, Debug)]
pub struct Answered {
    pub store: String,
    pub summary: GraphProjected,
}

pub struct ViewsTarget {
    provider: Provider,
    work: PathBuf,
    opened: Cell<u64>,
    scenario: RefCell<Option<Scenario>>,
    answered: RefCell<Vec<Answered>>,
}

fn unavailable(operation: &str, detail: impl std::fmt::Display) -> TargetError {
    TargetError::unavailable(operation, detail.to_string())
}

fn integer(value: u64) -> Result<Node, TargetError> {
    i64::try_from(value)
        .map(|exact| Node::Number(Number::from(exact)))
        .map_err(|_| unavailable("projecting an integer", format!("{value} exceeds i64")))
}

fn event(summary: &GraphProjected) -> Result<ObservedEvent, TargetError> {
    let name = "ekr.views.GraphProjected"
        .parse()
        .map_err(|e| unavailable("naming GraphProjected", e))?;
    let mut observed = ObservedEvent::new(name);
    for (field, value) in [
        ("revision", summary.revision),
        ("head", summary.head),
        ("nodes", summary.nodes),
        ("edges", summary.edges),
        ("assertions", summary.assertions),
        ("evidence", summary.evidence),
        ("schema_versions", summary.schema_versions),
        ("revisions", summary.revisions),
        ("transactions", summary.transactions),
        ("node_types", summary.node_types),
        ("edge_types", summary.edge_types),
        ("properties", summary.properties),
        ("edge_assertions", summary.edge_assertions),
        ("retracted_assertions", summary.retracted_assertions),
        ("retained_evidence", summary.retained_evidence),
    ] {
        observed = observed.with(field, integer(value)?);
    }
    Ok(observed.with(
        "projection_hash",
        Node::Text(summary.projection_hash.clone()),
    ))
}

fn outcome_ref(outcome: &str) -> Result<ess_conformance::scenario::OutcomeRef, TargetError> {
    serde_json::from_value(serde_json::json!({ "command": PROJECT_GRAPH, "outcome": outcome }))
        .map_err(|e| unavailable("naming a declared outcome", format!("{outcome}: {e}")))
}

fn error(name: &str) -> Result<DeclaredErrorValue, TargetError> {
    name.parse()
        .map(DeclaredErrorValue::new)
        .map_err(|e| unavailable("naming a declared error", format!("{name}: {e}")))
}

fn revision_input(request: &SemanticCommandRequest) -> Result<Option<RevisionNumber>, TargetError> {
    match request.input.get("at") {
        None | Some(Node::Null) => Ok(None),
        Some(Node::Number(number)) => number
            .as_i64()
            .and_then(|exact| u64::try_from(exact).ok())
            .map(|exact| Some(RevisionNumber::new(exact)))
            .ok_or_else(|| unavailable("reading `at`", format!("{number} is not a revision"))),
        Some(other) => Err(unavailable(
            "reading `at`",
            format!("{} is not a revision number", other.type_name()),
        )),
    }
}

impl ViewsTarget {
    pub fn new(provider: Provider, work: PathBuf) -> Self {
        Self {
            provider,
            work,
            opened: Cell::new(0),
            scenario: RefCell::new(None),
            answered: RefCell::new(Vec::new()),
        }
    }

    /// Every projection this target returned, in order.
    pub fn answered(&self) -> Vec<Answered> {
        self.answered.borrow().clone()
    }
}

impl ConformanceTarget for ViewsTarget {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            format!("ekr-views ({} provider)", self.provider.name()),
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn begin_scenario(&self, _scenario: &ScenarioContext) -> Result<(), TargetError> {
        let number = self.opened.get() + 1;
        self.opened.set(number);
        let directory = self.work.join(format!("scenario-{number:04}"));
        if directory.exists() {
            return Err(unavailable(
                "opening an isolated scenario",
                format!("{} already exists", directory.display()),
            ));
        }
        std::fs::create_dir_all(&directory)
            .map_err(|e| unavailable("opening an isolated scenario", e))?;
        *self.scenario.borrow_mut() = Some(Scenario {
            directory,
            control: None,
            stores: BTreeMap::new(),
            events: Vec::new(),
        });
        Ok(())
    }

    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        if request.command.to_string() != PROJECT_GRAPH {
            return Err(TargetError::unsupported(
                format!("executing `{}`", request.command),
                "the views target answers ekr.views.ProjectGraph only",
            ));
        }
        let store = match request.input.get("store") {
            Some(Node::Text(store)) => store.clone(),
            _ => return Err(unavailable("reading `store`", "not a store location")),
        };
        let at = revision_input(&request)?;
        let mut guard = self.scenario.borrow_mut();
        let scenario = guard
            .as_mut()
            .ok_or_else(|| unavailable("executing a command", "no scenario is open"))?;
        let control = scenario.control.take();
        if control.is_some() && scenario.stores.contains_key(&store) {
            return Err(unavailable(
                "establishing an external outcome",
                format!("the store `{store}` is already built in this scenario"),
            ));
        }
        if !scenario.stores.contains_key(&store) {
            let fixture = Fixture::named(&store).ok_or_else(|| {
                TargetError::unsupported(
                    format!("opening the store `{store}`"),
                    "no fixture of that name",
                )
            })?;
            let root = scenario.directory.join(&store);
            let runtime = fixtures::open(&root, self.provider);
            match control {
                Some(Control::Unseeded) => {}
                Some(Control::RevisionAbsent) => Fixture::SeedOnly.build(&runtime),
                None => fixture.build(&runtime),
            }
            scenario.stores.insert(store.clone(), runtime);
        }
        let runtime = &scenario.stores[&store];
        let before = runtime
            .published_events()
            .map_err(|e| unavailable("reading the provider log", e))?
            .len();
        let rendered = ekr_views::project(runtime, at);
        let gained = runtime
            .published_events()
            .map_err(|e| unavailable("reading the provider log", e))?;
        let mut result = match rendered {
            Ok(rendered) => {
                self.answered.borrow_mut().push(Answered {
                    store: store.clone(),
                    summary: rendered.summary.clone(),
                });
                SemanticCommandResult::took(outcome_ref("projected")?)
                    .emitting(event(&rendered.summary)?)
            }
            Err(ProjectError::RevisionNotFound { requested, head }) => {
                SemanticCommandResult::took(outcome_ref("not-found")?).with_error(
                    error("ekr.views.RevisionNotFound")?
                        .with("requested", integer(requested.get())?)
                        .with("head", integer(head.get())?),
                )
            }
            Err(ProjectError::NotSeeded { requested }) => {
                let mut refusal = error("ekr.views.NotSeeded")?;
                if let Some(requested) = requested {
                    refusal = refusal.with("requested", integer(requested.get())?);
                }
                SemanticCommandResult::took(outcome_ref("not-seeded")?).with_error(refusal)
            }
            Err(other) => return Err(unavailable("projecting the graph", other)),
        };
        for written in gained.iter().skip(before) {
            let name = written
                .name
                .parse()
                .map_err(|e| unavailable("naming a logged event", e))?;
            result = result.emitting(ObservedEvent::new(name).at(written.position));
        }
        scenario.events.extend(result.direct_events.iter().cloned());
        Ok(result)
    }

    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        Err(TargetError::unsupported(
            format!("reading `{}`", request.view),
            "ekr.views declares no view",
        ))
    }

    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        let guard = self.scenario.borrow();
        let scenario = guard
            .as_ref()
            .ok_or_else(|| unavailable("observing events", "no scenario is open"))?;
        Ok(scenario
            .events
            .iter()
            .filter(|event| event.event == request.event)
            .cloned()
            .collect())
    }

    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        let control = match (
            request.force.command.to_string().as_str(),
            request.force.outcome.to_string().as_str(),
        ) {
            (PROJECT_GRAPH, "not-found") => Control::RevisionAbsent,
            (PROJECT_GRAPH, "not-seeded") => Control::Unseeded,
            _ => {
                return Err(TargetError::unsupported(
                    format!("establishing `{}`", request.force),
                    "the views target knows no precondition for that branch",
                ))
            }
        };
        let mut guard = self.scenario.borrow_mut();
        let scenario = guard
            .as_mut()
            .ok_or_else(|| unavailable("configuring an external outcome", "no scenario is open"))?;
        scenario.control = Some(control);
        Ok(())
    }

    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        Err(TargetError::unsupported(
            format!("redelivering `{}`", request.event),
            "ekr.views declares no binding",
        ))
    }

    fn end_scenario(&self, _scenario: &ScenarioContext) -> Result<(), TargetError> {
        self.scenario.borrow_mut().take();
        Ok(())
    }
}
