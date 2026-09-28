//! One complete admitted run of the committed `ekr-views` suite, and the verdict it must reach.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ess_conformance::report::Status;
use ess_conformance::{AdmittedSuite, CountReport, CountRun, CountStatus, ExecutedRun, Runner};

use super::fixtures::Provider;
use super::target::{Answered, ViewsTarget};

pub const SUITE: &str = "systems/ekr/conformance/views-suite.json";

/// The committed floor the suite is held against. It is written by hand and never generated, so
/// a scenario lost from the fixtures or the model cannot lower it by regenerating the suite.
pub const BASELINE: &str = "systems/ekr/conformance/views-baseline.json";

/// The record of how the committed suite was synthesized, and from which authored files.
pub const PROVENANCE: &str = "systems/ekr/conformance/views-provenance.json";

/// The directory of authored scenario files the synthesis reads.
pub const AUTHORED: &str = "crates/ekr-views/tests/fixtures/conformance/scenarios";

/// The name prefix the synthesis gives a scenario read from an authored file.
pub const AUTHORED_PREFIX: &str = "ekr.views/authored/";

/// The committed views baseline: every scenario name it requires, and the counts it floors.
pub struct Baseline {
    pub suite_version: String,
    pub total: u64,
    pub answered_floor: u64,
    pub unavailable_ceiling: u64,
    pub scenarios: BTreeSet<String>,
}

pub fn baseline() -> Baseline {
    let value: serde_json::Value = serde_json::from_str(&super::read(BASELINE)).expect("baseline");
    assert_eq!(value["format"], "ekr.conformance-baseline/1");
    assert_eq!(value["suite"], SUITE);
    assert_eq!(value["component"], "ekr-views");
    assert_eq!(
        value["quarantine"],
        serde_json::json!([]),
        "the views baseline admits no quarantine"
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
    let baseline = Baseline {
        suite_version: value["suite_version"]
            .as_str()
            .expect("suite_version")
            .to_owned(),
        total: number("total"),
        answered_floor: number("answered_floor"),
        unavailable_ceiling: number("unavailable_ceiling"),
        scenarios,
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

/// Holds `names` — what `subject` selected or answered — to exactly the baseline's scenario set:
/// a baseline scenario that is absent is lost coverage, named; a scenario the baseline does not
/// list is coverage added without updating the baseline in the same change, named.
pub fn holds_the_baseline_scenarios(baseline: &Baseline, names: &BTreeSet<String>, subject: &str) {
    let missing: Vec<&String> = baseline.scenarios.difference(names).collect();
    assert!(
        missing.is_empty(),
        "{subject} lacks {} scenario(s) the committed baseline {BASELINE} requires: {missing:#?}",
        missing.len()
    );
    let unlisted: Vec<&String> = names.difference(&baseline.scenarios).collect();
    assert!(
        unlisted.is_empty(),
        "{subject} has {} scenario(s) {BASELINE} does not name; a new scenario updates the \
         baseline in the same change: {unlisted:#?}",
        unlisted.len()
    );
}

/// The authored scenario files `views-provenance.json` records the committed suite was
/// synthesized from, as workspace-relative paths.
pub fn provenance_authored() -> BTreeSet<String> {
    let value: serde_json::Value =
        serde_json::from_str(&super::read(PROVENANCE)).expect("views provenance");
    assert_eq!(value["format"], "ekr.conformance-provenance/1");
    assert_eq!(value["suite"], SUITE);
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
            .unwrap_or_else(|| panic!("{PROVENANCE}: the command carries no {flag}"))
    };
    assert_eq!(argument("--component"), "ekr-views");
    assert_eq!(argument("--scenarios"), AUTHORED);
    assert_eq!(argument("--out"), SUITE);
    let listed: Vec<String> = value["authored_scenarios"]
        .as_array()
        .expect("authored scenario paths")
        .iter()
        .map(|path| path.as_str().expect("path").to_owned())
        .collect();
    let authored: BTreeSet<String> = listed.iter().cloned().collect();
    assert_eq!(
        listed.len(),
        authored.len(),
        "{PROVENANCE} names a file twice"
    );
    authored
}

/// Every file in the authored scenario directory, as workspace-relative paths.
pub fn authored_files() -> BTreeSet<String> {
    std::fs::read_dir(super::workspace_root().join(AUTHORED))
        .expect("authored scenarios")
        .map(|entry| {
            let name = entry.expect("directory entry").file_name();
            format!("{AUTHORED}/{}", name.to_str().expect("UTF-8 file name"))
        })
        .collect()
}

/// The scenario name the synthesis gives an authored file: its stem under `AUTHORED_PREFIX`.
pub fn authored_scenario_name(path: &str) -> String {
    let stem = std::path::Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_else(|| panic!("{path} has no file stem"));
    format!("{AUTHORED_PREFIX}{stem}")
}

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
/// selected scenario ran and passed; none failed, errored, was unsupported or was skipped; and
/// the suite and the run each hold the committed baseline's scenario set, total and floor.
/// Returns every projection the target answered during the run.
pub fn passes_every_admitted_scenario(provider: Provider) -> Vec<Answered> {
    let baseline = baseline();
    let admitted = admitted();
    let expected = suite_scenarios();
    holds_the_baseline_scenarios(&baseline, &expected, "the committed views suite");
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
    holds_the_baseline_scenarios(&baseline, &ran, &format!("the {} run", provider.name()));
    assert!(
        counts.total >= baseline.total,
        "the run totals {} scenarios, below the baseline's {}",
        counts.total,
        baseline.total
    );
    assert!(
        counts.passed >= baseline.answered_floor,
        "the run passed {} scenarios, below the baseline's floor of {}:{}",
        counts.passed,
        baseline.answered_floor,
        findings(&run)
    );
    assert!(
        counts.error + counts.unsupported + counts.skipped <= baseline.unavailable_ceiling,
        "unavailable scenarios above the baseline's ceiling:{}",
        findings(&run)
    );
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
