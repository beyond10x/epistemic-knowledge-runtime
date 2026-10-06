//! `story:ocel-process-map`: ProjectProcessMap and its format ekr.process-map/1, held against
//! `systems/ekr/domains/views.yaml` on the `process-map` fixture, whose log is known
//! (a-process-map-counts-the-variants-and-directly-follows-edges-of-the-log.yaml states it), on
//! both native providers:
//!
//! * at revision 0 the log's three tickets follow two variants — opened, reviewed, closed twice
//!   and opened, closed once — and the reviewer's one trace is reviewed twice; the document is
//!   exactly the one the format's rules give, and its directly-follows counts are the log's;
//! * every variant and edge count equals the one recomputed here, independently, from the
//!   `ekr.ocel/1` document `export_ocel` answers for the same request, and `meta.ocel_hash` is
//!   that document's hash, by the overview's rule, with event types named and at every revision;
//! * at revision 1 the third ticket is reviewed too, and one ticket variant remains;
//! * two reads of one request are byte-identical, on one provider and across both;
//! * a name no node type holds, an unseeded store and a revision beyond the head are refused by
//!   name, as `export_ocel` refuses them;
//! * the pure half, `process_map`, reads any `ekr.ocel/1` document — its events in any order, an
//!   event relating to one object twice — and refuses a document of another format.

mod support;

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use ekr_views::{OcelError, OcelMalformed, ProcessMapped, ProjectError};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use support::fixtures::{
    self, Fixture, Provider, PM_CLOSED, PM_OPENED, PM_REVIEWED, PM_REVIEWER, PM_TICKET,
};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

/// Per object type, its variants with their case counts and its directly-follows counts.
type Figures = BTreeMap<String, (BTreeMap<Vec<String>, u64>, BTreeMap<(String, String), u64>)>;

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn built(provider: Provider) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    Fixture::ProcessMap.build(&runtime);
    (work, runtime)
}

fn names(events: &[&str]) -> Vec<String> {
    events.iter().map(|name| (*name).to_owned()).collect()
}

/// The map of revision `at` (the head when `None`) with `events` named: its bytes, parsed, and
/// its event.
fn map(runtime: &Runtime, at: Option<u64>, events: &[&str]) -> (Vec<u8>, Value, ProcessMapped) {
    let answer =
        ekr_views::export_process_map(runtime, at.map(RevisionNumber::new), &names(events), &[])
            .expect("the revision is mapped");
    let value: Value = serde_json::from_slice(&answer.bytes).expect("JSON");
    (answer.bytes, value, answer.summary)
}

fn variant(activities: &[u64], cases: u64) -> Value {
    json!({
        "activities": activities.iter().map(|a| uuid(*a)).collect::<Vec<_>>(),
        "cases": cases,
    })
}

fn follows(from: u64, to: u64, count: u64) -> Value {
    json!({"from": uuid(from), "to": uuid(to), "count": count})
}

fn node_type_names() -> Value {
    json!([
        {"id": uuid(PM_TICKET), "name": "ticket"},
        {"id": uuid(PM_REVIEWER), "name": "reviewer"},
        {"id": uuid(PM_OPENED), "name": "opened"},
        {"id": uuid(PM_REVIEWED), "name": "reviewed"},
        {"id": uuid(PM_CLOSED), "name": "closed"},
    ])
}

/// The map revision 0 gives by the overview's rule, whose `ocel_hash` is `ocel_hash`.
fn seed_map(ocel_hash: &str) -> Value {
    let (o, v, c) = (PM_OPENED, PM_REVIEWED, PM_CLOSED);
    json!({
        "meta": {"format": "ekr.process-map/1", "revision": 0, "ocel_hash": ocel_hash},
        "names": {"node_types": node_type_names()},
        "object_types": [
            {
                "object_type": uuid(PM_TICKET),
                "objects": 3,
                "cases": 3,
                "variants": [variant(&[o, v, c], 2), variant(&[o, c], 1)],
                "directly_follows": [follows(o, v, 2), follows(o, c, 1), follows(v, c, 2)],
            },
            {
                "object_type": uuid(PM_REVIEWER),
                "objects": 1,
                "cases": 1,
                "variants": [variant(&[v, v], 1)],
                "directly_follows": [follows(v, v, 1)],
            },
        ],
    })
}

/// The variants and directly-follows counts of `ocel`, an `ekr.ocel/1` document, recomputed
/// without the crate: per object type, each object's trace is the types of the events relating
/// to it, by time and then id, and an object with no event is no case.
fn recomputed(ocel: &Value) -> Figures {
    let log = &ocel["ocel"];
    let mut events: Vec<(&str, &str, &str, BTreeSet<&str>)> = log["events"]
        .as_array()
        .expect("events")
        .iter()
        .map(|event| {
            (
                event["time"].as_str().expect("time"),
                event["id"].as_str().expect("id"),
                event["type"].as_str().expect("type"),
                event["relationships"]
                    .as_array()
                    .expect("relationships")
                    .iter()
                    .map(|related| related["objectId"].as_str().expect("objectId"))
                    .collect(),
            )
        })
        .collect();
    events.sort();
    let mut out = BTreeMap::new();
    for object_type in log["objectTypes"].as_array().expect("object types") {
        out.insert(
            object_type["name"].as_str().expect("name").to_owned(),
            (BTreeMap::new(), BTreeMap::new()),
        );
    }
    for object in log["objects"].as_array().expect("objects") {
        let id = object["id"].as_str().expect("id");
        let trace: Vec<String> = events
            .iter()
            .filter(|(_, _, _, related)| related.contains(id))
            .map(|(_, _, kind, _)| (*kind).to_owned())
            .collect();
        if trace.is_empty() {
            continue;
        }
        let (variants, edges) = out
            .get_mut(object["type"].as_str().expect("type"))
            .expect("a declared object type");
        for pair in trace.windows(2) {
            *edges.entry((pair[0].clone(), pair[1].clone())).or_insert(0) += 1;
        }
        *variants.entry(trace).or_insert(0) += 1;
    }
    out
}

/// The same figures read off a map document.
fn read_back(map: &Value) -> Figures {
    map["object_types"]
        .as_array()
        .expect("object types")
        .iter()
        .map(|entry| {
            let variants = entry["variants"]
                .as_array()
                .expect("variants")
                .iter()
                .map(|variant| {
                    (
                        variant["activities"]
                            .as_array()
                            .expect("activities")
                            .iter()
                            .map(|a| a.as_str().expect("activity").to_owned())
                            .collect(),
                        variant["cases"].as_u64().expect("cases"),
                    )
                })
                .collect();
            let edges = entry["directly_follows"]
                .as_array()
                .expect("edges")
                .iter()
                .map(|edge| {
                    (
                        (
                            edge["from"].as_str().expect("from").to_owned(),
                            edge["to"].as_str().expect("to").to_owned(),
                        ),
                        edge["count"].as_u64().expect("count"),
                    )
                })
                .collect();
            (
                entry["object_type"].as_str().expect("type").to_owned(),
                (variants, edges),
            )
        })
        .collect()
}

/// Acceptance: three cases following two variants print both variants with their counts and a
/// directly-follows graph whose edge counts match the log.
#[test]
fn three_tickets_follow_two_variants_and_each_edge_count_is_the_logs() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let ocel =
            ekr_views::export_ocel(&runtime, Some(RevisionNumber::new(0)), &[]).expect("the log");
        let (bytes, value, summary) = map(&runtime, Some(0), &[]);
        assert_eq!(
            value,
            seed_map(&ocel.summary.ocel_hash),
            "{}",
            provider.name()
        );
        assert_eq!(value["meta"]["format"], ekr_views::PROCESS_MAP_FORMAT);
        assert_eq!(
            summary,
            ProcessMapped {
                revision: 0,
                object_types: 2,
                cases: 4,
                variants: 3,
                directly_follows: 4,
                ocel_hash: ocel.summary.ocel_hash.clone(),
                process_map_hash: hex::encode(Sha256::digest(&bytes)),
            },
            "{}",
            provider.name()
        );
        let text = String::from_utf8(bytes).expect("UTF-8");
        let declared = format!(
            "{{\"meta\":{{\"format\":\"ekr.process-map/1\",\"revision\":0,\"ocel_hash\":\"{}\"}},\
             \"names\":{{\"node_types\":[{{\"id\":\"{}\",\"name\":\"ticket\"}},",
            ocel.summary.ocel_hash,
            uuid(PM_TICKET)
        );
        assert!(
            text.starts_with(&declared),
            "{}: the document is compact JSON in its declared key order: {text}",
            provider.name()
        );
        assert!(
            text.contains(&format!(
                "\"object_types\":[{{\"object_type\":\"{}\",\"objects\":3,\"cases\":3,\
                 \"variants\":[{{\"activities\":[",
                uuid(PM_TICKET)
            )),
            "{}: {text}",
            provider.name()
        );
    }
}

/// Every count is the log's: the map equals what the `ekr.ocel/1` document of the same request
/// gives when recomputed here, and names that document by its hash.
#[test]
fn every_variant_and_edge_count_is_recomputed_from_the_ocel_document_of_the_same_request() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let requests: [(Option<u64>, &[&str]); 5] = [
            (Some(0), &[]),
            (Some(1), &[]),
            (None, &[]),
            (Some(0), &["opened", "closed"]),
            (None, &["reviewed"]),
        ];
        for (at, events) in requests {
            let ocel =
                ekr_views::export_ocel(&runtime, at.map(RevisionNumber::new), &names(events))
                    .expect("the log");
            let log: Value = serde_json::from_slice(&ocel.bytes).expect("JSON");
            let (_, value, summary) = map(&runtime, at, events);
            let label = format!("{} at {at:?} events {events:?}", provider.name());
            assert_eq!(read_back(&value), recomputed(&log), "{label}");
            assert_eq!(
                value["meta"]["ocel_hash"], ocel.summary.ocel_hash,
                "{label}"
            );
            assert_eq!(summary.ocel_hash, ocel.summary.ocel_hash, "{label}");
            assert_eq!(
                value["meta"]["revision"], log["meta"]["revision"],
                "{label}"
            );
            assert_eq!(
                value["names"]["node_types"], log["names"]["node_types"],
                "{label}"
            );
            assert_eq!(
                summary.object_types,
                log["ocel"]["objectTypes"].as_array().unwrap().len() as u64,
                "{label}"
            );
        }
    }
}

/// Revision 1 reviews the third ticket, so every ticket follows one variant; revision 0 still
/// maps as it did.
#[test]
fn at_revision_one_every_ticket_follows_one_variant() {
    let (o, v, c) = (PM_OPENED, PM_REVIEWED, PM_CLOSED);
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let (head, value, summary) = map(&runtime, None, &[]);
        assert_eq!(value["meta"]["revision"], 1, "{}", provider.name());
        assert_eq!(
            value["object_types"][0],
            json!({
                "object_type": uuid(PM_TICKET),
                "objects": 3,
                "cases": 3,
                "variants": [variant(&[o, v, c], 3)],
                "directly_follows": [follows(o, v, 3), follows(v, c, 3)],
            }),
            "{}",
            provider.name()
        );
        assert_eq!(
            (summary.cases, summary.variants, summary.directly_follows),
            (4, 2, 3),
            "{}",
            provider.name()
        );
        let (_, at_one, _) = map(&runtime, Some(1), &[]);
        assert_eq!(at_one, serde_json::from_slice::<Value>(&head).unwrap());
        let ocel = ekr_views::export_ocel(&runtime, Some(RevisionNumber::new(0)), &[]).unwrap();
        assert_eq!(
            map(&runtime, Some(0), &[]).1,
            seed_map(&ocel.summary.ocel_hash)
        );
    }
}

/// Named event types replace the rule's as `export_ocel` takes them: with opened and closed
/// named, the reviewed events are objects, every ticket's trace is opened then closed, and the
/// reviewer, related to no event, is no case.
#[test]
fn named_event_types_map_the_log_those_names_give() {
    let (o, c) = (PM_OPENED, PM_CLOSED);
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let (_, value, _) = map(&runtime, Some(0), &["opened", "closed"]);
        let entries = value["object_types"].as_array().unwrap();
        let ticket = entries
            .iter()
            .find(|entry| entry["object_type"] == uuid(PM_TICKET))
            .expect("ticket is an object type");
        assert_eq!(ticket["variants"], json!([variant(&[o, c], 3)]));
        assert_eq!(ticket["directly_follows"], json!([follows(o, c, 3)]));
        let reviewer = entries
            .iter()
            .find(|entry| entry["object_type"] == uuid(PM_REVIEWER))
            .expect("reviewer is an object type");
        assert_eq!(
            reviewer,
            &json!({
                "object_type": uuid(PM_REVIEWER),
                "objects": 1,
                "cases": 0,
                "variants": [],
                "directly_follows": [],
            })
        );
        assert!(entries
            .iter()
            .any(|entry| entry["object_type"] == uuid(PM_REVIEWED)));
    }
}

#[test]
fn two_reads_of_one_request_are_byte_identical_on_one_provider_and_across_both() {
    let mut across = Vec::new();
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let first = map(&runtime, Some(0), &[]).0;
        assert_eq!(first, map(&runtime, Some(0), &[]).0, "{}", provider.name());
        across.push((first, map(&runtime, None, &["opened", "closed"]).0));
    }
    assert_eq!(across[0], across[1], "file and sqlite map one store alike");
}

#[test]
fn an_unknown_name_an_unseeded_store_and_a_revision_beyond_the_head_are_refused_by_name() {
    for provider in PROVIDERS {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        assert!(
            matches!(
                ekr_views::export_process_map(&runtime, None, &[], &[]),
                Err(OcelError::Project(ProjectError::NotSeeded {
                    requested: None
                }))
            ),
            "{}",
            provider.name()
        );
        Fixture::ProcessMap.build(&runtime);
        match ekr_views::export_process_map(&runtime, Some(RevisionNumber::new(4)), &[], &[]) {
            Err(OcelError::Project(ProjectError::RevisionNotFound { requested, head })) => {
                assert_eq!((requested.get(), head.get()), (4, 1), "{}", provider.name());
            }
            other => panic!("{}: {other:?}", provider.name()),
        }
        match ekr_views::export_process_map(&runtime, None, &names(&["opened", "lorry"]), &[]) {
            Err(OcelError::EventTypeNotFound { name, revision }) => {
                assert_eq!((name.as_str(), revision.get()), ("lorry", 1));
            }
            other => panic!("{}: {other:?}", provider.name()),
        }
        match ekr_views::export_process_map(&runtime, None, &[], &names(&["nothing.at"])) {
            Err(OcelError::EventTimeInvalid { selector, .. }) => {
                assert_eq!(selector, "nothing.at");
            }
            other => panic!("{}: {other:?}", provider.name()),
        }
    }
}

/// A named event time is the selector `export_ocel_with_event_time` takes: selecting opened's and
/// closed's `at` gives the log those two event types give.
#[test]
fn a_named_event_time_maps_the_log_its_selectors_give() {
    let (_work, runtime) = built(Provider::File);
    let selectors = names(&["opened.at", "closed.at"]);
    let ocel = ekr_views::export_ocel_with_event_time(&runtime, None, &[], &selectors).unwrap();
    let answer = ekr_views::export_process_map(&runtime, None, &[], &selectors).unwrap();
    let value: Value = serde_json::from_slice(&answer.bytes).unwrap();
    assert_eq!(value["meta"]["ocel_hash"], ocel.summary.ocel_hash);
    assert_eq!(
        read_back(&value),
        recomputed(&serde_json::from_slice(&ocel.bytes).unwrap())
    );
    assert_eq!(
        value["object_types"][0]["variants"],
        json!([variant(&[PM_OPENED, PM_CLOSED], 3)])
    );
}

/// The pure half over a document written by hand: events listed out of time order are read in
/// time order, then by id; an event relating to one object under two qualifiers is one step of
/// its trace; an object type no event relates to is listed with no case; and variants of equal
/// count are ordered by their activities.
#[test]
fn the_pure_half_maps_any_ekr_ocel_1_document() {
    let event = |id: &str, kind: &str, time: &str, objects: &[(&str, &str)]| {
        json!({
            "id": id, "type": kind, "time": time, "attributes": [],
            "relationships": objects
                .iter()
                .map(|(object, qualifier)| json!({"objectId": object, "qualifier": qualifier}))
                .collect::<Vec<_>>(),
        })
    };
    let object = |id: &str, kind: &str| json!({"id": id, "type": kind, "attributes": [], "relationships": []});
    let document = json!({
        "meta": {"format": "ekr.ocel/1", "revision": 7},
        "names": {
            "node_types": [{"id": "a", "name": "first"}, {"id": "b", "name": "second"}],
            "edge_types": [],
            "properties": [],
        },
        "ocel": {
            "eventTypes": [{"name": "a", "attributes": []}, {"name": "b", "attributes": []}],
            "objectTypes": [{"name": "idle", "attributes": []}, {"name": "k", "attributes": []}],
            "events": [
                event("e3", "a", "2027-01-01T12:00:00.000Z", &[("k2", "q")]),
                event("e1", "a", "2027-01-01T10:00:00.000Z", &[("k1", "q"), ("k1", "r")]),
                event("e2", "b", "2027-01-01T11:00:00.000Z", &[("k1", "q"), ("k2", "q")]),
            ],
            "objects": [object("k1", "k"), object("k2", "k"), object("i1", "idle")],
        },
    });
    let bytes = serde_json::to_vec(&document).unwrap();
    let answer = ekr_views::process_map(&bytes).expect("an ekr.ocel/1 document maps");
    let value: Value = serde_json::from_slice(&answer.bytes).unwrap();
    assert_eq!(
        value,
        json!({
            "meta": {
                "format": "ekr.process-map/1",
                "revision": 7,
                "ocel_hash": hex::encode(Sha256::digest(&bytes)),
            },
            "names": {"node_types": document["names"]["node_types"]},
            "object_types": [
                {"object_type": "idle", "objects": 1, "cases": 0, "variants": [], "directly_follows": []},
                {
                    "object_type": "k",
                    "objects": 2,
                    "cases": 2,
                    "variants": [
                        {"activities": ["a", "b"], "cases": 1},
                        {"activities": ["b", "a"], "cases": 1},
                    ],
                    "directly_follows": [
                        {"from": "a", "to": "b", "count": 1},
                        {"from": "b", "to": "a", "count": 1},
                    ],
                },
            ],
        })
    );
    assert_eq!(
        (
            answer.summary.revision,
            answer.summary.object_types,
            answer.summary.cases,
            answer.summary.variants,
            answer.summary.directly_follows,
        ),
        (7, 2, 2, 2, 2)
    );
}

#[test]
fn a_document_of_another_format_is_refused() {
    for bytes in [
        &b"not json"[..],
        br#"{"meta": {"format": "ekr.graph-projection/1", "revision": 0}}"#,
        br#"{"meta": {"format": "ekr.ocel/1", "revision": 0}, "names": {"node_types": []}}"#,
    ] {
        let refused: OcelMalformed =
            ekr_views::process_map(bytes).expect_err("not an ekr.ocel/1 document");
        assert!(
            refused.to_string().contains("ekr.ocel/1"),
            "{}: {refused}",
            String::from_utf8_lossy(bytes)
        );
    }
}
