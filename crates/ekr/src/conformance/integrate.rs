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
//! * **Stored state.** The counts say nothing of what was written, so the target reads the store
//!   back through the read verbs a user runs (`ekr head`, `snapshot`, `transactions`, `ontology`
//!   and `explain --documents`) before and after the command. `applied` is reported only when the
//!   store holds what the report and the document say ([`IntegrateTarget::check_applied`]), and
//!   `refused` only when nothing moved ([`IntegrateTarget::check_unchanged`]); the refused store
//!   holds revision 1's assertion beside the refusing seed, a valid peer the refusal must leave
//!   as it was. A disagreement is a target error, and the scenario does not pass.

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
            // Revision 1 gives the refused store a valid assertion the document does not touch,
            // which `check_unchanged` holds to what it was.
            let revision_1 = self.kernel.manifest.setup.revision_1.clone();
            self.kernel.establish_commit(&revision_1)?;
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
        // `strict`, when the suite gives it, is `--strict`.
        let strict = match request.input.get("strict") {
            None | Some(ess_primitives::node::Node::Null) => false,
            Some(ess_primitives::node::Node::Bool(strict)) => *strict,
            Some(other) => {
                return Err(unavailable(
                    "reading `strict`",
                    format!("expected a boolean, found {}", other.type_name()),
                ))
            }
        };
        let options: Vec<std::ffi::OsString> = vec![
            "ekr".into(),
            "--host".into(),
            host.into_os_string(),
            "--store".into(),
            store.into_os_string(),
            "--backend".into(),
            backend.into(),
        ];
        // The store as a user reads it, through the same read verbs.
        let read = |args: &[&str]| -> Result<serde_json::Value, String> {
            let mut argv = options.clone();
            argv.extend(args.iter().map(std::ffi::OsString::from));
            let printed = crate::cli::run(argv, &|| self.kernel.tick(), &mut std::io::empty())
                .map_err(|failure| format!("`ekr {}`: {failure}", args.join(" ")))?;
            serde_json::from_str(&printed)
                .map_err(|e| format!("`ekr {}` printed no JSON: {e}", args.join(" ")))
        };
        let before = Self::store_state(&read)
            .map_err(|e| unavailable("reading the store before the extraction", e))?;
        let staged = std::fs::read_to_string(&document)
            .map_err(|e| unavailable("reading the staged extraction document", e))?;
        let mut argv = options.clone();
        argv.push("apply-extraction".into());
        if strict {
            argv.push("--strict".into());
        }
        argv.push(document.into_os_string());
        let mut stdin = std::io::empty();
        let mut result = SemanticCommandResult::undeclared();
        match crate::cli::run(argv, &|| self.kernel.tick(), &mut stdin) {
            Ok(printed) => {
                let report: serde_json::Value = serde_json::from_str(&printed)
                    .map_err(|e| unavailable("reading the extraction report", e))?;
                Self::check_applied(&read, &staged, &report, &before)
                    .map_err(|e| unavailable("reading what the store holds after `applied`", e))?;
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
                Self::check_unchanged(&read, &before)
                    .map_err(|e| unavailable("reading what the store holds after `refused`", e))?;
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

    /// The store as `read` answers `ekr head`, `ekr snapshot` and `ekr transactions` (every
    /// retained transaction, a proposed one too), one JSON object keyed by verb, for
    /// [`Self::check_applied`] and [`Self::check_unchanged`] to compare a later read with. `read`
    /// runs one read verb of `ekr` on the store, given its arguments after the store's options,
    /// and answers what it printed.
    ///
    /// # Errors
    ///
    /// A read `read` refuses.
    pub fn store_state(
        read: &dyn Fn(&[&str]) -> Result<serde_json::Value, String>,
    ) -> Result<serde_json::Value, String> {
        Ok(serde_json::json!({
            "head": read(&["head"])?,
            "snapshot": read(&["snapshot"])?,
            "transactions": read(&["transactions"])?,
        }))
    }

    /// `ekr.integrate.ApplyExtraction/refused` says nothing was proposed or written: the head, the
    /// knowledge it holds (every node, edge, assertion and evidence entry, so the valid
    /// assertions the store held before stay as they were) and the retained transactions are
    /// what `before`, [`Self::store_state`] before the command, read. Review finding 41 of EKR
    /// 0.0.42: the target reported the refusal without reading the store.
    ///
    /// # Errors
    ///
    /// What changed, or a read `read` refuses.
    pub fn check_unchanged(
        read: &dyn Fn(&[&str]) -> Result<serde_json::Value, String>,
        before: &serde_json::Value,
    ) -> Result<(), String> {
        let after = Self::store_state(read)?;
        for verb in ["head", "snapshot", "transactions"] {
            if after[verb] != before[verb] {
                return Err(format!(
                    "`ekr {verb}` changed under a refused document: before {}, after {}",
                    before[verb], after[verb]
                ));
            }
        }
        Ok(())
    }

    /// `ekr.integrate.ApplyExtraction/applied`, read back from the store: the report `ekr
    /// apply-extraction` printed for `document` against the store `before` read
    /// ([`Self::store_state`]) is what the store now holds. Review finding 41 of EKR 0.0.42: the
    /// target counted the report's rows, so a wrong fact or citation behind the same rows passed.
    ///
    /// * **Revisions.** The report's committed transactions are committed, one revision each,
    ///   in order from the head `before` read, and the last is the head.
    /// * **Facts.** Every fact the report does not name as rejected or held, about named things
    ///   none of which is ambiguous, is an active assertion of the head: its subject a node of
    ///   the fact's type known by one of its aliases, its predicate the relation's edge type or
    ///   the property the subject's type declares, its object the relation's object node or the
    ///   property's value, citing exactly the fact's evidence. A relation's two nodes are joined
    ///   by an edge of its type.
    /// * **Citations.** Each evidence item such a fact cites is the head's entry, field for
    ///   field, and `ekr explain --documents` of the assertion returns its payload, byte for
    ///   byte.
    ///
    /// # Errors
    ///
    /// The first disagreement, or a read `read` refuses.
    pub fn check_applied(
        read: &dyn Fn(&[&str]) -> Result<serde_json::Value, String>,
        document: &str,
        report: &serde_json::Value,
        before: &serde_json::Value,
    ) -> Result<(), String> {
        use serde_json::Value;
        let document = ekr_integrate::ExtractionDocument::from_yaml(document)
            .map_err(|e| format!("the document does not read: {e}"))?;
        let rows = |field: &str| {
            report[field]
                .as_array()
                .ok_or_else(|| format!("the report has no `{field}`"))
        };
        let revision = |head: &Value| {
            head["revision"]
                .as_u64()
                .ok_or_else(|| format!("a head without a revision: {head}"))
        };

        let mut expected = revision(&before["head"])?;
        let transactions = read(&["transactions"])?;
        for row in rows("committed")? {
            expected += 1;
            if row["revision"].as_u64() != Some(expected) {
                return Err(format!(
                    "the report's committed row {row} is not revision {expected}, the next one"
                ));
            }
            let id = &row["transaction_id"];
            let committed = transactions.as_array().is_some_and(|all| {
                all.iter()
                    .any(|t| &t["transaction_id"] == id && t["state"] == "Committed")
            });
            if !committed {
                return Err(format!(
                    "the store holds no committed transaction {id}: {transactions}"
                ));
            }
        }
        let head = revision(&read(&["head"])?)?;
        if head != expected {
            return Err(format!(
                "the head is revision {head}, the report's committed rows end at {expected}"
            ));
        }

        let snapshot = read(&["snapshot"])?;
        let graph = &snapshot["graph"]["graph"];
        let ontology = read(&["ontology"])?;
        let skipped: Vec<&str> = rows("rejected")?
            .iter()
            .chain(rows("held")?)
            .filter_map(|row| row["item"].as_str())
            .collect();
        let ambiguous: Vec<&Value> = rows("ambiguous")?
            .iter()
            .map(|row| &row["reference"])
            .collect();
        let is_ambiguous = |reference: &ekr_integrate::ExtractedReference| {
            ambiguous.iter().any(|group| {
                group["node_type"] == reference.node_type.as_str()
                    && group["aliases"].as_array().is_some_and(|aliases| {
                        aliases
                            .iter()
                            .any(|alias| reference.aliases.iter().any(|a| alias == a))
                    })
            })
        };
        let node_type = |name: &str| -> Result<&Value, String> {
            ontology["node_types"]
                .as_array()
                .and_then(|types| types.iter().find(|t| t["name"] == name))
                .ok_or_else(|| format!("the store declares no node type {name}"))
        };
        // The nodes a reference may be: of its type, known by one of its aliases.
        let nodes = |reference: &ekr_integrate::ExtractedReference| -> Result<Vec<&str>, String> {
            let type_id = &node_type(&reference.node_type)?["id"];
            Ok(graph["nodes"]
                .as_object()
                .map(|nodes| {
                    nodes
                        .iter()
                        .filter(|(_, node)| &node["type_id"] == type_id)
                        .filter(|(_, node)| {
                            reference.aliases.iter().any(|alias| {
                                node["canonical_name"] == alias.as_str()
                                    || node["aliases"]
                                        .as_array()
                                        .is_some_and(|known| known.iter().any(|k| k == alias))
                            })
                        })
                        .map(|(id, _)| id.as_str())
                        .collect()
                })
                .unwrap_or_default())
        };

        for (index, fact) in document.facts.iter().enumerate() {
            let item = format!("facts[{index}]");
            if skipped
                .iter()
                .any(|named| *named == item || named.starts_with(&format!("{item}.")))
            {
                continue;
            }
            let (subject, object) = match fact {
                ekr_integrate::ExtractedFact::Property(fact) => (&fact.subject, None),
                ekr_integrate::ExtractedFact::Relation(fact) => (&fact.subject, Some(&fact.object)),
            };
            if is_ambiguous(subject) || object.is_some_and(is_ambiguous) {
                continue;
            }
            let subjects = nodes(subject)?;
            let (predicate, objects): (Value, Vec<Value>) = match fact {
                ekr_integrate::ExtractedFact::Property(fact) => {
                    let declared = &node_type(&subject.node_type)?["properties"];
                    let property = declared
                        .as_array()
                        .and_then(|all| all.iter().find(|p| p["name"] == fact.property.as_str()))
                        .ok_or_else(|| {
                            format!(
                                "{item}: {} declares no {}",
                                subject.node_type, fact.property
                            )
                        })?;
                    let value = serde_json::to_value(&fact.value).map_err(|e| e.to_string())?;
                    (
                        serde_json::json!({"Property": property["id"]}),
                        vec![serde_json::json!({ "Value": value })],
                    )
                }
                ekr_integrate::ExtractedFact::Relation(fact) => {
                    let edge_type = ontology["edge_types"]
                        .as_array()
                        .and_then(|types| {
                            types.iter().find(|t| t["name"] == fact.relation.as_str())
                        })
                        .ok_or_else(|| {
                            format!("the store declares no edge type {}", fact.relation)
                        })?;
                    let objects = nodes(&fact.object)?;
                    let joined = graph["edges"].as_object().is_some_and(|edges| {
                        edges.values().any(|edge| {
                            edge["type_id"] == edge_type["id"]
                                && subjects.iter().any(|s| edge["source"] == *s)
                                && objects.iter().any(|o| edge["target"] == *o)
                        })
                    });
                    if !joined {
                        return Err(format!(
                            "{item}: no {} edge joins {:?} to {:?}",
                            fact.relation, subject.aliases, fact.object.aliases
                        ));
                    }
                    (
                        serde_json::json!({"Relation": edge_type["id"]}),
                        objects
                            .iter()
                            .map(|o| serde_json::json!({ "Node": o }))
                            .collect(),
                    )
                }
            };
            let cited: std::collections::BTreeSet<String> =
                fact.evidence().iter().map(ToString::to_string).collect();
            let assertion = graph["assertions"]
                .as_object()
                .and_then(|assertions| {
                    assertions.iter().find(|(_, a)| {
                        a["lifecycle"] == "Active"
                            && subjects.iter().any(|s| a["subject"]["Node"] == *s)
                            && a["predicate"] == predicate
                            && objects.contains(&a["object"])
                            && a["evidence"].as_array().is_some_and(|evidence| {
                                evidence
                                    .iter()
                                    .filter_map(Value::as_str)
                                    .map(str::to_owned)
                                    .collect::<std::collections::BTreeSet<_>>()
                                    == cited
                            })
                    })
                })
                .map(|(id, _)| id.clone())
                .ok_or_else(|| {
                    format!(
                        "{item}: the head holds no active assertion of it citing {cited:?}: {}",
                        graph["assertions"]
                    )
                })?;
            let explained = read(&["explain", "--documents", &assertion])?;
            for id in &cited {
                let item_of = document
                    .evidence
                    .iter()
                    .find(|e| e.evidence.id.to_string() == *id)
                    .ok_or_else(|| format!("{item}: the document carries no evidence {id}"))?;
                let entry = serde_json::to_value(&item_of.evidence).map_err(|e| e.to_string())?;
                if graph["evidence"][id] != entry {
                    return Err(format!(
                        "{item}: evidence {id} is {}, the document's is {entry}",
                        graph["evidence"][id]
                    ));
                }
                let payload = super::base64(&item_of.payload);
                let returned = explained["links"].as_array().is_some_and(|links| {
                    links.iter().any(|link| {
                        link["kind"] == "Evidence"
                            && link["id"] == id.as_str()
                            && link["payload"] == payload.as_str()
                            && link.get("truncated").is_none()
                    })
                });
                if !returned {
                    return Err(format!(
                        "{item}: `ekr explain --documents {assertion}` does not return evidence \
                         {id}'s payload: {explained}"
                    ));
                }
            }
        }
        Ok(())
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
