//! `story:ess-conformance-kernel`: the admitted, synthesized `ekr-kernel` suite executed through
//! the same `Runtime` and typed kernel handlers the CLI uses, on both native providers.
//!
//! Every case admits the committed `systems/ekr/conformance/suite.json` bytes, runs them with
//! `Runner::run_admitted`, and reads the verdict off the released library's `report/2`
//! (`CountReport::from_run`) paired with those exact bytes. A zero exit alone is never the
//! evidence: each run prints its actual selected/terminal counts and every scenario that did not
//! pass, and writes report/2 and run/2 under the build directory.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ekr::conformance::{KernelTarget, Provider};
use ess_conformance::report::Status;
use ess_conformance::target::{
    ConformanceTarget, EventObservationRequest, ExternalOutcomeControl, ImplementationIdentity,
    ObservedEvent, RedeliveryRequest, ScenarioContext, SemanticCommandRequest,
    SemanticCommandResult, SemanticViewRequest, SemanticViewResult, TargetError,
};
use ess_conformance::{AdmittedSuite, CountReport, CountRun, CountStatus, ExecutedRun, Runner};
use ess_primitives::node::Node;

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

fn fixtures() -> PathBuf {
    workspace_root().join("crates/ekr/tests/fixtures/conformance")
}

/// The committed count baseline.
struct Baseline {
    suite_digest: String,
    total: u64,
    answered_floor: u64,
    unavailable_ceiling: u64,
    scenarios: BTreeSet<String>,
}

fn baseline() -> Baseline {
    let value: serde_json::Value =
        serde_json::from_str(&read("systems/ekr/conformance/baseline.json")).expect("baseline");
    assert_eq!(value["format"], "ekr.conformance-baseline/1");
    assert_eq!(
        value["quarantine"],
        serde_json::json!([]),
        "P1 admits no quarantine"
    );
    let number = |key: &str| value[key].as_u64().unwrap_or_else(|| panic!("{key}"));
    Baseline {
        suite_digest: value["suite_digest"].as_str().expect("digest").to_owned(),
        total: number("total"),
        answered_floor: number("answered_floor"),
        unavailable_ceiling: number("unavailable_ceiling"),
        scenarios: value["scenarios"]
            .as_array()
            .expect("scenario names")
            .iter()
            .map(|name| name.as_str().expect("name").to_owned())
            .collect(),
    }
}

fn admitted() -> AdmittedSuite {
    AdmittedSuite::from_json(&read("systems/ekr/conformance/suite.json"))
        .expect("the committed suite admits under the pinned ESS library")
}

fn provider_name(provider: Provider) -> &'static str {
    match provider {
        Provider::File => "file",
        Provider::Sqlite => "sqlite",
    }
}

/// Where report/2 and run/2 of an actual run are written: the invoking build directory.
fn evidence_directory(provider: Provider) -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| workspace_root().join("target"), PathBuf::from);
    target.join("ekr-conformance").join(provider_name(provider))
}

/// One complete admitted run through a fresh target over `provider`.
fn execute<T: ConformanceTarget>(admitted: &AdmittedSuite, target: &T) -> ExecutedRun {
    Runner::for_suite(admitted.suite()).run_admitted(admitted, target)
}

fn kernel_target(provider: Provider, work: &Path) -> KernelTarget {
    KernelTarget::new(provider, &fixtures(), work).expect("the kernel target opens")
}

/// Every scenario that did not pass, with its diagnostics, for the assertion message.
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

fn statuses(run: &ExecutedRun) -> BTreeMap<String, Status> {
    run.scenarios
        .iter()
        .map(|scenario| (scenario.scenario.to_string(), scenario.status))
        .collect()
}

fn passes_every_admitted_scenario(provider: Provider) {
    let baseline = baseline();
    let admitted = admitted();
    let work = tempfile::TempDir::new().expect("isolated provider root");
    let target = kernel_target(provider, work.path());
    let run = execute(&admitted, &target);

    let report = CountReport::from_run(&run, &admitted).expect("report/2 pairs with the suite");
    let detailed = CountRun::from_run(&run, &admitted).expect("run/2 pairs with the suite");
    let evidence = evidence_directory(provider);
    std::fs::create_dir_all(&evidence).expect("evidence directory");
    std::fs::write(
        evidence.join("report.json"),
        report.to_canonical_json().expect("report/2"),
    )
    .expect("writing report/2");
    std::fs::write(
        evidence.join("run.json"),
        detailed.to_canonical_json().expect("run/2"),
    )
    .expect("writing run/2");

    let counts = report.counts();
    println!(
        "{} provider: selected {} total {} passed {} failed {} error {} unsupported {} skipped {} \
         execution {:?} conformance {:?} report {}",
        provider_name(provider),
        admitted.suite().len(),
        counts.total,
        counts.passed,
        counts.failed,
        counts.error,
        counts.unsupported,
        counts.skipped,
        report.execution_status(),
        report.conformance_status(),
        evidence.join("report.json").display(),
    );
    for (name, status) in statuses(&run) {
        println!("  {status:<11} {name}");
    }

    let ran: BTreeSet<String> = statuses(&run).into_keys().collect();
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
fn the_file_provider_passes_every_admitted_kernel_scenario() {
    passes_every_admitted_scenario(Provider::File);
}

#[test]
fn the_sqlite_provider_passes_every_admitted_kernel_scenario() {
    passes_every_admitted_scenario(Provider::Sqlite);
}

/// The committed suite is the complete generated inventory the baseline counts, byte for byte.
/// Freshness against the checked-in model is `task conform-check`'s synthesis-and-`cmp` step.
#[test]
fn the_committed_suite_is_the_complete_inventory_the_baseline_counts() {
    let baseline = baseline();
    let admitted = admitted();
    assert_eq!(admitted.digest(), baseline.suite_digest);
    let suite = admitted.suite();
    assert_eq!(suite.provenance.system.to_string(), "ekr");
    assert_eq!(
        suite.provenance.component.as_ref().map(ToString::to_string),
        Some("ekr-kernel".to_owned())
    );
    assert_eq!(
        suite.provenance.suite_version.to_string(),
        "ess-conformance/13"
    );
    let coverage = admitted.coverage().expect("declared coverage inventory");
    assert!(coverage.is_complete(), "coverage inventory is incomplete");
    assert!(coverage.refused.is_empty(), "{:?}", coverage.refused);
    assert_eq!(suite.len() as u64, baseline.total);
    let names: BTreeSet<String> = suite.scenarios.keys().map(ToString::to_string).collect();
    assert_eq!(names, baseline.scenarios);
    println!("{} admitted scenarios: {names:#?}", names.len());
}

/// Every scenario the fixture manifest stages documents for is one the admitted suite selects.
#[test]
fn the_fixture_manifest_names_only_admitted_scenarios() {
    let manifest: serde_json::Value =
        serde_json::from_str(&read("crates/ekr/tests/fixtures/conformance/manifest.json"))
            .expect("manifest");
    let names = baseline().scenarios;
    let staged: Vec<&String> = manifest["scenarios"]
        .as_object()
        .expect("scenarios")
        .keys()
        .collect();
    assert!(!staged.is_empty());
    for scenario in staged {
        assert!(names.contains(scenario), "{scenario} is not admitted");
    }
}

// ---- the ESS pin ---------------------------------------------------------------------------

/// The ESS release every pin site names (`story:pin-ess-0-32`): tag `0.32.0` and its commit.
const ESS_VERSION: &str = "0.32.0";
const ESS_REV: &str = "f4c1bb84298e75650674c028f9995b2324f79c06";

/// Every `rev` a line pinning the ESS repository names, read off `text`.
fn ess_revs(text: &str, marker: &str) -> Vec<String> {
    text.lines()
        .filter(|line| line.contains("github.com/beyond10x/ess"))
        .flat_map(|line| {
            line.match_indices(marker)
                .map(|(at, _)| {
                    line[at + marker.len()..]
                        .chars()
                        .take_while(char::is_ascii_hexdigit)
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Every version a text names as `ESS <x.y.z>` or `ess-<x.y.z>`.
fn ess_versions(text: &str, marker: &str) -> Vec<String> {
    text.match_indices(marker)
        .map(|(at, _)| {
            text[at + marker.len()..]
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.')
                .collect::<String>()
                .trim_end_matches('.')
                .to_owned()
        })
        .filter(|version| version.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .collect()
}

/// The conformance crates, CI's `ess` install, its cache key, the README, the lockfile and the
/// suite's provenance record all name one ESS release, `ESS_VERSION` at `ESS_REV`.
#[test]
fn every_ess_pin_site_names_one_release() {
    let manifest = ess_revs(&read("Cargo.toml"), "rev = \"");
    assert_eq!(
        manifest,
        vec![ESS_REV.to_owned(); 2],
        "Cargo.toml: ess-conformance and ess-primitives"
    );

    let lock = read("Cargo.lock");
    let locked = ess_revs(&lock, "?rev=");
    let resolved = ess_revs(&lock, "#");
    assert!(
        locked.len() >= 2,
        "Cargo.lock locks the ESS crates: {locked:?}"
    );
    assert!(
        locked.iter().chain(&resolved).all(|rev| rev == ESS_REV),
        "Cargo.lock: {locked:?} resolved {resolved:?}"
    );

    let workflow = read(".github/workflows/correctness.yml");
    assert_eq!(
        ess_revs(&workflow, "--rev "),
        vec![ESS_REV.to_owned()],
        "correctness.yml installs ess"
    );
    assert_eq!(
        ess_versions(&workflow, "-ess-"),
        vec![ESS_VERSION.to_owned()],
        "correctness.yml cache key"
    );

    let readme = ess_versions(&read("README.md"), "ESS ");
    assert!(!readme.is_empty(), "README.md names the ESS release");
    assert!(
        readme.iter().all(|version| version == ESS_VERSION),
        "README.md: {readme:?}"
    );

    let provenance: serde_json::Value =
        serde_json::from_str(&read("systems/ekr/conformance/provenance.json")).expect("provenance");
    assert_eq!(provenance["producer"]["tool"], "ess");
    assert_eq!(provenance["producer"]["version"], ESS_VERSION);
    assert_eq!(provenance["producer"]["rev"], ESS_REV);
}

/// The shell guard a Taskfile task declares as a precondition naming `ess --version`.
///
/// Only a `sh:` that go-task reads as a precondition counts: the first key of a list item sitting
/// directly under the task's own `preconditions:` key, inside the top-level `tasks:` map. A `- sh:`
/// filed under any other key (a misspelled `precondition:`, `cmds:`, `deps:`) is not a guard, and
/// task runs the commands without it (adversary pass 1 on p1-15-ess, finding 1).
fn taskfile_guard(task: &str) -> String {
    let taskfile = read("Taskfile.yml");
    let header = format!("  {task}:");
    let body: Vec<&str> = taskfile
        .lines()
        .skip_while(|line| *line != "tasks:")
        .skip(1)
        .take_while(|line| line.is_empty() || line.starts_with(' '))
        .skip_while(|line| *line != header)
        .skip(1)
        .take_while(|line| line.is_empty() || line.starts_with("   "))
        .collect();
    assert!(
        !body.is_empty(),
        "Taskfile.yml has no task {task} under tasks:"
    );
    let preconditions: Vec<&str> = body
        .iter()
        .skip_while(|line| **line != "    preconditions:")
        .skip(1)
        .take_while(|line| line.is_empty() || line.starts_with("     "))
        .copied()
        .collect();
    let guards: Vec<String> = preconditions
        .iter()
        .filter_map(|line| line.strip_prefix("      - sh: "))
        .filter(|sh| sh.contains("ess --version"))
        .map(str::to_owned)
        .collect();
    assert_eq!(
        guards.len(),
        1,
        "{task} declares one `ess --version` guard under its `preconditions:` key: {body:#?}"
    );
    guards.into_iter().next().expect("one guard")
}

/// Runs `guard` with an `ess` on PATH that answers `--version` with `reported`.
#[cfg(unix)]
fn guard_admits(guard: &str, reported: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let bin = tempfile::TempDir::new().expect("a directory for the stand-in ess");
    let ess = bin.path().join("ess");
    std::fs::write(&ess, format!("#!/bin/sh\necho '{reported}'\n")).expect("stand-in ess");
    std::fs::set_permissions(&ess, std::fs::Permissions::from_mode(0o755)).expect("executable");
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    std::process::Command::new("sh")
        .arg("-c")
        .arg(guard)
        .env("PATH", path)
        .status()
        .expect("sh runs the guard")
        .success()
}

/// `spec-check` and `conform-check` refuse an `ess` whose `--version` is not `ess ESS_VERSION`, and
/// admit one whose `--version` is. The guard reads the version string only, not the build's commit.
#[cfg(unix)]
#[test]
fn spec_and_conform_checks_refuse_an_ess_that_is_not_the_pinned_release() {
    for task in ["spec-check", "conform-check"] {
        let guard = taskfile_guard(task);
        assert!(
            guard_admits(&guard, &format!("ess {ESS_VERSION}")),
            "{task} refuses ess {ESS_VERSION}: {guard}"
        );
        for other in ["ess 0.29.0", "ess 0.32.1", "ess 0.32.0-rc.1", "ess 10.32.0"] {
            assert!(
                !guard_admits(&guard, other),
                "{task} admits `{other}`: {guard}"
            );
        }
    }
}

// ---- mutation controls ---------------------------------------------------------------------

/// One boundary defect a broken kernel would show, injected around the real target.
#[derive(Clone, Copy, Debug)]
enum Defect {
    /// A retained Commit retry answers a receipt whose time was sampled again.
    ResampledRetainedCommit,
    /// A wrong-state refusal names a missing transaction instead.
    WrongStateAsNotFound,
    /// The all-state Transactions view reports a committed transaction as still Validated.
    CommittedReadAsValidated,
    /// A snapshot reports a revision it was not asked for and a root no revision holds
    /// (adversary pass 1, finding 2).
    ForgedSnapshot,
    /// Every Transactions row returns null for the six Optional fields it declares beside
    /// `validated_against` (adversary pass 1, finding 4).
    NulledOptionalTransactionFields,
    /// Every stored object is reported as deletable Ephemeral bytes (adversary pass 2, finding A).
    EphemeralStoredObjects,
    /// A command reports none of the store events the provider log gained
    /// (adversary pass 1, finding 1).
    DroppedStoreEvents,
}

struct Defective<'a> {
    inner: &'a KernelTarget,
    defect: Defect,
}

impl ConformanceTarget for Defective<'_> {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        self.inner.identity()
    }
    fn begin_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        self.inner.begin_scenario(scenario)
    }
    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let commit = request.command.to_string() == "ekr.kernel.Commit";
        let request_is_snapshot = request.command.to_string() == "ekr.kernel.Snapshot";
        let mut result = self.inner.execute_command(request)?;
        match self.defect {
            Defect::ResampledRetainedCommit
                if commit
                    && result
                        .outcome
                        .as_ref()
                        .is_some_and(|o| o.outcome.to_string() == "retained-commit") =>
            {
                let response = result.response.as_mut().expect("a retained receipt");
                let Some(Node::Map(union)) = response.get_mut("result") else {
                    panic!("CommitCommandResult is a tagged union");
                };
                let Some(Node::Map(receipt)) = union.get_mut("value") else {
                    panic!("the committed receipt is the union content");
                };
                receipt.insert(
                    "committed_at".to_owned(),
                    Node::Text("2100-01-01T00:00:00.000Z".to_owned()),
                );
            }
            Defect::ForgedSnapshot if request_is_snapshot => {
                for event in &mut result.direct_events {
                    if event.event.to_string() == "ekr.kernel.SnapshotTaken" {
                        event
                            .payload
                            .insert("number".to_owned(), Node::Number(424_242_i64.into()));
                        event
                            .payload
                            .insert("knowledge_root".to_owned(), Node::Text("forged".to_owned()));
                    }
                }
            }
            Defect::EphemeralStoredObjects => {
                for event in &mut result.direct_events {
                    if event.event.to_string() == "ekr.store.ObjectStored" {
                        event.payload.insert(
                            "storage_class".to_owned(),
                            Node::Text("Ephemeral".to_owned()),
                        );
                    }
                }
            }
            Defect::DroppedStoreEvents => {
                result
                    .direct_events
                    .retain(|event| !event.event.to_string().starts_with("ekr.store."));
            }
            Defect::WrongStateAsNotFound => {
                if let Some(error) = result.error.as_mut() {
                    if error.error.to_string() == "ekr.kernel.TransactionStateConflict" {
                        error.error = "ekr.kernel.TransactionNotFound".parse().expect("error");
                    }
                }
            }
            _ => {}
        }
        Ok(result)
    }
    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        let transactions = request.view.to_string() == "ekr.kernel.Transactions";
        let mut result = self.inner.query_view(request)?;
        if matches!(self.defect, Defect::CommittedReadAsValidated) && transactions {
            for row in &mut result.rows {
                if row.get("state") == Some(&Node::Text("Committed".to_owned())) {
                    row.insert("state".to_owned(), Node::Text("Validated".to_owned()));
                }
            }
        }
        if matches!(self.defect, Defect::NulledOptionalTransactionFields) && transactions {
            for row in &mut result.rows {
                for field in [
                    "canonical_transaction_hash",
                    "canonical_operations_hash",
                    "validation_basis",
                    "validation_hash",
                    "validation_record_hash",
                    "terminal_record_hash",
                ] {
                    row.insert(field.to_owned(), Node::Null);
                }
            }
        }
        Ok(result)
    }
    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        self.inner.observe_events(request)
    }
    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        self.inner.configure_external_outcome(request)
    }
    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        self.inner.redeliver_event(request)
    }
    fn end_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        self.inner.end_scenario(scenario)
    }
}

/// Every admitted scenario that requires a store event (`ekr.store.*`) to be published, read off
/// the admitted suite itself rather than listed: after the kernel.yaml decision every durable kernel
/// outcome emits `PublicationPrepared` and `ObjectStored`, and a list written by hand would miss
/// the next outcome that gains one.
fn store_event_scenarios(admitted: &AdmittedSuite) -> Vec<String> {
    let suite: serde_json::Value =
        serde_json::from_str(&read("systems/ekr/conformance/suite.json")).expect("suite JSON");
    let names: Vec<String> = suite["scenarios"]
        .as_object()
        .expect("scenarios")
        .iter()
        .filter(|(_, scenario)| {
            scenario["steps"].as_array().is_some_and(|steps| {
                steps.iter().any(|step| {
                    step["step"] == "expect_event"
                        && step["event"]
                            .as_str()
                            .is_some_and(|event| event.starts_with("ekr.store."))
                })
            })
        })
        .map(|(name, _)| name.clone())
        .collect();
    assert_eq!(
        suite["scenarios"].as_object().map(|s| s.len()),
        Some(admitted.suite().len()),
        "the suite read here is the admitted one"
    );
    assert!(
        !names.is_empty(),
        "no admitted scenario expects a store event, so DroppedStoreEvents shows nothing"
    );
    names
}

/// Each defect fails exactly its named scenarios and leaves every other passing scenario passing.
#[test]
fn each_injected_kernel_defect_fails_exactly_its_named_scenarios() {
    let admitted = admitted();
    let work = tempfile::TempDir::new().expect("isolated provider root");
    let store_events = store_event_scenarios(&admitted);
    let matrix = [
        (
            Defect::ResampledRetainedCommit,
            vec!["ekr.kernel.Commit/outcome/retained-commit"],
        ),
        (
            Defect::WrongStateAsNotFound,
            vec![
                "ekr.kernel.Commit/outcome/wrong-state",
                "ekr.kernel.GraphTransaction/state/Committed/refuses/ekr.kernel.Validate",
                "ekr.kernel.GraphTransaction/state/Proposed/refuses/ekr.kernel.Commit",
                "ekr.kernel.GraphTransaction/state/Rejected/refuses/ekr.kernel.Commit",
                "ekr.kernel.GraphTransaction/state/Rejected/refuses/ekr.kernel.Validate",
                "ekr.kernel.GraphTransaction/state/Stale/refuses/ekr.kernel.Commit",
                "ekr.kernel.GraphTransaction/state/Stale/refuses/ekr.kernel.Validate",
                "ekr.kernel.GraphTransaction/state/Validated/refuses/ekr.kernel.Validate",
            ],
        ),
        (
            Defect::CommittedReadAsValidated,
            vec![
                "ekr.kernel.Commit/outcome/committed",
                "ekr.kernel.Commit/outcome/retained-commit",
                "ekr.kernel.GraphTransaction/invariant/after/ekr.kernel.Commit/committed",
                "ekr.kernel.GraphTransaction/transition/commit/by/ekr.kernel.Commit/committed",
                "ekr.kernel/authored/a-committed-transaction-row-carries-every-record-it-names",
            ],
        ),
        (
            Defect::ForgedSnapshot,
            vec![
                "ekr.kernel/authored/snapshot-at-a-past-revision-reports-that-revision",
                "ekr.kernel/authored/snapshot-without-a-revision-reports-the-newest",
            ],
        ),
        (
            Defect::NulledOptionalTransactionFields,
            vec!["ekr.kernel/authored/a-committed-transaction-row-carries-every-record-it-names"],
        ),
        (
            Defect::EphemeralStoredObjects,
            vec!["ekr.kernel/authored/a-proposal-is-stored-as-its-canonical-record"],
        ),
        (
            Defect::DroppedStoreEvents,
            store_events.iter().map(String::as_str).collect(),
        ),
    ];
    // One clean run and one per defect, each over its own provider roots, on their own threads.
    let (clean, faulted) = std::thread::scope(|scope| {
        let (admitted, work) = (&admitted, work.path());
        let clean = scope.spawn(move || {
            statuses(&execute(
                admitted,
                &kernel_target(Provider::File, &work.join("clean")),
            ))
        });
        let faulted: Vec<_> = matrix
            .iter()
            .map(|(defect, _)| {
                let defect = *defect;
                scope.spawn(move || {
                    let inner = kernel_target(Provider::File, &work.join(format!("{defect:?}")));
                    statuses(&execute(
                        admitted,
                        &Defective {
                            inner: &inner,
                            defect,
                        },
                    ))
                })
            })
            .collect();
        (
            clean.join().expect("the clean run"),
            faulted
                .into_iter()
                .map(|run| run.join().expect("a faulted run"))
                .collect::<Vec<_>>(),
        )
    });
    for ((defect, named), observed) in matrix.into_iter().zip(faulted) {
        let named: BTreeSet<String> = named.into_iter().map(str::to_owned).collect();
        for scenario in &named {
            assert_eq!(
                clean.get(scenario),
                Some(&Status::Passed),
                "{scenario} must pass unfaulted before it can show {defect:?}"
            );
            assert_eq!(
                observed.get(scenario),
                Some(&Status::Failed),
                "{defect:?} was not caught by {scenario}"
            );
        }
        for (scenario, status) in &clean {
            if *status == Status::Passed && !named.contains(scenario) {
                assert_eq!(
                    observed.get(scenario),
                    Some(&Status::Passed),
                    "{defect:?} also changed {scenario}"
                );
            }
        }
    }
}
