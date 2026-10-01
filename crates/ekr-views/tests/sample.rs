//! `story:fact-quality-by-judged-sample`: DrawFactSample (`ekr.fact-sample/1`) and
//! ReportFactQuality (`ekr.fact-quality/1`), held against `systems/ekr/domains/views.yaml` on
//! both native providers.
//!
//! * The same seed, size and revision draw the same sample on both providers, byte for byte, and
//!   the sample is the one the specification's draw key names: the `size` Active assertions with
//!   the lowest SHA-256 of `ekr.fact-sample/1:<seed>:<assertion id>`.
//! * For a judged fixture of known results the reported rate and interval equal the closed-form
//!   Wilson values, computed here from the published normal quantiles and the centre ± half-width
//!   form, which is not the form the report computes.

mod support;

use std::collections::BTreeSet;

use ekr_core::{AssertionId, RevisionNumber, TypeId};
use ekr_graph::AssertionLifecycle;
use ekr_kernel::Runtime;
use ekr_views::{
    draw_sample, report_fact_quality, wilson_z, FactJudgements, FactQualityError, FactSampleDrawn,
    Judgement, LimitExceeded, ProjectError, SampleOrigin, SampleRequest, Verdict,
    FACT_QUALITY_FORMAT, JUDGEMENTS_FORMAT, SAMPLE_FORMAT,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use support::fixtures::{
    self, Fixture, Provider, O_ASSERTIONS, O_CASE, O_NOTE, O_STEP, Q_ASSERTIONS, Q_ITEM, Q_OTHER,
};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

fn built(provider: Provider, fixture: Fixture) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    fixture.build(&runtime);
    (work, runtime)
}

fn request(seed: i64, size: i64, type_id: Option<u64>) -> SampleRequest {
    SampleRequest::new(seed, size, type_id.map(fixtures::id::<TypeId>)).expect("in bounds")
}

/// The sample of revision `at` (the head when `None`): its bytes, parsed, and its event.
fn draw(
    runtime: &Runtime,
    at: Option<u64>,
    request: &SampleRequest,
) -> (Vec<u8>, Value, FactSampleDrawn) {
    let answer = draw_sample(runtime, at.map(RevisionNumber::new), request).expect("drawn");
    let value: Value = serde_json::from_slice(&answer.bytes).expect("JSON");
    (answer.bytes, value, answer.summary)
}

fn drawn_ids(document: &Value) -> Vec<String> {
    document["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| item["assertion"]["id"].as_str().expect("id").to_owned())
        .collect()
}

/// The specification's draw, computed here from the revision's graph: every Active assertion
/// ranked by its draw key, then id, the `size` lowest.
fn specified_draw(runtime: &Runtime, at: u64, seed: i64, size: usize) -> Vec<String> {
    let loaded = ekr_views::load(runtime, Some(RevisionNumber::new(at))).expect("loaded");
    let mut ranked: Vec<(String, String)> = loaded
        .graph
        .assertions
        .values()
        .filter(|assertion| matches!(assertion.lifecycle, AssertionLifecycle::Active))
        .map(|assertion| {
            let id = assertion.id.to_string();
            let key = hex::encode(Sha256::digest(
                format!("ekr.fact-sample/1:{seed}:{id}").as_bytes(),
            ));
            (key, id)
        })
        .collect();
    ranked.sort();
    ranked.into_iter().take(size).map(|(_, id)| id).collect()
}

#[test]
fn the_same_seed_size_and_revision_draw_the_same_sample_on_both_providers() {
    let mut answers = Vec::new();
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider, Fixture::Quality);
        for at in [Some(1), Some(3), None] {
            let first = draw(&runtime, at, &request(7, 3, None));
            let second = draw(&runtime, at, &request(7, 3, None));
            assert_eq!(
                first.0,
                second.0,
                "{}: two draws of one request",
                provider.name()
            );
            answers.push((provider, at, first));
        }
        let head = draw(&runtime, None, &request(7, 3, None));
        let named = draw(&runtime, Some(3), &request(7, 3, None));
        assert_eq!(head.0, named.0, "the head's draw is the draw naming it");

        // The draw is the specification's, not merely a stable one.
        for (at, population) in [(1_u64, 5_u64), (3, 4)] {
            let (_, document, summary) = draw(&runtime, Some(at), &request(7, 3, None));
            assert_eq!(drawn_ids(&document), specified_draw(&runtime, at, 7, 3));
            assert_eq!(summary.population, population, "revision {at}");
            assert_eq!(summary.drawn, 3);
            assert_eq!(summary.revision, at);
            assert_eq!(
                summary.first_assertion.map(|id| id.to_string()),
                drawn_ids(&document).first().cloned()
            );
            assert_eq!(
                summary.sample_hash,
                hex::encode(Sha256::digest(
                    &draw(&runtime, Some(at), &request(7, 3, None)).0
                ))
            );
        }
    }
    let (file, sqlite): (Vec<_>, Vec<_>) = answers
        .into_iter()
        .partition(|(provider, _, _)| *provider == Provider::File);
    for ((_, at, on_file), (_, _, on_sqlite)) in file.iter().zip(&sqlite) {
        assert_eq!(
            on_file.0, on_sqlite.0,
            "revision {at:?} on the two providers"
        );
        assert_eq!(on_file.2, on_sqlite.2);
    }
}

#[test]
fn another_seed_draws_another_sample_and_a_larger_size_extends_the_smaller() {
    let (_work, runtime) = built(Provider::File, Fixture::Quality);
    let seeds: BTreeSet<Vec<String>> = [0_i64, 1, 2, 3, 4, 5, -1, i64::MIN, i64::MAX]
        .into_iter()
        .map(|seed| drawn_ids(&draw(&runtime, Some(1), &request(seed, 2, None)).1))
        .collect();
    assert!(seeds.len() > 1, "every seed drew one sample: {seeds:?}");
    for seed in [-1, i64::MIN, i64::MAX] {
        let (_, document, _) = draw(&runtime, Some(1), &request(seed, 5, None));
        assert_eq!(drawn_ids(&document), specified_draw(&runtime, 1, seed, 5));
    }

    let whole = drawn_ids(&draw(&runtime, Some(1), &request(11, 1000, None)).1);
    assert_eq!(
        whole.len(),
        5,
        "a size above the population draws all of it"
    );
    for size in 1..=5 {
        let part = drawn_ids(&draw(&runtime, Some(1), &request(11, size, None)).1);
        assert_eq!(part, whole[..usize::try_from(size).unwrap()], "size {size}");
    }
}

#[test]
fn a_later_commit_leaves_an_earlier_revisions_sample_as_it_was() {
    let (_work, runtime) = built(Provider::Sqlite, Fixture::Quality);
    let before = draw(&runtime, Some(3), &request(5, 4, None)).0;
    fixtures::commit_later_quality(&runtime);
    assert_eq!(draw(&runtime, Some(3), &request(5, 4, None)).0, before);
    let (_, document, summary) = draw(&runtime, None, &request(5, 4, None));
    assert_eq!(summary.revision, 4);
    assert_eq!(summary.population, 5);
    assert_eq!(document["meta"]["revision"], 4);
}

#[test]
fn the_document_carries_the_request_and_each_fact_with_its_names_and_evidence_bytes() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider, Fixture::Quality);
        let (bytes, document, summary) = draw(&runtime, Some(1), &request(3, 1000, None));
        assert_eq!(
            document["meta"],
            json!({
                "format": SAMPLE_FORMAT,
                "revision": 1,
                "seed": 3,
                "size": 1000,
                "population": 5,
                "drawn": 5,
            })
        );
        assert!(!bytes.contains(&b'\n'), "no insignificant whitespace");
        let read = runtime
            .read(Some(RevisionNumber::new(1)))
            .expect("verified read");
        let mut cited = 0;
        for item in document["items"].as_array().unwrap() {
            let id: AssertionId = item["assertion"]["id"].as_str().unwrap().parse().unwrap();
            let assertion = &read.graph.assertions[&id];
            let ekr_graph::Subject::Node(node) = &assertion.subject else {
                panic!("the quality fixture's facts are about nodes")
            };
            let node = &read.graph.nodes[&node.id()];
            assert_eq!(item["subject_kind"], "Node");
            assert_eq!(item["subject"], node.id.to_string());
            assert_eq!(
                item["subject_type"],
                fixtures::id::<TypeId>(Q_ITEM).to_string()
            );
            assert_eq!(item["subject_name"], node.canonical_name.as_str());
            assert_eq!(item["predicate_name"], "title");
            assert!(
                item.get("object_name").is_none(),
                "a value object has no name"
            );
            assert_eq!(item["assertion"]["lifecycle"]["kind"], "Active");
            let evidence = item["evidence"].as_array().unwrap();
            let ids: Vec<String> = evidence
                .iter()
                .map(|record| record["id"].as_str().unwrap().to_owned())
                .collect();
            let mut sorted = ids.clone();
            sorted.sort();
            assert_eq!(ids, sorted, "each item's evidence by id");
            assert_eq!(
                ids,
                item["assertion"]["evidence"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|id| id.as_str().unwrap().to_owned())
                    .collect::<Vec<_>>()
            );
            for record in evidence {
                cited += 1;
                let id = record["id"].as_str().unwrap().parse().unwrap();
                let held = &read.graph.evidence[&id];
                assert_eq!(record["content_hash"], held.content_hash.to_string());
                assert_eq!(record["kind"], "HumanStatement");
                let bytes = read.content(&held.content_hash).expect("retained bytes");
                assert_eq!(record["text"], std::str::from_utf8(bytes).unwrap());
                assert!(record.get("base64").is_none(), "{record}");
            }
        }
        assert_eq!(summary.evidence, cited);
        assert_eq!(cited, 7, "a1 1, a2 2, a3 1, a4 1, a5 2");
    }
}

#[test]
fn a_type_filter_draws_only_the_facts_whose_subject_is_of_that_type() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider, Fixture::Ocel);
        let assertion = |n: u64| fixtures::id::<AssertionId>(O_ASSERTIONS + n).to_string();
        for (type_id, expected) in [
            (O_STEP, vec![assertion(2), assertion(3), assertion(4)]),
            (O_CASE, vec![assertion(1)]),
            (O_NOTE, vec![assertion(5)]),
        ] {
            let (_, document, summary) = draw(&runtime, Some(0), &request(9, 10, Some(type_id)));
            let mut ids = drawn_ids(&document);
            ids.sort();
            assert_eq!(ids, expected, "{}: type {type_id:x}", provider.name());
            assert_eq!(summary.population, expected.len() as u64);
            assert_eq!(
                document["meta"]["type"],
                fixtures::id::<TypeId>(type_id).to_string()
            );
        }

        // A type no fact's subject is of, declared or not, draws nothing and is no refusal.
        let (_quality_work, quality) = built(provider, Fixture::Quality);
        for type_id in [Q_OTHER, 0xdead_beef] {
            let (_, document, summary) = draw(&quality, None, &request(1, 5, Some(type_id)));
            assert_eq!(summary.population, 0);
            assert_eq!(summary.drawn, 0);
            assert_eq!(summary.first_assertion, None);
            assert_eq!(document["items"], json!([]));
        }
        let (_, _, summary) = draw(&quality, None, &request(1, 5, Some(Q_ITEM)));
        assert_eq!(summary.population, 4);
    }
}

#[test]
fn a_fact_about_an_edge_names_its_edge_type_and_no_subject_name() {
    let (_work, runtime) = built(Provider::File, Fixture::EdgeAssertion);
    let (_, document, _) = draw(&runtime, None, &request(0, 10, None));
    let items = document["items"].as_array().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["subject_kind"], "Edge");
    assert!(items[0].get("subject_name").is_none(), "{}", items[0]);
    let read = runtime.read(None).unwrap();
    let edge = read
        .graph
        .edges
        .values()
        .next()
        .expect("the fixture's edge");
    assert_eq!(items[0]["subject"], edge.id.to_string());
    assert_eq!(items[0]["subject_type"], edge.type_id.to_string());
}

#[test]
fn a_size_outside_1_to_1000_is_refused_before_the_store_is_read() {
    for (size, maximum) in [(0, Some(1000)), (1001, Some(1000)), (-3, Some(1000))] {
        let refusal = SampleRequest::new(1, size, None).expect_err("out of bounds");
        assert_eq!(
            refusal,
            LimitExceeded {
                parameter: "size",
                requested: size,
                minimum: 1,
                maximum,
            }
        );
    }
    assert!(SampleRequest::new(1, 1, None).is_ok());
    assert!(SampleRequest::new(1, 1000, None).is_ok());
}

#[test]
fn an_unseeded_store_and_a_revision_beyond_the_head_are_refused_by_name() {
    let work = tempfile::tempdir().unwrap();
    let unseeded = fixtures::open(work.path(), Provider::File);
    assert!(matches!(
        draw_sample(&unseeded, None, &request(1, 1, None)),
        Err(ProjectError::NotSeeded { requested: None })
    ));
    let (_work, runtime) = built(Provider::Sqlite, Fixture::Quality);
    match draw_sample(&runtime, Some(RevisionNumber::new(9)), &request(1, 1, None)) {
        Err(ProjectError::RevisionNotFound { requested, head }) => {
            assert_eq!((requested.get(), head.get()), (9, 3));
        }
        other => panic!("{other:?}"),
    }
}

// ---- the report --------------------------------------------------------------------------------

/// Φ⁻¹((1 + c) / 2) for the confidences below, as published to double precision.
const QUANTILES: [(u64, f64); 6] = [
    (5_000, 0.674_489_750_196_081_7),
    (6_000, 0.841_621_233_572_914_3),
    (9_000, 1.644_853_626_951_472_2),
    (9_500, 1.959_963_984_540_054),
    (9_900, 2.575_829_303_548_900_4),
    (9_999, 3.890_591_886_413_094),
];

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= 1e-12 * expected.abs().max(1.0),
        "{what}: {actual} is not {expected}"
    );
}

#[test]
fn z_is_the_published_normal_quantile_at_the_stated_confidence() {
    for (confidence, quantile) in QUANTILES {
        close(
            wilson_z(confidence),
            quantile,
            &format!("z at {confidence}"),
        );
    }
    // Symmetric and increasing across the range.
    let mut previous = 0.0;
    for confidence in (1..=9_999).step_by(7) {
        let z = wilson_z(confidence);
        assert!(z > previous, "z at {confidence} is {z}, after {previous}");
        previous = z;
    }
}

fn judged(verdicts: &[(u64, Verdict)]) -> FactJudgements {
    FactJudgements {
        sample: None,
        judgements: verdicts
            .iter()
            .map(|(n, verdict)| Judgement {
                assertion: fixtures::id(Q_ASSERTIONS + n),
                verdict: *verdict,
            })
            .collect(),
    }
}

fn of(passed: u64, failed: u64) -> FactJudgements {
    let verdicts: Vec<(u64, Verdict)> = (0..passed)
        .map(|n| (n, Verdict::Pass))
        .chain((passed..passed + failed).map(|n| (n, Verdict::Fail)))
        .collect();
    judged(&verdicts)
}

/// The Wilson score interval in its centre ± half-width form.
fn closed_form(passed: u64, judged: u64, z: f64) -> (f64, f64) {
    let (k, n) = (passed as f64, judged as f64);
    let p = k / n;
    let z2 = z * z;
    let denominator = 1.0 + z2 / n;
    let centre = (p + z2 / (2.0 * n)) / denominator;
    let half = z / denominator * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
    ((centre - half).max(0.0), (centre + half).min(1.0))
}

#[test]
fn for_a_judged_fixture_of_known_results_the_rate_and_interval_equal_the_closed_form_wilson_values()
{
    for (confidence, z) in QUANTILES {
        for (passed, failed) in [(8, 2), (0, 10), (10, 0), (1, 0), (0, 1), (37, 63), (199, 1)] {
            let answer = report_fact_quality(&of(passed, failed), Some(confidence as i64))
                .expect("reported");
            let document: Value = serde_json::from_slice(&answer.bytes).unwrap();
            let judged = passed + failed;
            let (lower, upper) = closed_form(passed, judged, z);
            let what = format!("{passed}/{judged} at {confidence}");
            assert_eq!(document["judged"], judged, "{what}");
            assert_eq!(document["passed"], passed, "{what}");
            assert_eq!(document["failed"], failed, "{what}");
            assert_eq!(document["meta"]["confidence"], confidence, "{what}");
            close(document["meta"]["z"].as_f64().unwrap(), z, &what);
            close(
                document["rate"].as_f64().unwrap(),
                passed as f64 / judged as f64,
                &what,
            );
            close(document["lower"].as_f64().unwrap(), lower, &what);
            close(document["upper"].as_f64().unwrap(), upper, &what);
            if passed == 0 {
                assert_eq!(document["lower"].as_f64(), Some(0.0), "{what}");
            }
            if failed == 0 {
                assert_eq!(document["upper"].as_f64(), Some(1.0), "{what}");
            }
            let summary = &answer.summary;
            assert_eq!(summary.judged, judged);
            assert_eq!(summary.rate_bp, Some(passed * 10_000 / judged), "{what}");
            let floor = |x: f64| (x * 10_000.0).floor() as u64;
            assert_eq!(
                summary.lower_bp,
                Some(floor(document["lower"].as_f64().unwrap()))
            );
            assert_eq!(
                summary.upper_bp,
                Some(floor(document["upper"].as_f64().unwrap()))
            );
            assert_eq!(
                summary.fact_quality_hash,
                hex::encode(Sha256::digest(&answer.bytes))
            );
        }
    }
    // The textbook figure: 8 of 10 at 95 % is (0.4902, 0.9433).
    let answer = report_fact_quality(&of(8, 2), None).unwrap();
    assert_eq!(answer.summary.confidence, 9_500, "9500 when none is stated");
    assert_eq!(
        (answer.summary.lower_bp, answer.summary.upper_bp),
        (Some(4_901), Some(9_433))
    );
}

#[test]
fn the_report_is_the_same_bytes_for_the_same_input_and_echoes_the_sample() {
    let mut judgements = of(3, 1);
    judgements.sample = Some(SampleOrigin {
        revision: 3,
        seed: -4,
        size: 4,
        type_id: Some(fixtures::id(Q_ITEM)),
    });
    let first = report_fact_quality(&judgements, Some(9_000)).unwrap();
    let second = report_fact_quality(&judgements, Some(9_000)).unwrap();
    assert_eq!(first, second);
    let document: Value = serde_json::from_slice(&first.bytes).unwrap();
    assert_eq!(document["meta"]["format"], FACT_QUALITY_FORMAT);
    assert_eq!(
        document["meta"]["sample"],
        json!({
            "revision": 3,
            "seed": -4,
            "size": 4,
            "type": fixtures::id::<TypeId>(Q_ITEM).to_string(),
        })
    );
    let keys: Vec<&String> = document.as_object().unwrap().keys().collect();
    assert_eq!(keys.len(), 7, "{keys:?}");
    let text = String::from_utf8(first.bytes).unwrap();
    assert!(
        text.starts_with(r#"{"meta":{"format":"ekr.fact-quality/1","confidence":9000,"z":"#),
        "{text}"
    );
    assert!(
        text.contains(r#""judged":4,"passed":3,"failed":1,"rate":0.75,"lower":"#),
        "{text}"
    );
}

#[test]
fn no_judgement_reports_no_rate_and_no_interval() {
    let answer = report_fact_quality(&of(0, 0), None).unwrap();
    let document: Value = serde_json::from_slice(&answer.bytes).unwrap();
    assert_eq!(document["judged"], 0);
    for omitted in ["rate", "lower", "upper"] {
        assert!(document.get(omitted).is_none(), "{document}");
    }
    assert_eq!(
        (
            answer.summary.rate_bp,
            answer.summary.lower_bp,
            answer.summary.upper_bp
        ),
        (None, None, None)
    );
}

#[test]
fn a_confidence_outside_1_to_9999_and_a_fact_judged_twice_are_refused() {
    for confidence in [0, 10_000, -1] {
        assert_eq!(
            report_fact_quality(&of(1, 1), Some(confidence)),
            Err(FactQualityError::LimitExceeded(LimitExceeded {
                parameter: "confidence",
                requested: confidence,
                minimum: 1,
                maximum: Some(9_999),
            }))
        );
    }
    let twice = judged(&[
        (1, Verdict::Pass),
        (2, Verdict::Fail),
        (1, Verdict::Fail),
        (2, Verdict::Pass),
    ]);
    assert_eq!(
        report_fact_quality(&twice, None),
        Err(FactQualityError::JudgedTwice {
            assertion: fixtures::id(Q_ASSERTIONS + 1)
        })
    );
    // The bound is checked first.
    assert!(matches!(
        report_fact_quality(&twice, Some(0)),
        Err(FactQualityError::LimitExceeded(_))
    ));
}

#[test]
fn the_judgements_document_reads_and_writes_its_format_and_refuses_anything_else() {
    let assertion = fixtures::id::<AssertionId>(Q_ASSERTIONS + 1).to_string();
    let text = json!({
        "format": JUDGEMENTS_FORMAT,
        "sample": {"revision": 1, "seed": 7, "size": 3},
        "judgements": [{"assertion": assertion, "verdict": "Pass"}],
    })
    .to_string();
    let read = FactJudgements::from_json(text.as_bytes()).expect("a judged sample");
    assert_eq!(read.judgements.len(), 1);
    assert_eq!(read.judgements[0].verdict, Verdict::Pass);
    assert_eq!(read.sample.as_ref().map(|origin| origin.seed), Some(7));
    let written = read.to_json();
    assert_eq!(FactJudgements::from_json(&written).unwrap(), read);
    let written: Value = serde_json::from_slice(&written).unwrap();
    assert_eq!(written["format"], JUDGEMENTS_FORMAT);

    for refused in [
        json!({"format": "ekr.fact-judgements/2", "judgements": []}),
        json!({"judgements": []}),
        json!({"format": JUDGEMENTS_FORMAT}),
        json!({"format": JUDGEMENTS_FORMAT, "judgements": [], "extra": 1}),
        json!({"format": JUDGEMENTS_FORMAT, "judgements": [{"assertion": assertion, "verdict": "pass"}]}),
        json!({"format": JUDGEMENTS_FORMAT, "judgements": [{"assertion": "a1", "verdict": "Pass"}]}),
    ] {
        assert!(
            FactJudgements::from_json(refused.to_string().as_bytes()).is_err(),
            "{refused}"
        );
    }
}

/// The sample's public pieces, each held to what the draw and the report do with it: the draw is
/// the Active assertions ranked by [`ekr_views::draw_key`], the size bound is
/// [`ekr_views::SampleRequest::MAX_SIZE`], both documents carry their format constant, a report
/// with no stated confidence is the one at [`ekr_views::DEFAULT_CONFIDENCE`], its numbers are
/// [`ekr_views::wilson_interval`] at [`ekr_views::wilson_z`], and a document that is not a
/// judged sample is refused as [`ekr_views::JudgementsMalformed`].
#[test]
fn the_draw_key_bounds_formats_and_interval_functions_are_what_the_draw_and_report_use() {
    let (_work, runtime) = built(Provider::File, Fixture::Quality);
    let loaded = ekr_views::load(&runtime, Some(RevisionNumber::new(1))).expect("loaded");
    for assertion in loaded.graph.assertions.values() {
        assert_eq!(
            ekr_views::draw_key(7, assertion.id),
            hex::encode(Sha256::digest(
                format!("ekr.fact-sample/1:7:{}", assertion.id).as_bytes()
            ))
        );
    }
    let mut ranked: Vec<(String, String)> = loaded
        .graph
        .assertions
        .values()
        .filter(|assertion| matches!(assertion.lifecycle, AssertionLifecycle::Active))
        .map(|assertion| {
            (
                ekr_views::draw_key(7, assertion.id),
                assertion.id.to_string(),
            )
        })
        .collect();
    ranked.sort();
    let (_, document, _) = draw(&runtime, Some(1), &request(7, 3, None));
    assert_eq!(
        drawn_ids(&document),
        ranked
            .into_iter()
            .take(3)
            .map(|(_, id)| id)
            .collect::<Vec<_>>()
    );
    assert_eq!(document["meta"]["format"], ekr_views::SAMPLE_FORMAT);

    let largest = ekr_views::SampleRequest::MAX_SIZE;
    assert_eq!(largest, 1000);
    assert!(SampleRequest::new(1, largest, None).is_ok());
    assert_eq!(
        SampleRequest::new(1, largest + 1, None),
        Err(LimitExceeded {
            parameter: "size",
            requested: largest + 1,
            minimum: 1,
            maximum: Some(largest),
        })
    );

    assert_eq!(ekr_views::DEFAULT_CONFIDENCE, 9_500);
    let unstated = report_fact_quality(&of(8, 2), None).unwrap();
    let stated = report_fact_quality(&of(8, 2), Some(ekr_views::DEFAULT_CONFIDENCE)).unwrap();
    assert_eq!(unstated, stated);
    let report: Value = serde_json::from_slice(&unstated.bytes).unwrap();
    assert_eq!(report["meta"]["format"], ekr_views::FACT_QUALITY_FORMAT);
    let z = ekr_views::wilson_z(9_500);
    assert_eq!(report["meta"]["z"].as_f64(), Some(z));
    let (rate, lower, upper) = ekr_views::wilson_interval(8, 10, z);
    assert_eq!(report["rate"].as_f64(), Some(rate));
    assert_eq!(report["lower"].as_f64(), Some(lower));
    assert_eq!(report["upper"].as_f64(), Some(upper));
    assert_eq!(rate, 0.8);
    let (closed_lower, closed_upper) = closed_form(8, 10, z);
    close(lower, closed_lower, "8/10 lower");
    close(upper, closed_upper, "8/10 upper");
    assert_eq!(ekr_views::wilson_interval(0, 4, z).1, 0.0, "none passed");
    assert_eq!(ekr_views::wilson_interval(4, 4, z).2, 1.0, "all passed");

    let refused: ekr_views::JudgementsMalformed =
        FactJudgements::from_json(br#"{"format":"ekr.fact-judgements/2","judgements":[]}"#)
            .expect_err("another format");
    assert!(
        refused
            .to_string()
            .starts_with(&format!("not an {JUDGEMENTS_FORMAT} document: ")),
        "{refused}"
    );
}
