//! One complete admitted run of the committed `ekr-views` suite, and the verdict it must reach.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ess_conformance::report::Status;
use ess_conformance::{AdmittedSuite, CountReport, CountRun, CountStatus, ExecutedRun, Runner};

use super::fixtures::Provider;
use super::target::{Answered, ViewsTarget};

pub const SUITE: &str = "systems/ekr/conformance/views-suite.json";

pub fn admitted() -> AdmittedSuite {
    AdmittedSuite::from_json(&super::read(SUITE))
        .expect("the committed views suite admits under the pinned ESS library")
}

/// Every scenario name the committed suite selects, read off its JSON.
pub fn suite_scenarios() -> BTreeSet<String> {
    let suite: serde_json::Value = serde_json::from_str(&super::read(SUITE)).expect("suite");
    suite["scenarios"]
        .as_object()
        .expect("scenarios")
        .keys()
        .cloned()
        .collect()
}

fn evidence_directory(provider: Provider) -> PathBuf {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map_or_else(|| super::workspace_root().join("target"), PathBuf::from);
    target.join("ekr-views-conformance").join(provider.name())
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

fn statuses(run: &ExecutedRun) -> BTreeMap<String, Status> {
    run.scenarios
        .iter()
        .map(|scenario| (scenario.scenario.to_string(), scenario.status))
        .collect()
}

/// Runs the admitted suite through a fresh target over `provider` and holds the verdict: every
/// selected scenario ran and passed; none failed, errored, was unsupported or was skipped.
/// Returns every projection the target answered during the run.
pub fn passes_every_admitted_scenario(provider: Provider) -> Vec<Answered> {
    let admitted = admitted();
    let expected = suite_scenarios();
    let work = tempfile::TempDir::new().expect("isolated provider root");
    let target = ViewsTarget::new(provider, work.path().to_path_buf());
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &target);

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
        provider.name(),
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
    assert_eq!(ran, expected, "the run answered a different scenario set");
    assert_eq!(counts.total, expected.len() as u64, "{}", findings(&run));
    assert_eq!(counts.failed, 0, "failed scenarios:{}", findings(&run));
    assert_eq!(counts.error, 0, "errored scenarios:{}", findings(&run));
    assert_eq!(counts.unsupported, 0, "unsupported:{}", findings(&run));
    assert_eq!(counts.skipped, 0, "skipped:{}", findings(&run));
    assert_eq!(counts.passed, expected.len() as u64, "{}", findings(&run));
    assert_eq!(report.execution_status(), CountStatus::Passed);
    assert_eq!(report.conformance_status(), CountStatus::Passed);
    assert!(run.is_conformant(), "{}", findings(&run));
    target.answered()
}
