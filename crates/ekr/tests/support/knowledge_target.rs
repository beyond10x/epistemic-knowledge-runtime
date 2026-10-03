//! Runs declared commands through the real CLI; every invocation reopens its native store.
use ekr::{conformance::Provider, exit::Failure};
use ess_conformance::target::*;
use ess_primitives::{facts::Number, node::Node};
use std::{
    cell::{Cell, RefCell},
    ffi::OsString,
    path::{Path, PathBuf},
};

pub struct KnowledgeTarget {
    provider: Provider,
    work: PathBuf,
    host: PathBuf,
    scenario: Cell<u64>,
    invocation: Cell<u64>,
    discard_retained_state: bool,
    observed: RefCell<Vec<ObservedEvent>>,
}

pub fn unavailable(operation: &str, error: impl std::fmt::Display) -> TargetError {
    TargetError::unavailable(operation, error.to_string())
}

/// Exact integer conversion: neither number tokens nor result values pass through f64.
pub fn node(value: serde_json::Value) -> Result<Node, TargetError> {
    Ok(match value {
        serde_json::Value::Null => Node::Null,
        serde_json::Value::Bool(value) => Node::Bool(value),
        serde_json::Value::String(value) => Node::Text(value),
        serde_json::Value::Number(value) => {
            Node::Number(Number::from(value.as_i64().ok_or_else(|| {
                TargetError::unsupported(
                    "retained result number",
                    "knowledge contracts require an exact i64",
                )
            })?))
        }
        serde_json::Value::Array(values) => {
            Node::Seq(values.into_iter().map(node).collect::<Result<_, _>>()?)
        }
        serde_json::Value::Object(values) => Node::Map(
            values
                .into_iter()
                .map(|(key, value)| Ok((key, node(value)?)))
                .collect::<Result<_, TargetError>>()?,
        ),
    })
}

/// ESS preserves numeric meaning but may serialize an integral value with a decimal point.
/// The generated transport's Integer contract requires an integer JSON token.
pub fn json_input(value: &Node) -> Result<serde_json::Value, TargetError> {
    Ok(match value {
        Node::Null => serde_json::Value::Null,
        Node::Bool(value) => (*value).into(),
        Node::Text(value) => value.clone().into(),
        Node::Number(value) => value
            .as_i64()
            .ok_or_else(|| {
                TargetError::unsupported("knowledge input number", "expected exact i64")
            })?
            .into(),
        Node::Seq(values) => {
            serde_json::Value::Array(values.iter().map(json_input).collect::<Result<_, _>>()?)
        }
        Node::Map(values) => serde_json::Value::Object(
            values
                .iter()
                .map(|(key, value)| Ok((key.clone(), json_input(value)?)))
                .collect::<Result<_, TargetError>>()?,
        ),
    })
}

impl KnowledgeTarget {
    pub fn new(provider: Provider, host: &Path, work: &Path, discard_retained_state: bool) -> Self {
        Self {
            provider,
            host: host.into(),
            work: work.into(),
            scenario: Cell::new(0),
            invocation: Cell::new(0),
            discard_retained_state,
            observed: RefCell::new(vec![]),
        }
    }
    fn execute(
        &self,
        request: &SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let command = request.command.to_string();
        let (domain, verb, action, outcome, event, field) = match command.as_str() {
            "ekr.observe.ImportObservation" => (
                "observe",
                "observe",
                "import",
                "answered",
                "ImportObservationResult",
                "receipt",
            ),
            "ekr.observe.ListObservations" => (
                "observe",
                "observe",
                "list",
                "listed",
                "ObservationsListed",
                "observations",
            ),
            "ekr.observe.ShowObservation" => (
                "observe",
                "observe",
                "show",
                "shown",
                "ObservationShown",
                "retained",
            ),
            "ekr.integrate.ImportInterpretation" => (
                "integrate",
                "incubate",
                "import",
                "answered",
                "ImportInterpretationResult",
                "receipt",
            ),
            "ekr.integrate.ListInterpretations" => (
                "integrate",
                "incubate",
                "list",
                "answered",
                "ListInterpretationsResult",
                "interpretations",
            ),
            "ekr.integrate.ShowInterpretation" => (
                "integrate",
                "incubate",
                "show",
                "answered",
                "ShowInterpretationResult",
                "",
            ),
            _ => {
                return Err(TargetError::unsupported(
                    command,
                    "the knowledge target exposes only observation and incubation commands",
                ))
            }
        };
        let actor = if domain == "observe" {
            "ekr.observe.KnowledgeSupplier"
        } else {
            "ekr.integrate.KnowledgeProposer"
        };
        if request
            .actor
            .as_ref()
            .is_some_and(|supplied| supplied.to_string() != actor)
        {
            return Err(TargetError::unsupported(
                command,
                format!("the host binds this operation to {actor}"),
            ));
        }
        let count = self.invocation.get();
        self.invocation.set(count + 1);
        let directory =
            self.work
                .join(self.scenario.get().to_string())
                .join(if self.discard_retained_state {
                    count.to_string()
                } else {
                    "retained".into()
                });
        std::fs::create_dir_all(&directory)
            .map_err(|e| unavailable("creating scenario directory", e))?;
        let (backend, store) = match self.provider {
            Provider::File => ("file", directory.join("store")),
            Provider::Sqlite => ("sqlite", directory.join("store.sqlite")),
        };
        // Establish only an empty provider namespace, never a canonical seed or fixture facts.
        // Public CLI operations deliberately open existing stores. Runtime owns creation.
        if !store.exists() {
            let host = ekr::host::CliHostConfigurationV1::from_json(
                &std::fs::read(&self.host).map_err(|e| unavailable("reading host", e))?,
            )
            .map_err(|e| unavailable("decoding host", e))?;
            let runtime = match self.provider {
                Provider::File => {
                    ekr_kernel::Runtime::file(&store, &host.tenant, host.context, host.authority)
                }
                Provider::Sqlite => {
                    ekr_kernel::Runtime::sqlite(&store, &host.tenant, host.context, host.authority)
                }
            }
            .map_err(|e| unavailable("creating empty provider", e))?;
            drop(runtime);
        }
        let mut argv: Vec<OsString> = vec![
            "ekr".into(),
            "--host".into(),
            self.host.clone().into(),
            "--store".into(),
            store.into(),
            "--backend".into(),
            backend.into(),
            "--full-replay".into(),
            verb.into(),
            action.into(),
        ];
        let input = |key: &str| {
            request
                .input
                .get(key)
                .ok_or_else(|| unavailable("reading command input", format!("missing {key}")))
        };
        if action == "import" {
            let document = directory.join("input.json");
            let mut body = json_input(input("document")?)?;
            if domain == "integrate" {
                body =
                    serde_json::json!({"document":body,"payload":json_input(input("payload")?)?});
            }
            std::fs::write(
                &document,
                serde_json::to_vec(&body).map_err(|e| unavailable("encoding import", e))?,
            )
            .map_err(|e| unavailable("staging import", e))?;
            argv.push(document.into());
        } else if action == "show" && domain == "observe" {
            argv.push(
                input("observation_id")?
                    .as_text()
                    .ok_or_else(|| unavailable("reading identity", "expected text"))?
                    .into(),
            );
        } else if action == "show" {
            let version = input("version")?
                .as_map()
                .ok_or_else(|| unavailable("reading version", "expected map"))?;
            for key in ["interpretation_id", "version", "document_digest"] {
                let value = version
                    .get(key)
                    .ok_or_else(|| unavailable("reading version", key))?;
                argv.push(
                    match value {
                        Node::Text(value) => value.clone(),
                        Node::Number(value) => value.to_string(),
                        _ => return Err(unavailable("reading version field", key)),
                    }
                    .into(),
                );
            }
        }
        let mut result = SemanticCommandResult::undeclared();
        let outcome_ref = |name: &str| {
            serde_json::from_value(serde_json::json!({"command":command,"outcome":name}))
                .map_err(|e| unavailable("naming outcome", e))
        };
        match ekr::cli::run(argv, &|| ekr_core::Timestamp::EPOCH, &mut std::io::empty()) {
            Ok(printed) => {
                let value = node(
                    serde_json::from_str(&printed)
                        .map_err(|e| unavailable("decoding actual CLI result", e))?,
                )?;
                let mut observed = ObservedEvent::new(
                    format!("ekr.{domain}.{event}")
                        .parse()
                        .map_err(|e| unavailable("naming event", e))?,
                )
                .in_activity(request.correlation.clone())
                .at(count);
                if field.is_empty() {
                    observed.payload = value
                        .as_map()
                        .ok_or_else(|| unavailable("reading interpretation", "expected record"))?
                        .clone();
                } else {
                    observed = observed.with(field, value);
                }
                result.outcome = Some(outcome_ref(outcome)?);
                result.response = Some(observed.payload.clone());
                result.direct_events.push(observed.clone());
                self.observed.borrow_mut().push(observed);
            }
            Err(Failure::Refused { name, message })
                if name == format!("ekr.{domain}.KnowledgeRefused") =>
            {
                result.outcome = Some(outcome_ref("refused")?);
                result.error = Some(
                    DeclaredErrorValue::new(
                        name.parse().map_err(|e| unavailable("naming refusal", e))?,
                    )
                    .with("code", Node::Text(name.to_owned()))
                    .with("reason", Node::Text(message)),
                );
            }
            Err(error) => return Err(unavailable("running knowledge command", error)),
        }
        Ok(result)
    }
}

impl ConformanceTarget for KnowledgeTarget {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            format!(
                "ekr knowledge CLI ({:?}; discard={})",
                self.provider, self.discard_retained_state
            ),
            env!("CARGO_PKG_VERSION"),
        ))
    }
    fn begin_scenario(&self, _: &ScenarioContext) -> Result<(), TargetError> {
        self.scenario.set(self.scenario.get() + 1);
        self.invocation.set(0);
        self.observed.borrow_mut().clear();
        Ok(())
    }
    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        self.execute(&request)
    }
    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        Err(TargetError::unsupported(
            request.view.to_string(),
            "this target exposes command read results",
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
            .filter(|event| {
                event.event == request.event
                    && event.correlation.as_ref() == Some(&request.correlation)
            })
            .cloned()
            .collect())
    }
    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        // The authored refusal supplies an invalid local declaration in its input. No external
        // setup is needed. Do not save this request or use it to select the command's result.
        if request.force.command.to_string() == "ekr.integrate.ImportInterpretation"
            && request.force.outcome.to_string() == "refused"
        {
            return Ok(());
        }
        Err(TargetError::unsupported(
            request.force.to_string(),
            "no external fixture for this branch",
        ))
    }
    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        Err(TargetError::unsupported(
            request.event.to_string(),
            "retention declares no delivery binding",
        ))
    }
    fn end_scenario(&self, _: &ScenarioContext) -> Result<(), TargetError> {
        self.observed.borrow_mut().clear();
        Ok(())
    }
}
