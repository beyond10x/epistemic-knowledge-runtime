//! Typed sampled-fact checks, through the same transport as the other SDK reads.
#![cfg(unix)]

#[path = "../../ekr-views/tests/support/fixtures.rs"]
#[allow(dead_code, clippy::all, clippy::pedantic)]
mod fixtures;

use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::checks::{
    judge_sample, FactJudgements, FactQuality, FactSample, Judge, SampledFact, Verdict,
};
use ekr_sdk::read::{OneShotReader, ReadError, Reader};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport};
use serde_json::Value;

fn binary_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
        let output = std::process::Command::new(std::env::var_os("CARGO").unwrap())
            .args([
                "build",
                "--locked",
                "-p",
                "ekr",
                "--bin",
                "ekr",
                "--message-format=json",
            ])
            .current_dir(root)
            .stderr(std::process::Stdio::inherit())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "building the tested ekr: {:?}",
            output.status
        );
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find_map(|message| {
                (message["target"]["name"] == "ekr")
                    .then(|| message["executable"].as_str().map(PathBuf::from))
                    .flatten()
            })
            .expect("cargo returned the executable")
    })
}

struct World {
    _directory: tempfile::TempDir,
    config: StoreConfig,
    binary: EkrBinary,
}

impl World {
    fn new(backend: Backend) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let config = StoreConfig {
            host: directory.path().join("host.json"),
            store: directory.path().join("store"),
            backend,
        };
        let world = Self {
            _directory: directory,
            config,
            binary: EkrBinary::open(binary_path()).unwrap(),
        };
        std::fs::write(
            &world.config.host,
            world.command(&["example", "ekr.cli-host/1"]),
        )
        .unwrap();
        let seed = world._directory.path().join("seed.yaml");
        std::fs::write(&seed, world.command(&["example", "ekr-seed/2"])).unwrap();
        world.command(&["seed", seed.to_str().unwrap()]);
        world
    }

    fn command(&self, argv: &[&str]) -> Vec<u8> {
        let output = std::process::Command::new(binary_path())
            .env_clear()
            .arg("--host")
            .arg(&self.config.host)
            .arg("--store")
            .arg(&self.config.store)
            .args(["--backend", self.config.backend.as_str()])
            .args(argv)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{argv:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    fn session(&self) -> ProcessSession {
        ProcessSession::start(&self.binary, self.config.clone(), SessionOptions::default()).unwrap()
    }
}

#[test]
fn sample_and_report_roundtrip_the_verbs_on_both_providers_and_transports() {
    for backend in [Backend::File, Backend::Sqlite] {
        let world = World::new(backend);
        let mut session = world.session();
        let mut one = OneShotReader::new(
            &world.binary,
            world.config.clone(),
            SessionOptions::default(),
        );
        for revision in [None, Some(0)] {
            let sample = Reader::new(&mut session)
                .draw_sample(-7, 3, None, revision)
                .unwrap();
            assert_eq!(sample, one.draw_sample(-7, 3, None, revision).unwrap());
            assert_eq!(
                sample,
                Reader::new(&mut session)
                    .draw_sample(-7, 3, None, revision)
                    .unwrap()
            );
            let raw: Value = serde_json::from_slice(&world.command(&[
                "sample",
                "--seed",
                "-7",
                "--size",
                "3",
                "--revision",
                "0",
            ]))
            .unwrap();
            assert_eq!(serde_json::to_value(&sample).unwrap(), raw);
            let mut judge = Fixed {
                seen: Vec::new(),
                fail: false,
            };
            let judged = judge_sample(&sample, NonZeroUsize::new(2).unwrap(), &mut judge).unwrap();
            let report = Reader::new(&mut session)
                .report_judged(&judged, None)
                .unwrap();
            assert_eq!(report, one.report_judged(&judged, None).unwrap());
            let raw = session
                .request(
                    &Request::new(["fact-quality", "-"])
                        .with_stdin(serde_json::to_string(&judged).unwrap()),
                )
                .unwrap();
            assert_eq!(
                serde_json::to_value(&report).unwrap(),
                raw.document.unwrap()
            );
            assert_eq!(
                judge.seen,
                sample
                    .items
                    .iter()
                    .map(|item| item.assertion.id.to_string())
                    .collect::<Vec<_>>()
            );
        }
        assert!(matches!(
            Reader::new(&mut session).draw_sample(0, 0, None, None),
            Err(ReadError::Refused { .. })
        ));
        assert!(matches!(
            one.draw_sample(0, 1, None, Some(u64::MAX)),
            Err(ReadError::Refused { .. })
        ));
        let person = "00000000-0000-4000-8000-000000000201".parse().unwrap();
        let filtered = one.draw_sample(0, 1000, Some(person), Some(0)).unwrap();
        assert!(filtered
            .items
            .iter()
            .all(|fact| fact.subject_type == person));
        assert_eq!(filtered.meta.type_id, Some(person));
    }
}

struct Fixed {
    seen: Vec<String>,
    fail: bool,
}

impl Judge for Fixed {
    type Error = &'static str;

    fn judge(&mut self, facts: &[SampledFact]) -> Result<Vec<Verdict>, Self::Error> {
        if self.fail && !self.seen.is_empty() {
            return Err("judge unavailable");
        }
        Ok(facts
            .iter()
            .map(|fact| {
                let verdict = if self.seen.len() % 3 == 0 {
                    Verdict::Fail
                } else {
                    Verdict::Pass
                };
                self.seen.push(fact.assertion.id.to_string());
                verdict
            })
            .collect())
    }
}

#[test]
fn fixed_judge_gets_closed_form_bounds_and_an_error_returns_no_judgement_set() {
    let world = World::new(Backend::Sqlite);
    let mut session = world.session();
    let sample = Reader::new(&mut session)
        .draw_sample(5, 3, None, None)
        .unwrap();
    let mut judge = Fixed {
        seen: Vec::new(),
        fail: false,
    };
    let judged = judge_sample(&sample, NonZeroUsize::new(2).unwrap(), &mut judge).unwrap();
    let report = Reader::new(&mut session)
        .report_judged(&judged, Some(9500))
        .unwrap();
    assert_eq!((report.judged, report.passed, report.failed), (3, 2, 1));
    assert!((report.lower - 0.207_659_600_802_047_7).abs() < 1e-14);
    assert!((report.upper - 0.938_508_055_279_603_7).abs() < 1e-14);
    let mut failing = Fixed {
        seen: Vec::new(),
        fail: true,
    };
    assert!(judge_sample(&sample, NonZeroUsize::new(2).unwrap(), &mut failing).is_err());
    assert_eq!(failing.seen.len(), 2);
    struct Missing;
    impl Judge for Missing {
        type Error = &'static str;
        fn judge(&mut self, _: &[SampledFact]) -> Result<Vec<Verdict>, Self::Error> {
            Ok(Vec::new())
        }
    }
    assert!(judge_sample(&sample, NonZeroUsize::new(2).unwrap(), &mut Missing).is_err());
    let empty = FactJudgements {
        format: "ekr.fact-judgements/1".to_owned(),
        sample: None,
        judgements: vec![],
    };
    let report = Reader::new(&mut session)
        .report_judged(&empty, None)
        .unwrap();
    assert_eq!((report.rate, report.lower, report.upper), (None, 0.0, 1.0));
    assert_eq!(serde_json::to_value(report).unwrap()["rate"], Value::Null);
}

#[test]
fn every_sample_fixture_document_is_typed_without_dropping_fields() {
    for provider in [fixtures::Provider::File, fixtures::Provider::Sqlite] {
        let directory = tempfile::tempdir().unwrap();
        let runtime = fixtures::open(directory.path(), provider);
        fixtures::Fixture::named("quality").unwrap().build(&runtime);
        for revision in [0, 1, 2, 3] {
            let answer = ekr_views::draw_sample(
                &runtime,
                Some(ekr_core::RevisionNumber::new(revision)),
                &ekr_views::SampleRequest::new(5, 1000, None).unwrap(),
            )
            .unwrap();
            let document: Value = serde_json::from_slice(&answer.bytes).unwrap();
            let typed: FactSample = serde_json::from_value(document.clone()).unwrap();
            assert_eq!(serde_json::to_vec(&typed).unwrap(), answer.bytes);
            assert_eq!(serde_json::to_value(typed).unwrap(), document);
        }
    }
    for count in [0, 1, 3] {
        let verdicts = (0..count)
            .map(|index| ekr_views::Judgement {
                assertion: fixtures::id(100 + index),
                verdict: ekr_views::Verdict::Pass,
            })
            .collect();
        let answer = ekr_views::report_fact_quality(
            &ekr_views::FactJudgements {
                sample: None,
                judgements: verdicts,
            },
            Some(9999),
        )
        .unwrap();
        let document: Value = serde_json::from_slice(&answer.bytes).unwrap();
        let typed: FactQuality = serde_json::from_value(document.clone()).unwrap();
        assert_eq!(serde_json::to_vec(&typed).unwrap(), answer.bytes);
        assert_eq!(serde_json::to_value(typed).unwrap(), document);
    }
}
