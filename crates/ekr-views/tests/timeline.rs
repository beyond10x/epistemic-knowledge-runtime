//! `task:timeline-rows-are-subjects`: `ekr.graph-timeline/1` ([`Index::timeline`]) on the
//! `subjects` fixture, read row by row and cell by cell against `views.yaml`'s walk, and the bound
//! on the overview's timeline buckets.
//!
//! The `subjects` store is described in
//! `tests/fixtures/conformance/scenarios/a-timeline-row-is-a-subject-with-its-events-within-hops.yaml`;
//! the conformance suite holds its counts, and this file the exact rows, cells, paths and orders.

mod support;

use std::collections::BTreeMap;

use ekr_core::{NodeId, Timestamp, TypeId};
use ekr_graph::{Node, Object, Subject};
use ekr_views::{BucketWidth, Index, OverviewRequest, TimelineRequest};
use serde_json::{json, Value};

use support::fixtures::{
    self, id, Fixture, Provider, HAPPENING, HAPPENINGS, HOLDER, HOLDERS, NOTICE, PLACE, PLACES,
    TOUCHES,
};

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

/// 2027-01-01T00:00:00Z.
const JANUARY_FIRST: i64 = 1_798_761_600_000;
const DAY: i64 = 86_400_000;
const WEEK: i64 = 7 * DAY;

/// The start of January `day`, 2027.
fn january(day: i64) -> i64 {
    JANUARY_FIRST + (day - 1) * DAY
}

fn built(provider: Provider) -> (tempfile::TempDir, Index) {
    let work = tempfile::tempdir().unwrap();
    let runtime = fixtures::open(work.path(), provider);
    Fixture::Subjects.build(&runtime);
    let index = Index::load(&runtime, None).unwrap();
    (work, index)
}

fn timeline(
    index: &Index,
    row_type: Option<u64>,
    hops: i64,
    limit: i64,
    bucket: Option<BucketWidth>,
    subject: Option<u64>,
) -> Value {
    let request = TimelineRequest::new(
        row_type.map(id::<TypeId>),
        hops,
        limit,
        bucket,
        subject.map(id::<NodeId>),
    )
    .unwrap();
    serde_json::from_slice(&index.timeline(&request).unwrap().bytes).unwrap()
}

/// Each row as `(id, total, [(start, type, events)])`.
/// A cell: its start, its event type and how many events it holds.
type Cell = (i64, String, u64);
/// A row: its id, its total and its cells.
type Row = (String, u64, Vec<Cell>);
/// A listed event: its id, its distance and its path as (edge, forward, node).
type Listed = (String, u64, Vec<(String, bool, String)>);

fn rows(document: &Value) -> Vec<Row> {
    document["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            (
                row["id"].as_str().unwrap().to_owned(),
                row["total"].as_u64().unwrap(),
                cells(&row["cells"]),
            )
        })
        .collect()
}

fn cells(list: &Value) -> Vec<Cell> {
    list.as_array()
        .unwrap()
        .iter()
        .map(|cell| {
            (
                cell["start"].as_i64().unwrap(),
                cell["type"].as_str().unwrap().to_owned(),
                cell["events"].as_u64().unwrap(),
            )
        })
        .collect()
}

fn happening(start: i64) -> Cell {
    (start, uuid(HAPPENING), 1)
}

fn notice(start: i64) -> Cell {
    (start, uuid(NOTICE), 1)
}

#[test]
fn each_holder_row_holds_the_events_the_walk_reaches_within_one_two_and_three_hops() {
    let (_work, index) = built(Provider::File);
    let (h1, h2, h3) = (uuid(HOLDERS + 1), uuid(HOLDERS + 2), uuid(HOLDERS + 3));

    let one = timeline(&index, Some(HOLDER), 1, 500, None, None);
    assert_eq!(
        rows(&one),
        vec![
            (
                h1.clone(),
                2,
                vec![happening(january(1)), happening(january(2))]
            ),
            (h2.clone(), 1, vec![notice(january(9))]),
            (h3.clone(), 1, vec![happening(january(1))]),
        ],
        "at 1 hop H2 and H3 tie at one event and are listed by id"
    );

    let two = timeline(&index, Some(HOLDER), 2, 500, None, None);
    assert_eq!(
        rows(&two),
        vec![
            (
                h1.clone(),
                4,
                vec![
                    happening(january(1)),
                    happening(january(2)),
                    happening(january(3)),
                    happening(january(6)),
                ]
            ),
            (
                h3.clone(),
                2,
                vec![happening(january(1)), happening(january(6))]
            ),
            (h2.clone(), 1, vec![notice(january(9))]),
        ],
        "E3 points at L1, which only H1 reaches; E4 is L1's own; E5 sits at L2, which two holders share"
    );
    assert_eq!(
        cells(&two["strip"]),
        vec![
            happening(january(1)),
            happening(january(2)),
            happening(january(3)),
            happening(january(6)),
            notice(january(9)),
        ],
        "the strip counts each event once, E1 and E6 though two rows reach them"
    );
    assert_eq!(two["meta"]["events"], 5);
    assert_eq!(two["meta"]["first"], json!(january(1) + 10 * 3_600_000));
    assert_eq!(two["meta"]["last"], json!(january(60) + 12 * 3_600_000));

    let three = timeline(&index, Some(HOLDER), 3, 500, None, None);
    let totals: Vec<(String, u64)> = rows(&three)
        .into_iter()
        .map(|(row, total, _)| (row, total))
        .collect();
    assert_eq!(totals, vec![(h1, 5), (h3, 3), (h2, 1)]);
}

#[test]
fn the_rows_are_cut_at_the_limit_and_weeks_start_on_monday() {
    let (_work, index) = built(Provider::File);
    let limited = timeline(&index, Some(HOLDER), 1, 2, None, None);
    let ids: Vec<String> = rows(&limited).into_iter().map(|row| row.0).collect();
    assert_eq!(ids, vec![uuid(HOLDERS + 1), uuid(HOLDERS + 2)]);
    assert_eq!(limited["meta"]["active"], 3);

    let weekly = timeline(&index, Some(HOLDER), 2, 500, Some(BucketWidth::Week), None);
    assert_eq!(weekly["meta"]["bucket_ms"], json!(WEEK));
    // 2027-01-01 is a Friday: its week starts on Monday 2026-12-28.
    let (december_28, january_4) = (january(1) - 4 * DAY, january(4));
    assert_eq!(
        rows(&weekly)[0].2,
        vec![
            (december_28, uuid(HAPPENING), 3),
            (january_4, uuid(HAPPENING), 1)
        ]
    );
}

#[test]
fn row_types_are_the_non_event_types_ranked_by_weight_per_node() {
    let (_work, index) = built(Provider::File);
    let document = timeline(&index, None, 2, 500, None, None);
    assert_eq!(
        document["row_types"],
        json!([
            {"type": uuid(HOLDER), "nodes": 3, "reached": 3, "weight": 11},
            {"type": uuid(PLACE), "nodes": 2, "reached": 2, "weight": 6},
        ])
    );
    assert_eq!(document["meta"]["row_type"], json!(uuid(HOLDER)));

    let events = timeline(&index, Some(HAPPENING), 2, 500, None, None);
    let totals: Vec<(String, u64)> = rows(&events)
        .into_iter()
        .map(|(row, total, _)| (row, total))
        .collect();
    assert_eq!(
        totals,
        vec![(uuid(HAPPENINGS + 1), 2), (uuid(HAPPENINGS + 6), 2)],
        "from an event, the walk does not go on through a holder or a place"
    );
    assert_eq!(events["meta"]["subjects"], 7);

    let none = timeline(&index, Some(TOUCHES), 2, 500, None, None);
    assert_eq!(none["rows"], json!([]));
    assert_eq!(none["meta"]["subjects"], 0);
}

#[test]
fn a_named_subject_answers_its_row_alone_with_its_events_and_their_paths() {
    let (_work, index) = built(Provider::File);
    let document = timeline(&index, Some(PLACE), 2, 1, None, Some(HOLDERS + 1));
    assert_eq!(document["meta"]["row_type"], json!(uuid(HOLDER)));
    assert_eq!(document["meta"]["subject"], json!(uuid(HOLDERS + 1)));
    assert_eq!(rows(&document).len(), 1);
    let listed: Vec<Listed> = document["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|event| {
            (
                event["id"].as_str().unwrap().to_owned(),
                event["distance"].as_u64().unwrap(),
                event["path"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|step| {
                        (
                            step["edge"].as_str().unwrap().to_owned(),
                            step["forward"].as_bool().unwrap(),
                            step["node"].as_str().unwrap().to_owned(),
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    let edge = |n: u64| uuid(0xa0_0500 + n);
    let (e, l) = (HAPPENINGS, PLACES);
    assert_eq!(
        listed,
        vec![
            (uuid(e + 1), 1, vec![(edge(1), true, uuid(e + 1))]),
            (uuid(e + 2), 1, vec![(edge(2), false, uuid(e + 2))]),
            (
                uuid(e + 3),
                2,
                vec![(edge(4), true, uuid(l + 1)), (edge(5), false, uuid(e + 3))]
            ),
            (
                uuid(e + 6),
                2,
                vec![(edge(1), true, uuid(e + 1)), (edge(10), true, uuid(e + 6))]
            ),
        ]
    );

    let unknown = timeline(&index, None, 2, 10, None, Some(0xa0_9999));
    assert_eq!(unknown["rows"], json!([]));
    assert_eq!(unknown["events"], json!([]));
    assert_eq!(unknown["meta"]["active"], 3);
}

#[test]
fn a_request_keeps_its_inputs_and_the_document_names_its_format() {
    let request = TimelineRequest::new(
        Some(id(HOLDER)),
        TimelineRequest::MAX_HOPS,
        TimelineRequest::MAX_LIMIT,
        Some(BucketWidth::Week),
        Some(id(HOLDERS + 1)),
    )
    .unwrap();
    assert_eq!(request.row_type(), Some(id(HOLDER)));
    assert_eq!((request.hops(), request.limit()), (3, 500));
    assert_eq!(request.bucket(), Some(BucketWidth::Week));
    assert_eq!(request.subject(), Some(id(HOLDERS + 1)));
    let refused =
        TimelineRequest::new(None, TimelineRequest::MAX_HOPS + 1, 1, None, None).unwrap_err();
    assert_eq!((refused.parameter, refused.maximum), ("hops", Some(3)));
    let (_work, index) = built(Provider::File);
    let answer = index.timeline(&request).unwrap();
    let document: Value = serde_json::from_slice(&answer.bytes).unwrap();
    assert_eq!(document["meta"]["format"], ekr_views::TIMELINE_FORMAT);
    assert_eq!(answer.summary.first_row, Some(id(HOLDERS + 1)));
    assert_eq!(answer.summary.listed_events, 5);
}

#[test]
fn both_providers_answer_the_same_bytes_every_time() {
    let (_file_work, file) = built(Provider::File);
    let (_sqlite_work, sqlite) = built(Provider::Sqlite);
    for hops in 1..=3 {
        for bucket in [None, Some(BucketWidth::Day), Some(BucketWidth::Week)] {
            for subject in [None, Some(HOLDERS + 1)] {
                let request =
                    TimelineRequest::new(None, hops, 500, bucket, subject.map(id::<NodeId>))
                        .unwrap();
                let first = file.timeline(&request).unwrap();
                assert_eq!(first, file.timeline(&request).unwrap());
                assert_eq!(first.bytes, sqlite.timeline(&request).unwrap().bytes);
            }
        }
    }
}

// ---- the overview's size ------------------------------------------------------------------------

/// The page's first load is the overview alone, and the story's acceptance holds it under 300 KB
/// (`index_time.rs` names the same cap as the approved plan's). `limit` bounds only `top`: the
/// timeline had one bucket per week per node type once the dated facts span more than 120 days,
/// so a store of ten node types with a fact a week for ten years answered an overview far past
/// the cap at the default request — nothing the request can lower. Copied from the adversary's
/// `adversary_query_pass1.rs` (finding F4); the bucket width now coarsens past 200 buckets.
#[test]
fn the_default_overview_of_ten_types_dated_weekly_for_ten_years_stays_under_300_kb() {
    use ekr_core::{AssertionId, ContentHash, EvidenceId, PropertyId};
    use ekr_graph::{
        Assertion, AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Predicate,
        TemporalRange, TransactionTime,
    };
    use ekr_ontology::{NodeType, PropertyDefinition, Value as Stated, ValueType};

    const TYPES: u64 = 10;
    const WEEKS: i64 = 520;
    let mut document = ekr_kernel::SeedDocument::from_yaml(
        &std::fs::read_to_string(
            std::path::Path::new(
                &std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest dir"),
            )
            .join("tests/fixtures/seed-empty.yaml"),
        )
        .unwrap(),
    )
    .unwrap();
    let root = document.graph.root.id;
    let bytes = b"a weekly statement".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let evidence: EvidenceId = id(0xee_0001);
    document.graph.evidence.insert(
        evidence,
        Evidence {
            id: evidence,
            source: EvidenceSource::HumanStatement {
                identity: Some("operator".into()),
            },
            content_hash: hash,
            extracted_by: fixtures::context().operator,
            observed_at: Timestamp::from_millis(1_000),
            confidence: Confidence::from_basis_points(9_000).unwrap(),
        },
    );
    document.evidence_payloads.insert(hash, bytes);
    for t in 0..TYPES {
        let type_id: TypeId = id(0xee_1000 + t);
        let property: PropertyId = id(0xee_2000 + t);
        let mut declared = NodeType::new(type_id, format!("kind-{t}"));
        declared.properties.insert(
            property,
            PropertyDefinition::new(property, "reading", ValueType::String),
        );
        document.ontology.node_types.push(declared);
        let node = Node::<Stated>::new(id(0xee_3000 + t), root, type_id, format!("subject-{t}"));
        let node_id = node.id;
        document.graph.nodes.insert(node_id, node);
        for week in 0..WEEKS {
            let assertion = Assertion {
                id: id::<AssertionId>(0xee_0000_0000 + t * 0x1_0000 + week as u64),
                root_id: root,
                subject: Subject::Node(node_id),
                predicate: Predicate::Property(property),
                object: Object::Value(Stated::String(format!("week {week}"))),
                evidence: [evidence].into_iter().collect(),
                proposed_by: fixtures::context().operator,
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time: TemporalRange::since(Timestamp::from_millis(
                    1_500_000_000_000 + week * WEEK,
                )),
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            };
            document.graph.assertions.insert(assertion.id, assertion);
        }
    }
    let work = tempfile::tempdir().unwrap();
    let runtime = fixtures::open(work.path(), Provider::File);
    runtime
        .seed(document, || Timestamp::from_millis(1_800_000_000_001))
        .expect("the weekly seed is admitted");
    let index = Index::load(&runtime, None).unwrap();
    let overview = index
        .overview(&OverviewRequest::new(None).unwrap())
        .unwrap();
    let smallest = index
        .overview(&OverviewRequest::new(Some(1)).unwrap())
        .unwrap();
    assert!(
        overview.bytes.len() < 300 * 1024,
        "{TYPES} types and {} assertions: the default overview is {} bytes, {} timeline buckets \
         (limit 1: {} bytes)",
        TYPES * WEEKS as u64,
        overview.bytes.len(),
        overview.summary.timeline_buckets,
        smallest.bytes.len()
    );
    // 520 weeks are more than 200 buckets, so the width is the next one, 4 weeks: 130 columns.
    assert_eq!(overview.summary.bucket_ms, 4 * WEEK as u64);
    let columns: BTreeMap<i64, u64> = serde_json::from_slice::<Value>(&overview.bytes).unwrap()
        ["timeline"]["buckets"]
        .as_array()
        .unwrap()
        .iter()
        .fold(BTreeMap::new(), |mut columns, bucket| {
            *columns
                .entry(bucket["start"].as_i64().unwrap())
                .or_default() += 1;
            columns
        });
    assert!(columns.len() <= 200, "{} columns", columns.len());
    assert!(columns
        .keys()
        .all(|start| (start + 3 * DAY).rem_euclid(WEEK) == 0));
}
