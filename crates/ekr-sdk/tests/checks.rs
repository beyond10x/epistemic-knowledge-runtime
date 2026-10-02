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

fn assert_wire<T>(typed: &T, raw: &Value)
where
    T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    assert_eq!(serde_json::to_value(typed).unwrap(), *raw);
    assert_eq!(&serde_json::from_value::<T>(raw.clone()).unwrap(), typed);
}

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
        Self::with_timestamps(backend, false)
    }

    fn with_timestamps(backend: Backend, dated: bool) -> Self {
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
        let mut document =
            ekr_kernel::SeedDocument::from_bytes(&world.command(&["example", "ekr-seed/2"]))
                .unwrap();
        if dated {
            let property = ekr_ontology::PropertyDefinition::new(
                fixtures::id(0xfa_9000),
                "happened_at",
                ekr_ontology::ValueType::Timestamp,
            );
            document
                .ontology
                .node_types
                .iter_mut()
                .find(|kind| kind.name == "Person")
                .unwrap()
                .properties
                .insert(property.id, property.clone());
            document
                .graph
                .nodes
                .get_mut(&"00000000-0000-4000-8000-000000000301".parse().unwrap())
                .unwrap()
                .properties
                .insert(
                    property.id,
                    vec![ekr_ontology::Value::Timestamp(
                        ekr_core::Timestamp::from_millis(2000),
                    )],
                );
        }
        std::fs::write(&seed, serde_yaml_ng::to_string(&document).unwrap()).unwrap();
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
fn typed_ocel_preserves_document_and_engine_counts_in_both_transports() {
    use ekr_sdk::read::{
        OcelCounts, OcelDocument, OcelEvent, OcelEventAttribute, OcelExport, OcelLog, OcelMeta,
        OcelName, OcelNames, OcelObject, OcelObjectAttribute, OcelQuery, OcelRelationship,
        OcelType, OcelTypeAttribute,
    };
    for backend in [Backend::File, Backend::Sqlite] {
        let world = World::with_timestamps(backend, true);
        let mut session = world.session();
        let mut one = OneShotReader::new(
            &world.binary,
            world.config.clone(),
            SessionOptions::default(),
        );
        let query = OcelQuery {
            revision: Some(0),
            events: vec![],
            event_time: vec!["Person.happened_at".to_owned()],
        };
        let typed: OcelExport = Reader::new(&mut session).ocel(&query).unwrap();
        assert_eq!(typed, one.ocel(&query).unwrap());
        assert_eq!(
            (
                typed.counts.events,
                typed.counts.objects,
                typed.counts.undated_events
            ),
            (1, 1, 1)
        );
        assert_eq!(
            typed.document.ocel.events[0].time,
            "1970-01-01T00:00:02.000Z"
        );
        let reply = session
            .request(&Request::new([
                "ocel",
                "--revision",
                "0",
                "--event-time",
                "Person.happened_at",
            ]))
            .unwrap();
        let raw = reply.document.unwrap();
        assert_wire::<OcelDocument>(&typed.document, &raw);
        assert_wire::<OcelMeta>(&typed.document.meta, &raw["meta"]);
        assert_wire::<OcelNames>(&typed.document.names, &raw["names"]);
        assert_wire::<Vec<OcelName>>(
            &typed.document.names.node_types,
            &raw["names"]["node_types"],
        );
        assert_wire::<OcelLog>(&typed.document.ocel, &raw["ocel"]);
        let log = &typed.document.ocel;
        assert_wire::<Vec<OcelType>>(&log.event_types, &raw["ocel"]["eventTypes"]);
        assert_wire::<Vec<OcelTypeAttribute>>(
            &log.event_types[0].attributes,
            &raw["ocel"]["eventTypes"][0]["attributes"],
        );
        assert_wire::<Vec<OcelEvent>>(&log.events, &raw["ocel"]["events"]);
        assert_wire::<Vec<OcelEventAttribute>>(
            &log.events[0].attributes,
            &raw["ocel"]["events"][0]["attributes"],
        );
        assert_wire::<Vec<OcelRelationship>>(
            &log.events[0].relationships,
            &raw["ocel"]["events"][0]["relationships"],
        );
        assert_wire::<Vec<OcelObject>>(&log.objects, &raw["ocel"]["objects"]);
        assert_wire::<Vec<OcelObjectAttribute>>(
            &log.objects[0].attributes,
            &raw["ocel"]["objects"][0]["attributes"],
        );
        assert_wire::<OcelCounts>(&typed.counts, &serde_json::from_str(&reply.stderr).unwrap());
        assert_eq!(
            serde_json::to_string(&typed.counts).unwrap() + "\n",
            reply.stderr
        );
        assert!(session
            .request(&Request::new(["head"]))
            .unwrap()
            .stderr
            .is_empty());
        for selector in ["Person.absent", "Organization.legal_name"] {
            let invalid = OcelQuery {
                event_time: vec![selector.to_owned()],
                ..OcelQuery::default()
            };
            assert!(matches!(one.ocel(&invalid), Err(ReadError::Refused { .. })));
        }
        let conflict = OcelQuery {
            events: vec!["Person".to_owned()],
            ..query
        };
        assert!(matches!(
            Reader::new(&mut session).ocel(&conflict),
            Err(ReadError::Usage { .. })
        ));
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
            assert_wire::<ekr_sdk::checks::FactSampleMeta>(&sample.meta, &raw["meta"]);
            for (item, raw_item) in sample.items.iter().zip(raw["items"].as_array().unwrap()) {
                assert_wire::<Vec<ekr_sdk::checks::SampledEvidence>>(
                    &item.evidence,
                    &raw_item["evidence"],
                );
            }
            let mut judge = Fixed {
                seen: Vec::new(),
                fail: false,
            };
            let judged =
                ekr_sdk::checks::judge_sample(&sample, NonZeroUsize::new(2).unwrap(), &mut judge)
                    .unwrap();
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
            let raw_report = raw.document.unwrap();
            assert_eq!(serde_json::to_value(&report).unwrap(), raw_report);
            assert_wire::<ekr_sdk::checks::FactQualityMeta>(&report.meta, &raw_report["meta"]);
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
                let verdict = if self.seen.len().is_multiple_of(3) {
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

#[test]
fn adversary_judge_stops_at_the_first_bad_batch_and_preserves_identity_order() {
    use ekr_sdk::checks::JudgeError;
    struct Controlled {
        batches: Vec<Vec<ekr_core::AssertionId>>,
        failure: Option<usize>,
    }
    impl Judge for Controlled {
        type Error = &'static str;
        fn judge(&mut self, facts: &[SampledFact]) -> Result<Vec<Verdict>, Self::Error> {
            self.batches
                .push(facts.iter().map(|fact| fact.assertion.id).collect());
            if self.batches.len() == 2 {
                match self.failure {
                    Some(0) => return Err("second batch unavailable"),
                    Some(count) => return Ok(vec![Verdict::Pass; count]),
                    None => {}
                }
            }
            Ok(facts
                .iter()
                .enumerate()
                .map(|(i, _)| if i == 0 { Verdict::Fail } else { Verdict::Pass })
                .collect())
        }
    }
    let world = World::new(Backend::File);
    let mut session = world.session();
    let sample = Reader::new(&mut session)
        .draw_sample(i64::MIN, 1000, None, None)
        .unwrap();
    assert!(sample.items.len() >= 3, "must reach a second batch");
    let ids: Vec<_> = sample.items.iter().map(|fact| fact.assertion.id).collect();
    let mut good = Controlled {
        batches: vec![],
        failure: None,
    };
    let document = judge_sample(&sample, NonZeroUsize::new(2).unwrap(), &mut good).unwrap();
    assert_eq!(
        good.batches,
        ids.chunks(2).map(<[_]>::to_vec).collect::<Vec<_>>()
    );
    assert_eq!(
        document
            .judgements
            .iter()
            .map(|j| j.assertion)
            .collect::<Vec<_>>(),
        ids
    );
    assert_eq!(
        document
            .judgements
            .iter()
            .map(|j| j.verdict)
            .collect::<Vec<_>>(),
        (0..ids.len())
            .map(|i| if i.is_multiple_of(2) {
                Verdict::Fail
            } else {
                Verdict::Pass
            })
            .collect::<Vec<_>>()
    );
    for failure in [0, 3] {
        let mut bad = Controlled {
            batches: vec![],
            failure: Some(failure),
        };
        let error = judge_sample(&sample, NonZeroUsize::new(2).unwrap(), &mut bad).unwrap_err();
        match error {
            JudgeError::Failed { offset, source } => {
                assert_eq!(failure, 0);
                assert_eq!(offset, 2);
                assert_eq!(source, "second batch unavailable");
            }
            JudgeError::Count {
                offset,
                expected,
                actual,
            } => {
                assert_eq!((offset, expected, actual), (2, (ids.len() - 2).min(2), 3));
            }
        }
        assert_eq!(bad.batches.len(), 2, "must stop before a third call");
    }
    let mut empty = sample.clone();
    empty.items.clear();
    empty.meta.population = 0;
    empty.meta.drawn = 0;
    let mut never = Controlled {
        batches: vec![],
        failure: Some(0),
    };
    let document =
        judge_sample(&empty, NonZeroUsize::new(usize::MAX).unwrap(), &mut never).unwrap();
    assert!(never.batches.is_empty());
    assert!(document.judgements.is_empty());
    assert_eq!(document.sample.unwrap().seed, i64::MIN);
}

#[test]
fn adversary_reports_preserve_exact_bytes_at_every_confidence_and_empty_bounds() {
    for total in [0, 7] {
        let input = ekr_views::FactJudgements {
            sample: Some(ekr_views::SampleOrigin {
                revision: u64::MAX,
                seed: i64::MIN,
                size: 1000,
                type_id: Some(fixtures::id(0x201)),
            }),
            judgements: (0..total)
                .map(|index| ekr_views::Judgement {
                    assertion: fixtures::id(0x700 + index),
                    verdict: if index < 3 {
                        ekr_views::Verdict::Pass
                    } else {
                        ekr_views::Verdict::Fail
                    },
                })
                .collect(),
        };
        for confidence in 1..=9999 {
            let answer = ekr_views::report_fact_quality(&input, Some(confidence)).unwrap();
            let typed: FactQuality = serde_json::from_slice(&answer.bytes).unwrap();
            assert_eq!(
                serde_json::to_vec(&typed).unwrap(),
                answer.bytes,
                "n={total}, confidence={confidence}"
            );
            assert_eq!(typed.meta.sample.as_ref().unwrap().revision, u64::MAX);
            assert_eq!(typed.meta.confidence, u64::try_from(confidence).unwrap());
            if total == 0 {
                assert_eq!((typed.rate, typed.lower, typed.upper), (None, 0.0, 1.0));
                assert_eq!(
                    (
                        answer.summary.rate_bp,
                        answer.summary.lower_bp,
                        answer.summary.upper_bp
                    ),
                    (None, Some(0), Some(10_000))
                );
                let raw: Value = serde_json::from_slice(&answer.bytes).unwrap();
                assert_eq!(raw.get("rate"), Some(&Value::Null));
            } else {
                assert_eq!((typed.judged, typed.passed, typed.failed), (7, 3, 4));
                assert!(typed.lower < 3.0 / 7.0 && typed.upper > 3.0 / 7.0);
            }
        }
    }
}

#[test]
fn adversary_historical_type_filtered_draw_survives_a_later_retraction() {
    for backend in [Backend::File, Backend::Sqlite] {
        let world = World::new(backend);
        let mut session = world.session();
        let mut one = OneShotReader::new(
            &world.binary,
            world.config.clone(),
            SessionOptions::default(),
        );
        let kind = "00000000-0000-4000-8000-000000000201".parse().unwrap();
        let before = Reader::new(&mut session)
            .draw_sample(i64::MAX, 1000, Some(kind), Some(0))
            .unwrap();
        assert!(
            !before.items.is_empty(),
            "positive control for exact-type filter"
        );
        assert!(before.items.iter().all(|fact| fact.subject_type == kind));
        let removed = before.items[0].assertion.id;
        let transaction = ekr_core::TransactionId::mint().to_string();
        let input = format!("format: ekr.transaction-document/2\ntransaction:\n  id: {transaction}\n  proposer: 00000000-0000-4000-8000-000000000101\n  operations:\n  - !RetractAssertion\n    assertion: {removed}\n    reason: adversary historical sample\n  evidence: []\n");
        let proposal = session
            .request(&Request::new(["propose", "-"]).with_stdin(input))
            .unwrap();
        assert_eq!(proposal.exit, 0, "{proposal:?}");
        for (verb, expected) in [("validate", "Validated"), ("commit", "Committed")] {
            let reply = session
                .request(&Request::new([verb, &transaction]))
                .unwrap();
            assert_eq!(reply.exit, 0, "{reply:?}");
            assert_eq!(reply.document.unwrap()["kind"], expected);
        }
        let now = Reader::new(&mut session)
            .draw_sample(i64::MAX, 1000, Some(kind), None)
            .unwrap();
        assert_eq!(now.meta.revision, 1);
        assert_eq!(now.meta.population + 1, before.meta.population);
        assert!(!now.items.iter().any(|fact| fact.assertion.id == removed));
        assert_eq!(
            one.draw_sample(i64::MAX, 1000, Some(kind), Some(0))
                .unwrap(),
            before
        );
        assert_eq!(
            Reader::new(&mut session)
                .draw_sample(i64::MAX, 1000, Some(kind), Some(0))
                .unwrap(),
            before
        );
        let raw = world.command(&[
            "sample",
            "--seed",
            &i64::MAX.to_string(),
            "--size",
            "1000",
            "--type",
            &kind.to_string(),
            "--revision",
            "0",
        ]);
        let mut encoded =
            serde_json::to_vec_pretty(&serde_json::to_value(&before).unwrap()).unwrap();
        encoded.push(b'\n');
        assert_eq!(
            encoded, raw,
            "typed sample matches the historical verb's exact bytes"
        );
    }
}

#[test]
fn adversary_named_refusals_leave_the_session_usable_and_empty_draws_are_real() {
    use ekr_sdk::checks::Judgement;
    fn refused<T: std::fmt::Debug>(result: Result<T, ReadError>, verb: &str, code: &str) {
        match result.unwrap_err() {
            ReadError::Refused {
                verb: actual,
                refusal,
            } => {
                assert_eq!(actual, verb);
                assert_eq!(refusal.code, code);
            }
            other => panic!("expected named refusal: {other:?}"),
        }
    }
    for backend in [Backend::File, Backend::Sqlite] {
        let world = World::new(backend);
        let mut session = world.session();
        let mut reader = Reader::new(&mut session);
        let sample = reader.draw_sample(0, 1000, None, None).unwrap();
        assert!(!sample.items.is_empty());
        for size in [i64::MIN, 0, 1001, i64::MAX] {
            refused(
                reader.draw_sample(0, size, None, None),
                "sample",
                "ekr.views.LimitExceeded",
            );
        }
        let judgement = Judgement {
            assertion: sample.items[0].assertion.id,
            verdict: Verdict::Pass,
        };
        let duplicate = FactJudgements {
            format: "ekr.fact-judgements/1".into(),
            sample: None,
            judgements: vec![judgement.clone(), judgement.clone()],
        };
        refused(
            reader.report_judged(&duplicate, None),
            "fact-quality",
            "ekr.views.JudgedTwice",
        );
        let valid = FactJudgements {
            judgements: vec![judgement],
            ..duplicate
        };
        for confidence in [i64::MIN, 0, 10_000, i64::MAX] {
            refused(
                reader.report_judged(&valid, Some(confidence)),
                "fact-quality",
                "ekr.views.LimitExceeded",
            );
        }
        let report = reader.report_judged(&valid, Some(9999)).unwrap();
        assert_eq!(
            (report.judged, report.passed, report.rate, report.upper),
            (1, 1, Some(1.0), 1.0)
        );
        let absent_type = ekr_core::TypeId::mint();
        let empty = reader
            .draw_sample(0, 1000, Some(absent_type), None)
            .unwrap();
        assert_eq!((empty.meta.population, empty.meta.drawn), (0, 0));
        assert_eq!(empty.meta.type_id, Some(absent_type));
        assert!(empty.items.is_empty());
        assert_eq!(reader.draw_sample(0, 1000, None, None).unwrap(), sample);
    }
}

fn adversary_seeded_world(
    backend: Backend,
    change: impl FnOnce(&mut ekr_kernel::SeedDocument),
) -> World {
    let directory = tempfile::tempdir().unwrap();
    let world = World {
        config: StoreConfig {
            host: directory.path().join("host.json"),
            store: directory.path().join("store"),
            backend,
        },
        _directory: directory,
        binary: EkrBinary::open(binary_path()).unwrap(),
    };
    std::fs::write(
        &world.config.host,
        world.command(&["example", "ekr.cli-host/1"]),
    )
    .unwrap();
    let mut document =
        ekr_kernel::SeedDocument::from_bytes(&world.command(&["example", "ekr-seed/2"])).unwrap();
    change(&mut document);
    let path = world._directory.path().join("adversary-seed.yaml");
    std::fs::write(&path, serde_yaml_ng::to_string(&document).unwrap()).unwrap();
    world.command(&["seed", path.to_str().unwrap()]);
    world
}

fn adversary_timestamp_property(
    document: &mut ekr_kernel::SeedDocument,
    name: &str,
    values: &[i64],
) {
    let mut property = ekr_ontology::PropertyDefinition::new(
        fixtures::id(0xbe_0020),
        "at",
        ekr_ontology::ValueType::Timestamp,
    );
    property.cardinality = ekr_ontology::Cardinality::Many;
    let kind = document
        .ontology
        .node_types
        .iter_mut()
        .find(|kind| kind.name == "Person")
        .unwrap();
    kind.name = name.to_owned();
    kind.properties.insert(property.id, property.clone());
    document
        .graph
        .nodes
        .get_mut(&"00000000-0000-4000-8000-000000000301".parse().unwrap())
        .unwrap()
        .properties
        .insert(
            property.id,
            values
                .iter()
                .map(|value| {
                    ekr_ontology::Value::Timestamp(ekr_core::Timestamp::from_millis(*value))
                })
                .collect(),
        );
}

#[test]
fn adversary_sdk_event_time_treats_a_leading_dash_in_a_type_name_as_data() {
    use ekr_sdk::read::OcelQuery;
    let mut defects = Vec::new();
    for backend in [Backend::File, Backend::Sqlite] {
        let world = adversary_seeded_world(backend, |document| {
            adversary_timestamp_property(document, "-Alert.Kind", &[2000])
        });
        let raw = world.command(&["ocel", "--event-time=-Alert.Kind.at"]);
        let control: Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(
            control["ocel"]["events"][0]["time"], "1970-01-01T00:00:02.000Z",
            "the admitted name is selectable by the actual verb"
        );
        let query = OcelQuery {
            event_time: vec!["-Alert.Kind.at".to_owned()],
            ..OcelQuery::default()
        };
        let mut session = world.session();
        let mut one = OneShotReader::new(
            &world.binary,
            world.config.clone(),
            SessionOptions::default(),
        );
        for (transport, result) in [
            ("session", Reader::new(&mut session).ocel(&query)),
            ("one-shot", one.ocel(&query)),
        ] {
            match result {
                Ok(export) => assert_eq!(serde_json::to_value(export.document).unwrap(), control),
                Err(error) => defects.push(format!("{} {transport}: {error}", backend.as_str())),
            }
        }
    }
    assert!(
        defects.is_empty(),
        "valid selector names must survive SDK argv encoding: {defects:?}"
    );
}

#[test]
fn adversary_named_times_deduplicate_equal_values_and_count_unwritable_instants() {
    use ekr_sdk::read::OcelQuery;
    for backend in [Backend::File, Backend::Sqlite] {
        for (values, expected) in [
            (vec![2000, 2000], Some("1970-01-01T00:00:02.000Z")),
            (vec![-62_167_219_200_000], Some("0000-01-01T00:00:00.000Z")),
            (vec![253_402_300_799_999], Some("9999-12-31T23:59:59.999Z")),
            (vec![i64::MIN], None),
            (vec![i64::MAX], None),
        ] {
            let world = adversary_seeded_world(backend, |document| {
                adversary_timestamp_property(document, "Person", &values)
            });
            let query = OcelQuery {
                event_time: vec!["Person.at".to_owned(), "Person.at".to_owned()],
                ..OcelQuery::default()
            };
            let mut session = world.session();
            let export = Reader::new(&mut session).ocel(&query).unwrap();
            let mut one = OneShotReader::new(
                &world.binary,
                world.config.clone(),
                SessionOptions::default(),
            );
            assert_eq!(one.ocel(&query).unwrap(), export);
            assert_eq!(export.counts.objects, 1);
            if let Some(time) = expected {
                assert_eq!((export.counts.events, export.counts.undated_events), (1, 1));
                assert_eq!(export.document.ocel.events[0].time, time);
            } else {
                assert_eq!((export.counts.events, export.counts.undated_events), (0, 2));
                assert!(export.document.ocel.events.is_empty());
            }
        }
    }
}

#[test]
fn adversary_word_mode_handles_overlapping_punctuation_names_and_scalar_columns() {
    use ekr_sdk::read::CodeNameMode;
    for backend in [Backend::File, Backend::Sqlite] {
        let world = adversary_seeded_world(backend, |document| {
            document
                .graph
                .nodes
                .get_mut(&"00000000-0000-4000-8000-000000000301".parse().unwrap())
                .unwrap()
                .aliases = ["-", "--", "A-B", "éclair", "ZORBED_BY"]
                .into_iter()
                .map(str::to_owned)
                .collect();
        });
        let path = world._directory.path().join("punctuation.rs");
        std::fs::write(&path, "😀 -- A-B éclair ZORBED_BY\néZORBED_BY ZORBED_BY_ ZORBED_BYé éclair2 _éclair aéclair\n\u{301}ZORBED_BY\n'A-B' `éclair`\n").unwrap();
        let files = [path.to_str().unwrap()];
        let mut session = world.session();
        let words = Reader::new(&mut session)
            .code_names_with_mode(files, None, CodeNameMode::Words)
            .unwrap();
        let actual: Vec<_> = serde_json::to_value(&words).unwrap()["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|finding| {
                (
                    finding["line"].as_u64().unwrap(),
                    finding["column"].as_u64().unwrap(),
                    finding["literal"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let expected: Vec<_> = [
            (1, 3, "-"),
            (1, 3, "--"),
            (1, 4, "-"),
            (1, 6, "A-B"),
            (1, 10, "éclair"),
            (1, 17, "ZORBED_BY"),
            (3, 2, "ZORBED_BY"),
            (4, 2, "A-B"),
            (4, 8, "éclair"),
        ]
        .into_iter()
        .map(|(line, column, name)| (line, column, name.to_owned()))
        .collect();
        assert_eq!(actual, expected);
        assert_eq!(words.meta.findings, 9);
        let mut one = OneShotReader::new(
            &world.binary,
            world.config.clone(),
            SessionOptions::default(),
        );
        assert_eq!(
            one.code_names_with_mode(files, None, CodeNameMode::Words)
                .unwrap(),
            words
        );
        let literal = Reader::new(&mut session).code_names(files, None).unwrap();
        assert_eq!((literal.meta.findings, literal.meta.mode), (2, None));
        let mut encoded =
            serde_json::to_vec_pretty(&serde_json::to_value(literal).unwrap()).unwrap();
        encoded.push(b'\n');
        assert_eq!(encoded, world.command(&["code-names", files[0]]));
    }
}

#[test]
fn adversary_ocel_selectors_preserve_equals_spaces_and_punctuation_as_data() {
    use ekr_sdk::read::OcelQuery;
    let mut defects = Vec::new();
    for backend in [Backend::File, Backend::Sqlite] {
        for name in ["=Alert.Kind", "Alert with spaces", "--Alert=two Kind"] {
            let world = adversary_seeded_world(backend, |document| {
                adversary_timestamp_property(document, name, &[2000]);
            });
            for named_time in [true, false] {
                let selector = if named_time {
                    format!("{name}.at")
                } else {
                    name.to_owned()
                };
                let option = if named_time { "event-time" } else { "events" };
                let flag = format!("--{option}={selector}");
                let raw = world.command(&["ocel", &flag]);
                let control: Value = serde_json::from_slice(&raw).unwrap();
                assert!(
                    !control["ocel"]["events"].as_array().unwrap().is_empty(),
                    "the admitted name is selectable by the actual verb"
                );
                let query = if named_time {
                    OcelQuery {
                        event_time: vec![selector.clone(), selector],
                        ..OcelQuery::default()
                    }
                } else {
                    OcelQuery {
                        events: vec![selector.clone(), selector],
                        ..OcelQuery::default()
                    }
                };
                let mut session = world.session();
                let mut one = OneShotReader::new(
                    &world.binary,
                    world.config.clone(),
                    SessionOptions::default(),
                );
                for (transport, result) in [
                    ("session", Reader::new(&mut session).ocel(&query)),
                    ("one-shot", one.ocel(&query)),
                ] {
                    match result {
                        Ok(export) => {
                            assert_eq!(serde_json::to_value(export.document).unwrap(), control)
                        }
                        Err(error) => defects.push(format!(
                            "{} {transport} {option} {name:?}: {error}",
                            backend.as_str()
                        )),
                    }
                }
                assert_eq!(session.request(&Request::new(["head"])).unwrap().stderr, "");
            }
        }
    }
    assert!(
        defects.is_empty(),
        "OCEL names must survive SDK argv encoding in both modes: {defects:?}"
    );
}
