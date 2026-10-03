//! `story:extraction-verb-shares-the-sdk-path`: the `ekr-integrate` component's conformance target,
//! [`IntegrateTarget`], answers both branches of `ekr.integrate.ApplyExtraction` on both native
//! providers, and publishes `ekr.integrate.ExtractionApplied` for an applied document.
//!
//! `systems/ekr` declares the command on the `ekr-integrate` component; the suite ESS synthesizes
//! for it (`ess conform synthesize --component ekr-integrate`) is committed as
//! `systems/ekr/conformance/integrate-suite.json`. The original extraction scenarios,
//! `ekr.integrate.ApplyExtraction/outcome/applied` and `.../outcome/refused`, remain in the
//! full inventory alongside the knowledge-import and schema-proposal obligations. The committed suite
//! is admitted and run with `Runner::run_admitted` on both providers, and its scenario set, total,
//! floor and each scenario's step floors are held against the hand-committed
//! `systems/ekr/conformance/integrate-baseline.json`, never against the suite under test, so a
//! scenario lost, or kept by name with its expectations stripped, and regenerated is named rather
//! than absorbed. `integrate-provenance.json` records how the suite was synthesized; there are no
//! authored integrate scenarios. Freshness of the committed suite against the specification is
//! `task conform-fresh`'s synthesis-and-`cmp` step. A further case drives the target directly
//! through the same `ConformanceTarget` calls the runner makes, and reads the event's counts.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ekr::conformance::{IntegrateTarget, Provider};
use ess_conformance::target::{
    ConformanceTarget, Deadline, EventObservationRequest, ExternalOutcomeControl, ScenarioContext,
    SemanticCommandRequest, SemanticCommandResult,
};
use ess_conformance::{AdmittedSuite, CountReport, CountStatus, ExecutedRun, Runner};
use ess_primitives::ids::CorrelationId;
use ess_primitives::node::Node;

const SUITE: &str = "systems/ekr/conformance/integrate-suite.json";
const BASELINE: &str = "systems/ekr/conformance/integrate-baseline.json";
const PROVENANCE: &str = "systems/ekr/conformance/integrate-provenance.json";

/// The workspace root, resolved per process: this crate is `<root>/crates/ekr`.
fn workspace_root() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .ancestors()
    .nth(2)
    .expect("crates/ekr sits two levels below the workspace root")
    .to_path_buf()
}

fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn json(relative: &str) -> serde_json::Value {
    serde_json::from_str(&read(relative)).unwrap_or_else(|e| panic!("{relative}: {e}"))
}

fn fixtures() -> PathBuf {
    workspace_root().join("crates/ekr/tests/fixtures/conformance")
}

fn signed_target(provider: Provider, work: &std::path::Path) -> IntegrateTarget {
    use ring::signature::{Ed25519KeyPair, KeyPair};
    // Synthetic test identity is supplied independently of all command inputs.
    let key = Ed25519KeyPair::from_seed_unchecked(&[71; 32]).unwrap();
    IntegrateTarget::new(provider, &fixtures(), work)
        .unwrap()
        .with_fixture_reviewer(key.public_key().as_ref().to_vec(), move |message| {
            key.sign(message).as_ref().to_vec()
        })
}

/// The committed integrate baseline, in the views baseline's shape.
struct Baseline {
    suite_version: String,
    total: u64,
    answered_floor: u64,
    unavailable_ceiling: u64,
    scenarios: BTreeSet<String>,
    step_floors: BTreeMap<String, BTreeMap<String, u64>>,
}

fn baseline() -> Baseline {
    let value = json(BASELINE);
    assert_eq!(value["format"], "ekr.conformance-baseline/1");
    assert_eq!(value["suite"], SUITE);
    assert_eq!(value["component"], "ekr-integrate");
    assert_eq!(value["providers"], serde_json::json!(["file", "sqlite"]));
    assert_eq!(
        value["quarantine"],
        serde_json::json!([]),
        "the integrate baseline admits no quarantine"
    );
    assert_eq!(
        value["authored"],
        serde_json::json!({}),
        "there are no authored integrate scenarios"
    );
    let number = |key: &str| value[key].as_u64().unwrap_or_else(|| panic!("{key}"));
    let listed: Vec<String> = value["scenarios"]
        .as_array()
        .expect("scenario names")
        .iter()
        .map(|name| name.as_str().expect("name").to_owned())
        .collect();
    let scenarios: BTreeSet<String> = listed.iter().cloned().collect();
    assert_eq!(
        listed.len(),
        scenarios.len(),
        "{BASELINE} names a scenario twice"
    );
    let step_floors: BTreeMap<String, BTreeMap<String, u64>> = value["step_floors"]
        .as_object()
        .expect("step floors")
        .iter()
        .map(|(name, floors)| {
            let floors = floors
                .as_object()
                .unwrap_or_else(|| panic!("{BASELINE}: {name}'s step floors"))
                .iter()
                .map(|(kind, floor)| {
                    let floor = floor
                        .as_u64()
                        .unwrap_or_else(|| panic!("{BASELINE}: {name}'s {kind} floor"));
                    (kind.clone(), floor)
                })
                .collect();
            (name.clone(), floors)
        })
        .collect();
    assert_eq!(
        step_floors.keys().cloned().collect::<BTreeSet<_>>(),
        scenarios,
        "{BASELINE}: `step_floors` floors exactly the scenarios it names"
    );
    let baseline = Baseline {
        suite_version: value["suite_version"]
            .as_str()
            .expect("suite_version")
            .to_owned(),
        total: number("total"),
        answered_floor: number("answered_floor"),
        unavailable_ceiling: number("unavailable_ceiling"),
        scenarios,
        step_floors,
    };
    assert_eq!(
        baseline.scenarios.len() as u64,
        baseline.total,
        "{BASELINE}: total counts its own scenario list"
    );
    assert!(
        baseline.answered_floor <= baseline.total,
        "{BASELINE}: the answered floor exceeds the total"
    );
    baseline
}

fn admitted() -> AdmittedSuite {
    AdmittedSuite::from_json(&read(SUITE))
        .expect("the committed integrate suite admits under the pinned ESS library")
}

/// Each scenario of `suite` has at least the baseline's number of steps of every kind it floors.
fn holds_the_step_floors(baseline: &Baseline, suite: &serde_json::Value) {
    let mut changed = Vec::new();
    for (name, floors) in &baseline.step_floors {
        let Some(steps) = suite["scenarios"][name.as_str()]["steps"].as_array() else {
            changed.push(format!("{name}: no steps"));
            continue;
        };
        for (kind, floor) in floors {
            let count = steps
                .iter()
                .filter(|step| step["step"] == kind.as_str())
                .count() as u64;
            if count < *floor {
                changed.push(format!(
                    "{name}: {count} {kind} step(s), below the baseline's floor of {floor}"
                ));
            }
        }
    }
    assert!(
        changed.is_empty(),
        "the committed integrate suite changed scenario content {BASELINE} pins; a deliberate \
         change updates the baseline in the same change: {changed:#?}"
    );
}

/// The committed suite is the `ekr-integrate` component's, complete, with nothing refused; it
/// selects exactly the scenarios the committed baseline names, each with at least its steps.
#[test]
fn the_committed_suite_is_the_complete_integrate_inventory() {
    let baseline = baseline();
    let admitted = admitted();
    let suite = admitted.suite();
    assert_eq!(suite.provenance.system.to_string(), "ekr");
    assert_eq!(
        suite.provenance.component.as_ref().map(ToString::to_string),
        Some("ekr-integrate".to_owned())
    );
    assert_eq!(
        suite.provenance.suite_version.to_string(),
        baseline.suite_version
    );
    let coverage = admitted.coverage().expect("declared coverage inventory");
    assert!(coverage.is_complete(), "coverage inventory is incomplete");
    assert!(coverage.refused.is_empty(), "{:?}", coverage.refused);
    let names: BTreeSet<String> = suite.scenarios.keys().map(ToString::to_string).collect();
    assert_eq!(names, baseline.scenarios, "the committed integrate suite");
    assert_eq!(suite.len() as u64, baseline.total);
    holds_the_step_floors(&baseline, &json(SUITE));
    assert!(
        !names.iter().any(|name| name.contains("/authored/")),
        "{names:#?}"
    );
}

/// `integrate-provenance.json` records the synthesis of exactly the committed suite, for the
/// `ekr-integrate` component, from no authored scenario directory.
#[test]
fn the_provenance_records_the_integrate_synthesis() {
    let value = json(PROVENANCE);
    assert_eq!(value["format"], "ekr.conformance-provenance/1");
    assert_eq!(value["suite"], SUITE);
    assert_eq!(value["producer"]["tool"], "ess");
    assert_eq!(value["producer"]["version"], "0.52.0");
    let command: Vec<&str> = value["command"]
        .as_array()
        .expect("synthesis command")
        .iter()
        .map(|part| part.as_str().expect("command word"))
        .collect();
    let argument = |flag: &str| {
        command
            .windows(2)
            .find(|pair| pair[0] == flag)
            .map(|pair| pair[1])
    };
    assert_eq!(command[..4], ["ess", "verify", "conform", "synthesize"]);
    assert_eq!(argument("--path"), Some("systems/ekr"));
    assert_eq!(argument("--component"), Some("ekr-integrate"));
    assert_eq!(argument("--suite-format"), Some("5"));
    assert_eq!(argument("--target"), Some("ir"));
    assert_eq!(argument("--out"), Some(SUITE));
    assert_eq!(
        argument("--scenarios"),
        None,
        "no authored integrate scenarios"
    );
    assert_eq!(value["authored_scenarios"], serde_json::json!([]));
}

fn findings(run: &ExecutedRun) -> String {
    let mut text = String::new();
    for scenario in run.failures() {
        text.push_str(&format!("\n{} [{}]", scenario.scenario, scenario.status));
        for diagnostic in scenario.diagnostics() {
            text.push_str(&format!("\n    {diagnostic}"));
        }
    }
    text
}

fn record_report(provider: Provider, label: &str, report: &CountReport) {
    if let Some(directory) = std::env::var_os("EKR_INTEGRATE_REPORT_DIR") {
        let directory = PathBuf::from(directory);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(format!("{label}-{provider:?}.json")),
            report.to_canonical_json().unwrap(),
        )
        .unwrap();
    }
}

/// Runs the admitted committed suite through a fresh [`IntegrateTarget`] over `provider`: every
/// selected scenario ran and passed, and the run holds the baseline's set, total and floor.
fn passes_every_admitted_scenario(provider: Provider) {
    let baseline = baseline();
    let admitted = admitted();
    let work = tempfile::tempdir().unwrap();
    let target = signed_target(provider, work.path());
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &target);
    let report = CountReport::from_run(&run, &admitted).expect("report/2 pairs with the suite");
    record_report(provider, "full", &report);
    let counts = report.counts();
    println!(
        "{provider:?} provider: selected {} total {} passed {} failed {} error {} unsupported {} \
         skipped {}",
        admitted.suite().len(),
        counts.total,
        counts.passed,
        counts.failed,
        counts.error,
        counts.unsupported,
        counts.skipped,
    );
    let ran: BTreeSet<String> = run
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario.to_string())
        .collect();
    assert_eq!(
        ran, baseline.scenarios,
        "the run answered a different scenario set"
    );
    assert_eq!(counts.total, baseline.total, "{}", findings(&run));
    assert_eq!(counts.failed, 0, "failed scenarios:{}", findings(&run));
    assert!(
        counts.error + counts.unsupported + counts.skipped <= baseline.unavailable_ceiling,
        "unavailable scenarios:{}",
        findings(&run)
    );
    assert!(
        counts.passed >= baseline.answered_floor,
        "{}",
        findings(&run)
    );
    assert_eq!(report.execution_status(), CountStatus::Passed);
    assert_eq!(report.conformance_status(), CountStatus::Passed);
    assert!(run.is_conformant(), "{}", findings(&run));
}

#[test]
fn the_file_provider_passes_every_admitted_integrate_scenario() {
    passes_every_admitted_scenario(Provider::File);
}

#[test]
fn the_sqlite_provider_passes_every_admitted_integrate_scenario() {
    passes_every_admitted_scenario(Provider::Sqlite);
}

fn correlation() -> CorrelationId {
    CorrelationId::new("ekr-integrate").unwrap()
}

/// One scenario: `refused` forces that branch first, as the synthesized scenario does.
fn run(target: &IntegrateTarget, outcome: &str) -> SemanticCommandResult {
    let scenario = ScenarioContext::new(
        format!("ekr.integrate.ApplyExtraction/outcome/{outcome}")
            .parse()
            .unwrap(),
        correlation(),
    );
    target.begin_scenario(&scenario).unwrap();
    if outcome == "refused" {
        target
            .configure_external_outcome(ExternalOutcomeControl {
                force: serde_json::from_value(serde_json::json!({
                    "command": "ekr.integrate.ApplyExtraction",
                    "outcome": "refused",
                }))
                .unwrap(),
                correlation: correlation(),
            })
            .unwrap();
    }
    let result = target
        .execute_command(SemanticCommandRequest {
            command: "ekr.integrate.ApplyExtraction".parse().unwrap(),
            actor: Some("ekr.integrate.Applier".parse().unwrap()),
            caller: None,
            input: BTreeMap::from([(
                "extraction_document".to_owned(),
                Node::Text("extraction_document".to_owned()),
            )]),
            correlation: correlation(),
        })
        .unwrap();
    let observed = target
        .observe_events(EventObservationRequest {
            event: "ekr.integrate.ExtractionApplied".parse().unwrap(),
            correlation: correlation(),
            // A fixed instant, not a clock: the target answers at once from what it published.
            deadline: Deadline::at(ess_primitives::time::Timestamp::EPOCH),
        })
        .unwrap();
    assert_eq!(observed, result.direct_events, "{outcome}");
    target.end_scenario(&scenario).unwrap();
    result
}

#[test]
fn the_integrate_target_answers_both_branches_of_apply_extraction() {
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().unwrap();
        let target: IntegrateTarget =
            IntegrateTarget::new(provider, &fixtures(), work.path()).unwrap();
        assert!(target
            .identity()
            .unwrap()
            .to_string()
            .contains("ekr-integrate"));

        // Dana is new and CEO of Initech, also new: two nodes, then the fact with its evidence.
        let applied = run(&target, "applied");
        assert_eq!(
            applied.outcome.as_ref().map(ToString::to_string).as_deref(),
            Some("ekr.integrate.ApplyExtraction/applied"),
            "{provider:?}: {applied:?}"
        );
        assert_eq!(applied.error, None, "{provider:?}");
        let [event] = applied.direct_events.as_slice() else {
            panic!("{provider:?}: one event, {applied:?}");
        };
        let count = |field: &str| event.payload.get(field).cloned();
        let integer = |value: i64| Some(Node::Number(value.into()));
        assert_eq!(count("committed"), integer(2), "{provider:?}: {event:?}");
        assert_eq!(count("rejected"), integer(0), "{provider:?}");
        assert_eq!(count("ambiguous"), integer(0), "{provider:?}");
        assert_eq!(count("held"), integer(0), "{provider:?}");

        // Person has a subtype in the refusing seed: the reader refuses before any write.
        let refused = run(&target, "refused");
        assert_eq!(
            refused.outcome.as_ref().map(ToString::to_string).as_deref(),
            Some("ekr.integrate.ApplyExtraction/refused"),
            "{provider:?}: {refused:?}"
        );
        let error = refused.error.expect("a declared error");
        assert_eq!(error.error.to_string(), "ekr.integrate.ExtractionRefused");
        assert_eq!(
            error.fields.get("code"),
            Some(&Node::Text("reference-type-has-subtypes".to_owned())),
            "{provider:?}: {error:?}"
        );
        assert!(refused.direct_events.is_empty(), "{provider:?}");
    }
}

/// Partial delivery stays visibly nonconformant until F supplies its two apply scenarios.
#[test]
fn implemented_knowledge_commands_answer_seventeen_scenarios_on_both_providers() {
    let mut failures = Vec::new();
    for provider in [Provider::File, Provider::Sqlite] {
        let admitted = admitted();
        let work = tempfile::tempdir().unwrap();
        let target = signed_target(provider, work.path());
        let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &target);
        let report = CountReport::from_run(&run, &admitted).unwrap();
        record_report(provider, "partial", &report);
        let counts = report.counts();
        let actual = (
            counts.passed,
            counts.failed,
            counts.error,
            counts.unsupported,
            counts.skipped,
        );
        println!("{provider:?}: partial counts {actual:?}");
        if actual != (17, 0, 0, 2, 0) {
            failures.push(format!("{provider:?}: {actual:?}: {}", findings(&run)));
        }
        assert!(
            !run.is_conformant(),
            "F application is not implemented in this slice"
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Mutation controls run the same admitted suite and actual adapter. They alter only the boundary
/// being tested, so a scenario must witness the real command result or its emitted event.
struct BrokenKnowledge<'a> {
    inner: &'a IntegrateTarget,
    drop_events: bool,
}
impl ConformanceTarget for BrokenKnowledge<'_> {
    fn identity(
        &self,
    ) -> Result<ess_conformance::target::ImplementationIdentity, ess_conformance::target::TargetError>
    {
        self.inner.identity()
    }
    fn fixture_values(
        &self,
        scenario: &ScenarioContext,
        contract: &ess_conformance::fixtures::Contract,
    ) -> Result<BTreeMap<String, Node>, ess_conformance::target::TargetError> {
        self.inner.fixture_values(scenario, contract)
    }
    fn begin_scenario(
        &self,
        scenario: &ScenarioContext,
    ) -> Result<(), ess_conformance::target::TargetError> {
        self.inner.begin_scenario(scenario)
    }
    fn execute_command(
        &self,
        mut request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, ess_conformance::target::TargetError> {
        if !self.drop_events
            && matches!(
                request.command.to_string().as_str(),
                "ekr.integrate.ApproveSchemaProposal" | "ekr.integrate.RejectSchemaProposal"
            )
        {
            let Some(Node::Map(proof)) = request.input.get_mut("human_proof") else {
                panic!("the review fixture supplies a typed proof")
            };
            proof.insert(
                "signature".into(),
                Node::Text(ekr_core::bytes::encode(&[0; 64])),
            );
        }
        let mut result = self.inner.execute_command(request)?;
        if self.drop_events {
            result.direct_events.clear();
            result.response = None;
        }
        Ok(result)
    }
    fn query_view(
        &self,
        request: ess_conformance::target::SemanticViewRequest,
    ) -> Result<ess_conformance::target::SemanticViewResult, ess_conformance::target::TargetError>
    {
        self.inner.query_view(request)
    }
    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ess_conformance::target::ObservedEvent>, ess_conformance::target::TargetError>
    {
        if self.drop_events {
            Ok(vec![])
        } else {
            self.inner.observe_events(request)
        }
    }
    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), ess_conformance::target::TargetError> {
        self.inner.configure_external_outcome(request)
    }
    fn redeliver_event(
        &self,
        request: ess_conformance::target::RedeliveryRequest,
    ) -> Result<(), ess_conformance::target::TargetError> {
        self.inner.redeliver_event(request)
    }
    fn end_scenario(
        &self,
        scenario: &ScenarioContext,
    ) -> Result<(), ess_conformance::target::TargetError> {
        self.inner.end_scenario(scenario)
    }
}
fn existing_failures(provider: Provider, suite: &AdmittedSuite) -> BTreeSet<String> {
    let work = tempfile::tempdir().unwrap();
    let target = signed_target(provider, work.path());
    let run = Runner::for_suite(suite.suite()).run_admitted(suite, &target);
    let report = CountReport::from_run(&run, suite).unwrap();
    assert_eq!(report.counts().error, 0, "{}", findings(&run));
    run.scenarios
        .iter()
        .filter(|s| s.status.to_string() == "failed")
        .map(|s| s.scenario.to_string())
        .collect()
}
#[test]
fn knowledge_scenarios_detect_missing_observable_results() {
    for provider in [Provider::File, Provider::Sqlite] {
        let suite = admitted();
        let prior = existing_failures(provider, &suite);
        let work = tempfile::tempdir().unwrap();
        let target = signed_target(provider, work.path());
        let broken = BrokenKnowledge {
            inner: &target,
            drop_events: true,
        };
        let run = Runner::for_suite(suite.suite()).run_admitted(&suite, &broken);
        let report = CountReport::from_run(&run, &suite).unwrap();
        record_report(provider, "event-suppression", &report);
        assert_eq!(report.counts().error, 0, "{}", findings(&run));
        let names: BTreeSet<_> = run
            .scenarios
            .iter()
            .filter(|s| s.status.to_string() == "failed")
            .map(|s| s.scenario.to_string())
            .filter(|name| !prior.contains(name))
            .collect();
        let expected: BTreeSet<_> = baseline()
            .scenarios
            .into_iter()
            .filter(|s| {
                !s.contains("ApplySchemaProposal")
                    && !prior.contains(s)
                    && (s.ends_with("/answered") || s.ends_with("/applied"))
            })
            .collect();
        assert_eq!(names, expected, "{provider:?}: {}", findings(&run));
    }
}
#[test]
fn exact_review_scenarios_fail_when_the_signed_proof_is_corrupted() {
    for provider in [Provider::File, Provider::Sqlite] {
        let suite = admitted();
        let prior = existing_failures(provider, &suite);
        let work = tempfile::tempdir().unwrap();
        let target = signed_target(provider, work.path());
        let broken = BrokenKnowledge {
            inner: &target,
            drop_events: false,
        };
        let run = Runner::for_suite(suite.suite()).run_admitted(&suite, &broken);
        let report = CountReport::from_run(&run, &suite).unwrap();
        record_report(provider, "signature-corruption", &report);
        assert_eq!(report.counts().error, 0, "{}", findings(&run));
        let names: BTreeSet<_> = run
            .scenarios
            .iter()
            .filter(|s| s.status.to_string() == "failed")
            .map(|s| s.scenario.to_string())
            .filter(|name| !prior.contains(name))
            .collect();
        assert_eq!(
            names,
            BTreeSet::from([
                "ekr.integrate.ApproveSchemaProposal/outcome/answered".to_owned(),
                "ekr.integrate.RejectSchemaProposal/outcome/answered".to_owned()
            ]),
            "{provider:?}: {}",
            findings(&run)
        );
    }
}
