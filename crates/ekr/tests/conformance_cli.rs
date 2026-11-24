//! Review finding 40 of EKR 0.0.42: the `ekr` component's conformance suite. `systems/ekr`
//! declares the stage commands and `CompareStores` on the `ekr` component (`ekr.cli`); the suite
//! ESS synthesizes for it (`ess conform synthesize --component ekr`) is committed as
//! `systems/ekr/conformance/cli-suite.json`. [`CliTarget`] runs it on both native providers.
//!
//! The suite selects two kinds of scenario, counted apart: command scenarios (`ekr.cli.*`, one per
//! declared outcome of a command) and entity scenarios (`ekr.store.Stage/*`, the stage's
//! lifecycle: each transition, each state that refuses a command). The hand-committed
//! `systems/ekr/conformance/cli-baseline.json` pins, per kind, how many the suite selects, and per
//! provider how many of each kind executed and passed and which scenarios the target answers
//! unsupported, with the reason in the target's documentation. A scenario lost from the suite, one
//! more answered unsupported, or one fewer executed is named rather than absorbed.
//! `cli-provenance.json` records how the suite was synthesized; freshness against the
//! specification is `task conform-fresh`'s synthesis-and-`cmp` step.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use ekr::conformance::{CliTarget, Provider};
use ess_conformance::report::Status;
use ess_conformance::ScenarioResult;
use ess_conformance::{AdmittedSuite, ExecutedRun, Runner};

const SUITE: &str = "systems/ekr/conformance/cli-suite.json";
const BASELINE: &str = "systems/ekr/conformance/cli-baseline.json";
const PROVENANCE: &str = "systems/ekr/conformance/cli-provenance.json";

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

fn admitted() -> AdmittedSuite {
    AdmittedSuite::from_json(&read(SUITE))
        .expect("the committed ekr suite admits under the pinned ESS library")
}

/// The kind of a scenario: `command` for one outcome of an `ekr.cli` command, `entity` for one of
/// the stage's lifecycle.
fn kind(scenario: &str) -> &'static str {
    if scenario.starts_with("ekr.cli.") {
        "command"
    } else if scenario.starts_with("ekr.store.Stage/") {
        "entity"
    } else {
        panic!("{scenario} is neither a command nor an entity scenario of the ekr suite")
    }
}

fn names(value: &serde_json::Value, what: &str) -> BTreeSet<String> {
    let listed: Vec<String> = value
        .as_array()
        .unwrap_or_else(|| panic!("{BASELINE}: {what}"))
        .iter()
        .map(|name| name.as_str().expect("a scenario name").to_owned())
        .collect();
    let set: BTreeSet<String> = listed.iter().cloned().collect();
    assert_eq!(
        listed.len(),
        set.len(),
        "{BASELINE}: {what} names a scenario twice"
    );
    set
}

fn counts(value: &serde_json::Value, what: &str) -> BTreeMap<String, u64> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("{BASELINE}: {what}"))
        .iter()
        .map(|(kind, count)| {
            let count = count
                .as_u64()
                .unwrap_or_else(|| panic!("{BASELINE}: {what}.{kind}"));
            (kind.clone(), count)
        })
        .collect()
}

/// What one provider ran: per kind, how many passed, and which scenarios were unsupported.
struct Answered {
    executed: BTreeMap<String, u64>,
    unsupported: BTreeSet<String>,
}

struct Baseline {
    suite_version: String,
    scenarios: BTreeSet<String>,
    selected: BTreeMap<String, u64>,
    /// Scenarios that fail by a known disagreement of the specification with the documented
    /// behaviour, each with exactly the checks it fails.
    quarantine: BTreeMap<String, BTreeSet<String>>,
    providers: BTreeMap<String, Answered>,
    step_floors: BTreeMap<String, BTreeMap<String, u64>>,
}

fn baseline() -> Baseline {
    let value = json(BASELINE);
    assert_eq!(value["format"], "ekr.conformance-baseline/1");
    assert_eq!(value["suite"], SUITE);
    assert_eq!(value["component"], "ekr");
    let scenarios = names(&value["scenarios"], "scenarios");
    let quarantine: BTreeMap<String, BTreeSet<String>> = value["quarantine"]
        .as_object()
        .expect("quarantine")
        .iter()
        .map(|(scenario, entry)| {
            assert!(
                scenarios.contains(scenario),
                "{BASELINE}: quarantines {scenario}, which the suite does not select"
            );
            assert!(
                entry["reason"]
                    .as_str()
                    .is_some_and(|reason| !reason.is_empty()),
                "{BASELINE}: {scenario} is quarantined without a reason"
            );
            let failed = names(
                &entry["failed_checks"],
                &format!("{scenario}.failed_checks"),
            );
            assert!(
                !failed.is_empty(),
                "{BASELINE}: {scenario} pins no failed check"
            );
            (scenario.clone(), failed)
        })
        .collect();
    let selected = counts(&value["selected"], "selected");
    let mut by_kind = BTreeMap::<String, u64>::new();
    for scenario in &scenarios {
        *by_kind.entry(kind(scenario).to_owned()).or_default() += 1;
    }
    assert_eq!(
        by_kind, selected,
        "{BASELINE}: `selected` counts its own scenario list by kind"
    );
    assert_eq!(
        value["total"].as_u64(),
        Some(scenarios.len() as u64),
        "{BASELINE}: total counts its own scenario list"
    );
    let providers: BTreeMap<String, Answered> = value["providers"]
        .as_object()
        .expect("providers")
        .iter()
        .map(|(provider, answered)| {
            let unsupported = answered["unsupported"]
                .as_object()
                .unwrap_or_else(|| panic!("{BASELINE}: {provider}.unsupported"))
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>();
            assert!(
                unsupported.is_subset(&scenarios),
                "{BASELINE}: {provider} names an unsupported scenario the suite does not select"
            );
            let executed = counts(&answered["executed"], &format!("{provider}.executed"));
            let mut sum = executed.clone();
            for scenario in unsupported.iter().chain(
                quarantine
                    .keys()
                    .filter(|scenario| !unsupported.contains(*scenario)),
            ) {
                *sum.entry(kind(scenario).to_owned()).or_default() += 1;
            }
            sum.retain(|_, count| *count > 0);
            assert_eq!(
                sum, selected,
                "{BASELINE}: {provider}'s executed, unsupported and quarantined scenarios are \
                 every selected one"
            );
            (
                provider.clone(),
                Answered {
                    executed,
                    unsupported,
                },
            )
        })
        .collect();
    assert_eq!(
        providers.keys().map(String::as_str).collect::<Vec<_>>(),
        ["file", "sqlite"]
    );
    let step_floors: BTreeMap<String, BTreeMap<String, u64>> = value["step_floors"]
        .as_object()
        .expect("step floors")
        .iter()
        .map(|(name, floors)| (name.clone(), counts(floors, &format!("step_floors.{name}"))))
        .collect();
    assert_eq!(
        step_floors.keys().cloned().collect::<BTreeSet<_>>(),
        scenarios,
        "{BASELINE}: `step_floors` floors exactly the scenarios it names"
    );
    Baseline {
        suite_version: value["suite_version"]
            .as_str()
            .expect("suite_version")
            .to_owned(),
        scenarios,
        selected,
        quarantine,
        providers,
        step_floors,
    }
}

/// The committed suite is the `ekr` component's, complete, with nothing refused; it selects
/// exactly the scenarios the committed baseline names, of each kind as many as it pins, each with
/// at least its steps.
#[test]
fn the_committed_suite_is_the_complete_ekr_inventory() {
    let baseline = baseline();
    let admitted = admitted();
    let suite = admitted.suite();
    assert_eq!(suite.provenance.system.to_string(), "ekr");
    assert_eq!(
        suite.provenance.component.as_ref().map(ToString::to_string),
        Some("ekr".to_owned())
    );
    assert_eq!(
        suite.provenance.suite_version.to_string(),
        baseline.suite_version
    );
    let coverage = admitted.coverage().expect("declared coverage inventory");
    assert!(coverage.is_complete(), "coverage inventory is incomplete");
    assert!(coverage.refused.is_empty(), "{:?}", coverage.refused);
    let names: BTreeSet<String> = suite.scenarios.keys().map(ToString::to_string).collect();
    assert_eq!(names, baseline.scenarios, "the committed ekr suite");
    let mut selected = BTreeMap::<String, u64>::new();
    for name in &names {
        *selected.entry(kind(name).to_owned()).or_default() += 1;
    }
    assert_eq!(selected, baseline.selected, "selected scenarios by kind");

    let committed = json(SUITE);
    let mut changed = Vec::new();
    for (name, floors) in &baseline.step_floors {
        let Some(steps) = committed["scenarios"][name.as_str()]["steps"].as_array() else {
            changed.push(format!("{name}: no steps"));
            continue;
        };
        for (step, floor) in floors {
            let count = steps.iter().filter(|s| s["step"] == step.as_str()).count() as u64;
            if count < *floor {
                changed.push(format!("{name}: {count} {step} step(s), floor {floor}"));
            }
        }
    }
    assert!(
        changed.is_empty(),
        "the committed ekr suite changed scenario content {BASELINE} pins: {changed:#?}"
    );
}

/// `cli-provenance.json` records the synthesis of exactly the committed suite, for the `ekr`
/// component, from no authored scenario directory.
#[test]
fn the_provenance_records_the_ekr_synthesis() {
    let value = json(PROVENANCE);
    assert_eq!(value["format"], "ekr.conformance-provenance/1");
    assert_eq!(value["suite"], SUITE);
    assert_eq!(value["producer"]["tool"], "ess");
    assert_eq!(value["producer"]["version"], "0.36.0");
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
    assert_eq!(command[..3], ["ess", "conform", "synthesize"]);
    assert_eq!(argument("--path"), Some("systems/ekr"));
    assert_eq!(argument("--component"), Some("ekr"));
    assert_eq!(argument("--suite-format"), Some("5"));
    assert_eq!(argument("--target"), Some("ir"));
    assert_eq!(argument("--out"), Some(SUITE));
    assert_eq!(argument("--scenarios"), None, "no authored ekr scenarios");
    assert_eq!(value["authored_scenarios"], serde_json::json!([]));
}

fn findings(run: &ExecutedRun, shown: impl Fn(&ScenarioResult) -> bool) -> String {
    let mut text = String::new();
    for scenario in run.scenarios.iter().filter(|s| shown(s)) {
        text.push_str(&format!("\n{} [{}]", scenario.scenario, scenario.status));
        for diagnostic in scenario.diagnostics() {
            text.push_str(&format!("\n    {diagnostic}"));
        }
    }
    text
}

/// Runs the admitted committed suite through a fresh [`CliTarget`] over `provider`: no scenario
/// fails or errors; the scenarios answered unsupported are exactly the baseline's for the
/// provider; and per kind, exactly the baseline's number executed and passed.
fn holds_the_baseline(provider: Provider, name: &str) {
    let baseline = baseline();
    let expected = &baseline.providers[name];
    let admitted = admitted();
    let work = tempfile::tempdir().unwrap();
    let target = CliTarget::new(provider, &fixtures(), work.path()).unwrap();
    let run = Runner::for_suite(admitted.suite()).run_admitted(&admitted, &target);

    let ran: BTreeSet<String> = run
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario.to_string())
        .collect();
    assert_eq!(
        ran, baseline.scenarios,
        "the run answered another scenario set"
    );
    let mut executed = BTreeMap::<String, u64>::new();
    let mut unsupported = BTreeSet::new();
    let mut quarantined = BTreeSet::new();
    for scenario in &run.scenarios {
        let id = scenario.scenario.to_string();
        if scenario.status != Status::Unsupported {
            if let Some(pinned) = baseline.quarantine.get(&id) {
                // A quarantined scenario fails exactly the checks the baseline pins, no more and
                // no fewer: a new finding in it, or one of its findings gone, is named.
                let failed: BTreeSet<String> = scenario
                    .checks
                    .iter()
                    .filter(|check| check.status != Status::Passed)
                    .map(|check| format!("{} {}", check.code, check.about))
                    .collect();
                assert_eq!(
                    &failed, pinned,
                    "the checks the quarantined {id} fails on {name}"
                );
                quarantined.insert(id);
                continue;
            }
        }
        match scenario.status {
            Status::Passed => *executed.entry(kind(&id).to_owned()).or_default() += 1,
            Status::Unsupported => {
                unsupported.insert(id);
            }
            Status::Failed | Status::Error => {}
        }
    }
    println!(
        "{name} provider: selected {:?}, executed and passed {executed:?}, unsupported {}, \
         quarantined {}",
        baseline.selected,
        unsupported.len(),
        quarantined.len()
    );
    assert_eq!(
        findings(&run, |scenario| {
            matches!(scenario.status, Status::Failed | Status::Error)
                && !quarantined.contains(&scenario.scenario.to_string())
        }),
        "",
        "failed or errored scenarios"
    );
    assert_eq!(
        unsupported,
        expected.unsupported,
        "the scenarios answered unsupported on {name}:{}",
        findings(&run, |scenario| scenario.status == Status::Unsupported)
    );
    let mut pinned = expected.executed.clone();
    pinned.retain(|_, count| *count > 0);
    assert_eq!(
        executed, pinned,
        "executed and passed scenarios by kind on {name}"
    );
}

#[test]
fn the_file_provider_holds_the_ekr_baseline() {
    holds_the_baseline(Provider::File, "file");
}

#[test]
fn the_sqlite_provider_holds_the_ekr_baseline() {
    holds_the_baseline(Provider::Sqlite, "sqlite");
}
