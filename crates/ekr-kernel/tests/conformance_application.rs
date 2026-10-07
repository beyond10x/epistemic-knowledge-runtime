//! Authored E/F acceptance executed against actual retained File and SQLite histories.
#[path = "support/application_conformance/mod.rs"]
mod application;

use ess_conformance::coverage::AdmittedInput;
use ess_conformance::runner::{AdvancingClock, Ids, RunnerConfig};
use ess_conformance::{AdmittedSuite, CountReport, Runner};
use std::path::PathBuf;

const IDS: [&str; 3] = [
    "ekr.integrate/authored/interrupted-integration-resumes-without-duplicates",
    "ekr.integrate/authored/schema-proposal-requires-exact-human-approval",
    "ekr.kernel/authored/canonical-derivation-has-no-transient-dependency",
];

fn run(control: application::Control) {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let raw = std::fs::read_to_string(
        manifest.join("../../systems/ekr/conformance/application-suite.json"),
    )
    .unwrap();
    let all = AdmittedInput::from_suite(AdmittedSuite::from_json(&raw).unwrap()).unwrap();
    let ids: Vec<_> = match control {
        application::Control::Normal | application::Control::Inert => IDS.to_vec(),
        application::Control::DropProvenance => vec![IDS[2]],
        application::Control::DuplicateWrite => vec![IDS[0]],
    }
    .into_iter()
    .map(|id| id.parse().unwrap())
    .collect();
    let input = all.select(&ids).unwrap();
    for sqlite in [false, true] {
        let work = tempfile::tempdir().unwrap();
        let target = application::Target::new(work.path(), sqlite, control);
        let selected = input.selected();
        let started = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .try_into()
            .unwrap();
        let run = Runner::new(
            RunnerConfig::default(),
            AdvancingClock::new(started, 1),
            Ids::for_suite(selected.suite()),
        )
        .run_admitted(selected, &target);
        let report = CountReport::from_run(&run, selected).unwrap();
        let json = report.to_canonical_json().unwrap();
        let name = format!(
            "{}-{}",
            if sqlite { "sqlite" } else { "file" },
            control.name()
        );
        eprintln!("{name}: {json}");
        if let Some(directory) = std::env::var_os("EKR_APPLICATION_REPORT_DIR") {
            let directory = PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
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
            std::fs::write(directory.join(format!("{name}.report.json")), &json).unwrap();
            std::fs::write(
                directory.join(format!("{name}.integrity.json")),
                serde_json::to_vec_pretty(&target.integrity_observations()).unwrap(),
            )
            .unwrap();
        }
        let counts = report.counts();
        assert_eq!(counts.total, ids.len() as u64, "{json}");
        assert_eq!(counts.unsupported + counts.skipped, 0, "{json}");
        match control {
            application::Control::Normal => {
                assert_eq!(counts.passed, 3, "{:#?}", run.report());
                assert_eq!(counts.failed + counts.error, 0, "{json}");
            }
            application::Control::Inert | application::Control::DropProvenance => {
                assert_eq!(counts.failed, ids.len() as u64, "{:#?}", run.report());
                assert_eq!(counts.passed + counts.error, 0, "{json}");
            }
            application::Control::DuplicateWrite => {
                // A real unexpected write is an adapter-integrity error, not an invented
                // product refusal. Keep its report separate from assertion-failure controls.
                assert_eq!(counts.error, 1, "{:#?}", run.report());
                assert_eq!(counts.passed + counts.failed, 0, "{json}");
                assert!(target
                    .integrity_observations()
                    .iter()
                    .any(|s| s == "duplicate-write: repeat changed retained history"));
            }
        }
    }
}

#[test]
fn authored_application_acceptance_on_both_providers() {
    run(application::Control::Normal);
}
#[test]
fn authored_application_rejects_inert_commands() {
    run(application::Control::Inert);
}
#[test]
fn authored_application_rejects_dropped_provenance() {
    run(application::Control::DropProvenance);
}
#[test]
fn authored_application_detects_real_duplicate_write() {
    run(application::Control::DuplicateWrite);
}
