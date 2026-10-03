//! Native implementation coverage of retention and upgrades, with inert-target checks.
#[path = "support/knowledge_target.rs"]
mod knowledge_target;
#[path = "support/upgrade_target.rs"]
mod upgrade_target;
use ekr::conformance::Provider;
use ess_conformance::coverage::AdmittedInput;
use ess_conformance::runner::{AdvancingClock, Ids, RunnerConfig};
use ess_conformance::{AdmittedSuite, CountReport, Runner};
use knowledge_target::KnowledgeTarget;
use std::path::PathBuf;

const SCENARIOS: [&str; 3] = [
    "ekr.observe/authored/observation-retry-is-idempotent",
    "ekr.integrate/authored/rejected-interpretation-retains-its-sources",
    "ekr.integrate/authored/unmapped-knowledge-survives-reopen",
];
const UPGRADE_SCENARIOS: [&str; 3] = [
    "ekr.kernel/authored/overlapping-single-value-claims-become-disputed",
    "ekr.kernel/authored/equal-disjoint-and-many-claims-do-not-conflict",
    "ekr.kernel/authored/upgrade-preserves-historical-rules-and-hashes",
];
fn fixtures() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("tests/fixtures/conformance")
}
fn selected() -> AdmittedInput {
    select("knowledge-suite.json", SCENARIOS)
}
fn select<const N: usize>(file: &str, scenarios: [&str; N]) -> AdmittedInput {
    let raw = std::fs::read_to_string(fixtures().join(file)).unwrap();
    let full = AdmittedInput::from_suite(AdmittedSuite::from_json(&raw).unwrap()).unwrap();
    let mut ids = scenarios.map(|id| id.parse().unwrap());
    ids.sort();
    full.select(&ids).unwrap()
}
fn run(discard: bool) {
    let input = selected();
    let selected = input.selected();
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().unwrap();
        let target = KnowledgeTarget::new(
            provider,
            &fixtures().join("host.json"),
            work.path(),
            discard,
        );
        // Wall time dates the evidence only; assertions never depend on elapsed host time.
        let started = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap();
        let run = Runner::new(
            RunnerConfig::default(),
            AdvancingClock::new(started, 1),
            Ids::for_suite(selected.suite()),
        )
        .run_admitted(selected, &target);
        let report = CountReport::from_run(&run, selected).unwrap();
        let json = report.to_canonical_json().unwrap();
        eprintln!("{provider:?} discard={discard}: {json}");
        if let Some(directory) = std::env::var_os("EKR_KNOWLEDGE_REPORT_DIR") {
            let directory = PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let name = format!(
                "{}-{}",
                match provider {
                    Provider::File => "file",
                    Provider::Sqlite => "sqlite",
                },
                if discard { "discard" } else { "retained" }
            );
            std::fs::write(directory.join(format!("{name}.report.json")), &json).unwrap();
            std::fs::write(
                directory.join(format!("{name}.suite-input.json")),
                input.document().to_canonical_json().unwrap(),
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("{name}.run.json")),
                serde_json::to_vec_pretty(&run.report()).unwrap(),
            )
            .unwrap();
        }
        let counts = report.counts();
        assert_eq!(counts.total, 3);
        assert_eq!(
            counts.error + counts.unsupported + counts.skipped,
            0,
            "{json}"
        );
        if discard {
            assert_eq!(
                counts.failed, 3,
                "every named scenario must detect lost persistence: {json}"
            );
            assert_eq!(counts.passed, 0);
        } else {
            assert_eq!(counts.failed, 0, "{:#?}", run.report());
            assert_eq!(counts.passed, 3);
        }
    }
}
#[test]
fn supplied_knowledge_conforms_after_reopen_and_full_replay() {
    run(false);
}
#[test]
fn every_retention_scenario_detects_discarded_state() {
    run(true);
}

fn run_upgrade(no_op: bool) {
    let input = select("upgrade-suite.json", UPGRADE_SCENARIOS);
    let selected = input.selected();
    let mut failures = Vec::new();
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().unwrap();
        let target = upgrade_target::UpgradeTarget::new(provider, work.path(), no_op);
        let started = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap();
        let run = Runner::new(
            RunnerConfig::default(),
            AdvancingClock::new(started, 1),
            Ids::for_suite(selected.suite()),
        )
        .run_admitted(selected, &target);
        let report = CountReport::from_run(&run, selected).unwrap();
        let json = report.to_canonical_json().unwrap();
        eprintln!("{provider:?} upgrade no-op={no_op}: {json}");
        if let Some(directory) = std::env::var_os("EKR_KNOWLEDGE_REPORT_DIR") {
            let directory = PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let name = format!(
                "upgrade-{}-{}",
                match provider {
                    Provider::File => "file",
                    Provider::Sqlite => "sqlite",
                },
                if no_op { "no-op" } else { "persisted" }
            );
            std::fs::write(directory.join(format!("{name}.report.json")), &json).unwrap();
            std::fs::write(
                directory.join(format!("{name}.suite-input.json")),
                input.document().to_canonical_json().unwrap(),
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("{name}.run.json")),
                serde_json::to_vec_pretty(&run.report()).unwrap(),
            )
            .unwrap();
        }
        let counts = report.counts();
        if counts.total != 3
            || counts.error + counts.unsupported + counts.skipped != 0
            || counts.passed != if no_op { 0 } else { 3 }
            || counts.failed != if no_op { 3 } else { 0 }
        {
            failures.push(format!("{provider:?}: {:#?}", run.report()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn disputes_and_upgrades_conform_after_reopen_and_full_replay() {
    run_upgrade(false);
}
#[test]
fn every_upgrade_scenario_detects_an_inert_target() {
    run_upgrade(true);
}
#[test]
fn event_projection_preserves_large_integer_values() {
    for value in [i64::MIN, 9_007_199_254_740_993, i64::MAX] {
        let projected = knowledge_target::node(serde_json::json!(value)).unwrap();
        assert_eq!(
            knowledge_target::json_input(&projected).unwrap(),
            serde_json::json!(value)
        );
    }
}

#[test]
fn retention_scenarios_keep_their_positive_read_assertions() {
    let suite: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("knowledge-suite.json")).unwrap(),
    )
    .unwrap();
    for (id, events, required) in [
        (
            SCENARIOS[0],
            4,
            vec![
                ("ekr.observe.ObservationsListed", "observations"),
                ("ekr.observe.ObservationShown", "retained"),
            ],
        ),
        (
            SCENARIOS[1],
            3,
            vec![("ekr.observe.ObservationShown", "retained")],
        ),
        (
            SCENARIOS[2],
            4,
            vec![
                ("ekr.integrate.ListInterpretationsResult", "interpretations"),
                ("ekr.integrate.ShowInterpretationResult", "document"),
            ],
        ),
    ] {
        let steps = suite["scenarios"][id]["steps"].as_array().unwrap();
        for (kind, floor) in [
            ("execute_command", 4),
            ("expect_outcome", 4),
            ("expect_event", events),
        ] {
            assert!(
                steps.iter().filter(|step| step["step"] == kind).count() >= floor,
                "{id} lost {kind} assertions"
            );
        }
        for (event, field) in required {
            assert!(
                steps.iter().any(|step| step["step"] == "expect_event"
                    && step["event"] == event
                    && step["payload"].get(field).is_some_and(|value| value
                        .as_object()
                        .is_some_and(|value| !value.is_empty())
                        || value.as_array().is_some_and(|value| !value.is_empty()))),
                "{id} lost positive read {event}.{field}"
            );
        }
    }
}

const ANSWER_SCENARIOS: [&str; 2] = [
    "ekr.kernel/authored/human-resolution-preserves-evidence-and-history",
    "ekr.kernel/authored/changed-answer-basis-requires-review",
];
fn run_answers(no_op: bool) {
    let input = select("answer-suite.json", ANSWER_SCENARIOS);
    let selected = input.selected();
    let mut failures = Vec::new();
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().unwrap();
        let target = upgrade_target::UpgradeTarget::answers(provider, work.path(), no_op);
        let started = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap();
        let run = Runner::new(
            RunnerConfig::default(),
            AdvancingClock::new(started, 1),
            Ids::for_suite(selected.suite()),
        )
        .run_admitted(selected, &target);
        let report = CountReport::from_run(&run, selected).unwrap();
        let json = report.to_canonical_json().unwrap();
        eprintln!("{provider:?} answers no-op={no_op}: {json}");
        if let Some(directory) = std::env::var_os("EKR_KNOWLEDGE_REPORT_DIR") {
            let directory = PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let name = format!(
                "answer-{}-{}",
                match provider {
                    Provider::File => "file",
                    Provider::Sqlite => "sqlite",
                },
                if no_op { "no-op" } else { "persisted" }
            );
            std::fs::write(directory.join(format!("{name}.report.json")), &json).unwrap();
            std::fs::write(
                directory.join(format!("{name}.suite-input.json")),
                input.document().to_canonical_json().unwrap(),
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("{name}.run.json")),
                serde_json::to_vec_pretty(&run.report()).unwrap(),
            )
            .unwrap();
        }
        let counts = report.counts();
        if counts.total != 2
            || counts.error + counts.unsupported + counts.skipped != 0
            || counts.passed != if no_op { 0 } else { 2 }
            || counts.failed != if no_op { 2 } else { 0 }
        {
            failures.push(format!("{provider:?}: {:#?}", run.report()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn human_answers_conform_after_reopen_and_full_replay() {
    run_answers(false);
}
#[test]
fn every_answer_scenario_detects_an_inert_target() {
    run_answers(true);
}

fn run_schema(no_op: bool) {
    let input = select(
        "schema-suite.json",
        ["ekr.kernel/authored/schema-change-exposes-supporting-evidence"],
    );
    let selected = input.selected();
    let mut failures = Vec::new();
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().unwrap();
        let target = upgrade_target::UpgradeTarget::schema(provider, work.path(), no_op);
        let started = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap();
        let run = Runner::new(
            RunnerConfig::default(),
            AdvancingClock::new(started, 1),
            Ids::for_suite(selected.suite()),
        )
        .run_admitted(selected, &target);
        let report = CountReport::from_run(&run, selected).unwrap();
        let json = report.to_canonical_json().unwrap();
        eprintln!("{provider:?} schema no-op={no_op}: {json}");
        if let Some(directory) = std::env::var_os("EKR_KNOWLEDGE_REPORT_DIR") {
            let directory = PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let name = format!(
                "schema-{}-{}",
                if provider == Provider::File {
                    "file"
                } else {
                    "sqlite"
                },
                if no_op { "no-op" } else { "persisted" }
            );
            std::fs::write(directory.join(format!("{name}.report.json")), &json).unwrap();
            std::fs::write(
                directory.join(format!("{name}.suite-input.json")),
                input.document().to_canonical_json().unwrap(),
            )
            .unwrap();
            std::fs::write(
                directory.join(format!("{name}.run.json")),
                serde_json::to_vec_pretty(&run.report()).unwrap(),
            )
            .unwrap();
        }
        let counts = report.counts();
        if counts.total != 1
            || counts.error + counts.unsupported + counts.skipped != 0
            || counts.passed != u64::from(!no_op)
            || counts.failed != u64::from(no_op)
        {
            failures.push(format!("{provider:?}: {:#?}", run.report()));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
#[test]
fn schema_change_exposes_supporting_evidence() {
    run_schema(false);
}
#[test]
fn schema_evidence_scenario_detects_an_inert_target() {
    run_schema(true);
}
