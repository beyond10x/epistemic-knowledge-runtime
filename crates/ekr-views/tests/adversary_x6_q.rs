//! Adversary pass on `story:fact-quality-by-judged-sample` (`ekr.fact-sample/1`,
//! `ekr.fact-quality/1`): inputs the unit's suite does not reach, each held against a value
//! written out here, not one the code under test computed.
//!
//! * z at every basis point and at the branch seam of AS 241 (8500 / 8501), against the normal
//!   quantile computed to 60 digits (mpmath `sqrt(2)·erfinv(c / 10000)`) and rounded to binary64.
//! * The Wilson interval at n = 1, at a confidence of 1 basis point, at 9999, and at n = 10⁶,
//!   against the same 60-digit evaluation of the formula `views.yaml` states.
//! * The binary64 values the document carries survive the parse every CLI and session answer
//!   makes of it (`serde_json::from_slice::<Value>`), as `views.yaml` says ("every host answers
//!   the same binary64 values").
//! * Evidence bytes that are not UTF-8 are printed as their padded base64, and a subtype's fact
//!   names the property it inherits.
//! * The draw never includes a retracted or superseded assertion, at any revision or seed.
//! * A sample's own `meta`, cut to the four origin fields, reads back as a `SampleOrigin`.

mod support;

use std::collections::BTreeMap;
use std::path::Path;

use ekr_core::{AssertionId, ContentHash, EvidenceId, RevisionNumber, Timestamp, TypeId};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Node, Object,
    Predicate, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{Runtime, SeedDocument};
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use ekr_views::{
    draw_sample, report_fact_quality, wilson_z, FactJudgements, Judgement, SampleRequest, Verdict,
};
use serde_json::{json, Value as Json};

use support::fixtures::{self, context, id, Fixture, Provider};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

/// Φ⁻¹((1 + c/10000) / 2), computed with mpmath at 60 digits and rounded to binary64.
const REFERENCE_Z: [(u64, f64); 20] = [
    (1, 0.000_125_331_414_059_666_9),
    (2, 0.000_250_662_830_088_035_1),
    (10, 0.001_253_314_465_432_554_4),
    (100, 0.012_533_469_508_069_264),
    (1_000, 0.125_661_346_855_074_05),
    (2_500, 0.318_639_363_964_375_14),
    (5_000, 0.674_489_750_196_081_7),
    (6_827, 1.000_021_713_322_999_2),
    (8_000, 1.281_551_565_544_600_4),
    (8_499, 1.439_178_342_158_350_6),
    (8_500, 1.439_531_470_938_456),
    (8_501, 1.439_884_779_319_350_3),
    (9_000, 1.644_853_626_951_472_6),
    (9_500, 1.959_963_984_540_054_3),
    (9_545, 2.000_002_443_899_604),
    (9_900, 2.575_829_303_548_901),
    (9_973, 2.999_976_992_703_393),
    (9_990, 3.290_526_731_491_895),
    (9_998, 3.719_016_485_455_680_4),
    (9_999, 3.890_591_886_413_094),
];

#[test]
fn z_is_within_four_units_in_the_last_place_of_the_true_quantile_across_the_range() {
    let mut worst = Vec::new();
    for (confidence, reference) in REFERENCE_Z {
        let z = wilson_z(confidence);
        let ulp = f64::EPSILON * reference.abs();
        let off = (z - reference).abs() / ulp;
        if off > 4.0 {
            worst.push(format!(
                "{confidence}: {z} against {reference}, {off:.1} ulp"
            ));
        }
    }
    assert!(worst.is_empty(), "{worst:#?}");
}

#[test]
fn z_rises_at_every_basis_point_including_the_seam_between_the_two_branches() {
    let mut previous = 0.0;
    for confidence in 1..=9_999 {
        let z = wilson_z(confidence);
        assert!(
            z.is_finite() && z > previous,
            "z at {confidence} is {z}, after {previous}"
        );
        previous = z;
    }
}

/// `(passed, judged, confidence, lower, upper)`: the formula `views.yaml` states for
/// `ekr.views.FactQualityV1`, evaluated with mpmath at 60 digits and rounded to binary64.
const REFERENCE_WILSON: [(u64, u64, i64, f64, f64); 13] = [
    (1, 1, 9_500, 0.206_549_314_377_237_4, 1.0),
    (0, 1, 9_500, 0.0, 0.793_450_685_622_762_7),
    (1, 1, 9_999, 0.061_970_519_133_616_835, 1.0),
    (0, 1, 1, 0.0, 1.570_796_310_345_556e-8),
    (1, 1, 1, 0.999_999_984_292_036_8, 1.0),
    (8, 10, 1, 0.799_984_146_219_534_6, 0.800_015_852_837_987_6),
    (
        8,
        10,
        9_999,
        0.260_204_751_816_809_24,
        0.978_490_015_293_923_1,
    ),
    (1, 2, 1, 0.499_955_688_653_785_37, 0.500_044_311_346_214_6),
    (
        3,
        7,
        8_500,
        0.207_811_151_468_404_95,
        0.681_962_619_573_404_6,
    ),
    (
        3,
        7,
        8_501,
        0.207_772_226_510_930_27,
        0.682_013_903_424_470_8,
    ),
    (
        1,
        1_000_000,
        9_500,
        1.765_245_767_453_714_7e-7,
        5.664_911_804_311_443e-6,
    ),
    (
        999_999,
        1_000_000,
        9_500,
        0.999_994_335_088_195_7,
        0.999_999_823_475_423_3,
    ),
    (
        1,
        1_000_000,
        9_999,
        5.855_435_165_193_56e-8,
        1.707_789_148_564_219e-5,
    ),
];

fn of(passed: u64, judged: u64) -> FactJudgements {
    FactJudgements {
        sample: None,
        judgements: (0..judged)
            .map(|n| Judgement {
                assertion: id(0x7a_0000_0000 + n),
                verdict: if n < passed {
                    Verdict::Pass
                } else {
                    Verdict::Fail
                },
            })
            .collect(),
    }
}

fn close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() <= 1e-12 * expected.abs().max(f64::MIN_POSITIVE),
        "{what}: {actual} is not {expected}"
    );
}

#[test]
fn the_interval_at_n_1_at_the_confidence_bounds_and_at_a_million_is_the_closed_form() {
    for (passed, judged, confidence, lower, upper) in REFERENCE_WILSON {
        let what = format!("{passed}/{judged} at {confidence}");
        let answer = report_fact_quality(&of(passed, judged), Some(confidence)).expect(&what);
        let document: Json = serde_json::from_slice(&answer.bytes).unwrap();
        assert_eq!(document["judged"], judged, "{what}");
        assert_eq!(document["passed"], passed, "{what}");
        assert_eq!(document["failed"], judged - passed, "{what}");
        close(document["lower"].as_f64().unwrap(), lower, &what);
        close(document["upper"].as_f64().unwrap(), upper, &what);
        let lower_doc = document["lower"].as_f64().unwrap();
        let upper_doc = document["upper"].as_f64().unwrap();
        let rate = document["rate"].as_f64().unwrap();
        assert!(
            (0.0..=1.0).contains(&lower_doc) && lower_doc <= rate && rate <= upper_doc,
            "{what}: {document}"
        );
        assert_eq!(
            answer.summary.rate_bp,
            Some(passed * 10_000 / judged),
            "{what}"
        );
    }
}

/// The text of the JSON number following `"key":` in a compact document.
fn raw_number<'a>(text: &'a str, key: &str) -> &'a str {
    let start = text
        .find(&format!("\"{key}\":"))
        .unwrap_or_else(|| panic!("{key} in {text}"))
        + key.len()
        + 3;
    let rest = &text[start..];
    let end = rest.find([',', '}']).expect("a terminated number");
    &rest[..end]
}

#[test]
fn every_binary64_the_report_holds_survives_the_parse_each_cli_and_session_answer_makes() {
    // `ekr fact-quality` and the session lane both print `serde_json::from_slice::<Value>` of the
    // library's bytes (crates/ekr/src/cli/sample.rs `report`), so a number that parse rounds to
    // another binary64 is printed as a value the library did not compute.
    let mut drifted = Vec::new();
    for (passed, judged) in [(2_u64, 3_u64), (8, 10), (37, 63), (1, 7)] {
        let judgements = of(passed, judged);
        for confidence in 1..=9_999_i64 {
            let answer = report_fact_quality(&judgements, Some(confidence)).unwrap();
            let text = std::str::from_utf8(&answer.bytes).unwrap();
            let parsed: Json = serde_json::from_slice(&answer.bytes).unwrap();
            for (key, value) in [
                ("z", &parsed["meta"]["z"]),
                ("lower", &parsed["lower"]),
                ("upper", &parsed["upper"]),
            ] {
                let exact: f64 = raw_number(text, key).parse().unwrap();
                let through_value = value.as_f64().unwrap();
                if exact.to_bits() != through_value.to_bits() {
                    drifted.push(format!(
                        "{passed}/{judged} at {confidence}: {key} {exact:?} printed as {through_value:?}"
                    ));
                }
            }
        }
    }
    assert!(
        drifted.is_empty(),
        "{} values change on the CLI's parse, first {:#?}",
        drifted.len(),
        &drifted[..drifted.len().min(5)]
    );
}

#[test]
fn the_documented_example_prints_what_the_library_computes_after_the_clis_parse() {
    // docs/cli.md, `ekr fact-quality`: 2 of 3 at 9500 prints z 1.9599639845400538, lower
    // 0.20765960080204776 and upper 0.9385080552796037. The library computes those; the parse
    // `ekr` makes before printing, in a build without `serde_json/float_roundtrip` (every build
    // but `cargo test -p ekr`, where the `jsonschema` dev-dependency turns it on), does not.
    let answer = report_fact_quality(&of(2, 3), Some(9_500)).unwrap();
    let text = std::str::from_utf8(&answer.bytes).unwrap();
    assert_eq!(raw_number(text, "z"), "1.9599639845400538");
    assert_eq!(raw_number(text, "lower"), "0.20765960080204776");
    assert_eq!(raw_number(text, "upper"), "0.9385080552796037");
    let printed =
        serde_json::to_string(&serde_json::from_slice::<Json>(&answer.bytes).unwrap()).unwrap();
    assert_eq!(
        (
            raw_number(&printed, "z"),
            raw_number(&printed, "lower"),
            raw_number(&printed, "upper")
        ),
        (
            "1.9599639845400538",
            "0.20765960080204776",
            "0.9385080552796037"
        ),
        "{printed}"
    );
}

// ---- the draw ----------------------------------------------------------------------------------

const THING: u64 = 0x7b_0001;
const SUB_THING: u64 = 0x7b_0002;
const LABEL: u64 = 0x7b_0010;
const NODES: u64 = 0x7b_0100;
const BINARY: u64 = 0x7b_0300;
const TEXT: u64 = 0x7b_0301;
const ASSERTIONS: u64 = 0x7b_0400;

const BINARY_BYTES: [u8; 4] = [0xff, 0xfe, 0x00, 0x41];

fn empty_seed() -> SeedDocument {
    let path = Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("the manifest dir"))
        .join("tests/fixtures/seed-empty.yaml");
    SeedDocument::from_yaml(&std::fs::read_to_string(path).expect("the empty seed"))
        .expect("the empty seed parses")
}

fn evidence(document: &mut SeedDocument, n: u64, payload: &[u8]) {
    let record = Evidence {
        id: id(n),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: ContentHash::of_bytes(payload),
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(1_000),
        confidence: Confidence::from_basis_points(9_000).expect("basis points"),
    };
    document
        .evidence_payloads
        .insert(record.content_hash, payload.to_vec().into());
    document.graph.evidence.insert(record.id, record);
}

fn labelled(assertion: u64, node: u64, cited: u64) -> Assertion<Value> {
    Assertion {
        id: id::<AssertionId>(ASSERTIONS + assertion),
        root_id: id(2),
        subject: Subject::Node(id(NODES + node)),
        predicate: Predicate::Property(id(LABEL)),
        object: Object::Value(Value::String(format!("label {assertion}"))),
        evidence: [id::<EvidenceId>(cited)].into_iter().collect(),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::from_millis(500)),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}

/// `thing` declares `label`; `sub-thing` specialises it and declares nothing. Node 1 is a thing
/// whose label cites bytes that are not UTF-8; node 2 a sub-thing whose label cites text.
fn binary_and_subtype(runtime: &Runtime) {
    let mut document = empty_seed();
    let root = document.graph.root.id;
    let mut thing = NodeType::new(id(THING), "thing");
    thing.properties.insert(
        id(LABEL),
        PropertyDefinition::new(id(LABEL), "label", ValueType::String),
    );
    let mut sub_thing = NodeType::new(id(SUB_THING), "sub-thing");
    sub_thing.parents = [id::<TypeId>(THING)].into_iter().collect();
    document.ontology.node_types.push(thing);
    document.ontology.node_types.push(sub_thing);
    for (n, type_id, name) in [(1, THING, "one"), (2, SUB_THING, "two")] {
        let node = Node::<Value>::new(id(NODES + n), root, id(type_id), name);
        document.graph.nodes.insert(node.id, node);
    }
    evidence(&mut document, BINARY, &BINARY_BYTES);
    evidence(&mut document, TEXT, b"two is labelled");
    for assertion in [labelled(1, 1, BINARY), labelled(2, 2, TEXT)] {
        document.graph.assertions.insert(assertion.id, assertion);
    }
    let now = Timestamp::from_millis(fixtures::CLOCK_START_MS + 1);
    runtime
        .seed(document, || now)
        .expect("the seed is admitted");
}

fn items_by_assertion(runtime: &Runtime, type_id: Option<u64>) -> BTreeMap<String, Json> {
    let request = SampleRequest::new(3, 1_000, type_id.map(id::<TypeId>)).unwrap();
    let answer = draw_sample(runtime, None, &request).expect("drawn");
    let document: Json = serde_json::from_slice(&answer.bytes).unwrap();
    document["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item["assertion"]["id"].as_str().unwrap().to_owned(),
                item.clone(),
            )
        })
        .collect()
}

#[test]
fn evidence_bytes_that_are_not_utf8_are_printed_as_padded_base64_and_never_as_text() {
    for provider in PROVIDERS {
        let work = tempfile::tempdir().unwrap();
        let runtime = fixtures::open(work.path(), provider);
        binary_and_subtype(&runtime);
        let items = items_by_assertion(&runtime, None);
        let binary = &items[&id::<AssertionId>(ASSERTIONS + 1).to_string()];
        assert_eq!(
            binary["evidence"],
            json!([{
                "id": id::<EvidenceId>(BINARY).to_string(),
                "kind": "HumanStatement",
                "locator": "operator",
                "content_hash": ContentHash::of_bytes(&BINARY_BYTES).to_string(),
                "base64": "//4AQQ==",
            }]),
            "{}",
            provider.name()
        );
        let text = &items[&id::<AssertionId>(ASSERTIONS + 2).to_string()];
        assert_eq!(text["evidence"][0]["text"], "two is labelled");
        assert!(text["evidence"][0].get("base64").is_none(), "{text}");
    }
}

#[test]
fn a_subtypes_fact_names_the_property_it_inherits_and_the_supertype_filter_leaves_it_out() {
    let work = tempfile::tempdir().unwrap();
    let runtime = fixtures::open(work.path(), Provider::Sqlite);
    binary_and_subtype(&runtime);
    let sub = id::<AssertionId>(ASSERTIONS + 2).to_string();
    let items = items_by_assertion(&runtime, None);
    assert_eq!(
        items[&sub]["subject_type"],
        id::<TypeId>(SUB_THING).to_string()
    );
    assert_eq!(items[&sub]["subject_name"], "two");
    assert_eq!(items[&sub]["predicate_name"], "label", "{}", items[&sub]);
    // `--type` compares exactly (views.yaml): the supertype draws only its own node's fact.
    let things = items_by_assertion(&runtime, Some(THING));
    assert_eq!(
        things.keys().cloned().collect::<Vec<_>>(),
        vec![id::<AssertionId>(ASSERTIONS + 1).to_string()]
    );
    let subs = items_by_assertion(&runtime, Some(SUB_THING));
    assert_eq!(subs.keys().cloned().collect::<Vec<_>>(), vec![sub]);
}

#[test]
fn no_seed_at_any_revision_draws_a_retracted_or_superseded_assertion() {
    let work = tempfile::tempdir().unwrap();
    let runtime = fixtures::open(work.path(), Provider::File);
    Fixture::Changes.build(&runtime);
    let head = runtime.head().unwrap().unwrap().revision.get();
    for at in 0..=head {
        let read = runtime.read(Some(RevisionNumber::new(at))).unwrap();
        let active = read
            .graph
            .assertions
            .values()
            .filter(|assertion| matches!(assertion.lifecycle, AssertionLifecycle::Active))
            .count() as u64;
        let inactive = read.graph.assertions.len() as u64 - active;
        for seed in -20..20 {
            let request = SampleRequest::new(seed, 1_000, None).unwrap();
            let answer = draw_sample(&runtime, Some(RevisionNumber::new(at)), &request).unwrap();
            let document: Json = serde_json::from_slice(&answer.bytes).unwrap();
            assert_eq!(answer.summary.population, active, "revision {at}");
            for item in document["items"].as_array().unwrap() {
                assert_eq!(
                    item["assertion"]["lifecycle"]["kind"], "Active",
                    "revision {at}, seed {seed}: {item}"
                );
            }
            if seed == 0 {
                eprintln!("revision {at}: {active} active, {inactive} not");
            }
        }
    }
}

#[test]
fn a_samples_meta_cut_to_its_origin_fields_reads_back_and_is_echoed_unchanged() {
    let work = tempfile::tempdir().unwrap();
    let runtime = fixtures::open(work.path(), Provider::File);
    binary_and_subtype(&runtime);
    let request = SampleRequest::new(i64::MIN, 2, Some(id(THING))).unwrap();
    let sample: Json =
        serde_json::from_slice(&draw_sample(&runtime, None, &request).unwrap().bytes).unwrap();
    let meta = &sample["meta"];
    let origin = json!({
        "revision": meta["revision"],
        "seed": meta["seed"],
        "size": meta["size"],
        "type": meta["type"],
    });
    let judged = json!({
        "format": "ekr.fact-judgements/1",
        "sample": origin,
        "judgements": [{"assertion": sample["items"][0]["assertion"]["id"], "verdict": "Pass"}],
    });
    let read = FactJudgements::from_json(judged.to_string().as_bytes()).expect("reads back");
    let report: Json =
        serde_json::from_slice(&report_fact_quality(&read, None).unwrap().bytes).unwrap();
    assert_eq!(report["meta"]["sample"], origin);
    assert_eq!(report["meta"]["sample"]["seed"], i64::MIN);
}
