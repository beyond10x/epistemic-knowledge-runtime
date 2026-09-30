//! `task:store-exports-ocel-event-log`: ExportOcel and its format ekr.ocel/1, held against
//! `systems/ekr/domains/views.yaml` on the `ocel` fixture, whose log per revision and request is
//! known (an-ocel-export-takes-events-from-valid-time-and-objects-from-the-rest.yaml states it),
//! on both native providers:
//!
//! * with no event type named, the event types are the overview's (`ekr.views.TypeTiming`
//!   `event`), and the head's `ocel` member is exactly the log the format's rules give, its
//!   bytes pinned;
//! * naming event types replaces the rule's: naming the rule's own gives the same bytes, naming
//!   another gives another log, whose undated events are left out and counted; a name no node
//!   type holds is refused by name;
//! * `names` gives every type and property id the log uses its name;
//! * an OCEL 2.0 reader — the `process_mining` crate (rust4pm) — reads the `ocel` member back
//!   with the fixture's event types, object types, events, objects and relationships, and every
//!   value the OCEL 2.0 JSON schema requires to be a string is one;
//! * two reads of one request are byte-identical, on one provider and across both, before and
//!   after a later commit;
//! * an unseeded store and a revision beyond the head are refused by name.

mod support;

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use ekr_views::{Index, OcelError, OcelExported, ProjectError};
use process_mining::core::event_data::object_centric::ocel_json::import_ocel_json_slice;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use support::fixtures::{
    self, Fixture, Provider, JANUARY_FIRST_0900_MS, O_ABOUT, O_AT, O_CASE, O_COST, O_DEPTH,
    O_FOLLOWS, O_HOLDS, O_NODES, O_NOTE, O_OUTCOME, O_SIZE, O_STEP, O_SUBCASE, O_TAGS, O_TEXT,
    O_TITLE, O_TOUCHES, O_UNUSED,
};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

/// The SHA-256 of the head's `ekr.ocel/1` document with no event type named, pinned once its log
/// equalled [`log`]: its bytes do not change unless the format does.
const HEAD_HASH: &str = "89036f017b1b6dfe5e5688841d231ab7615b9d8128fa99fc660fb77afdc17ac5";

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn node(n: u64) -> String {
    uuid(O_NODES + n)
}

fn built(provider: Provider) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    Fixture::Ocel.build(&runtime);
    (work, runtime)
}

fn names(events: &[&str]) -> Vec<String> {
    events.iter().map(|name| (*name).to_owned()).collect()
}

/// The export of revision `at` (the head when `None`) with `events` named: its bytes, parsed,
/// and its event.
fn read_with(
    runtime: &Runtime,
    at: Option<u64>,
    events: &[&str],
) -> (Vec<u8>, Value, OcelExported) {
    let answer = ekr_views::export_ocel(runtime, at.map(RevisionNumber::new), &names(events))
        .expect("the revision is exported");
    let value: Value = serde_json::from_slice(&answer.bytes).expect("JSON");
    (answer.bytes, value, answer.summary)
}

/// The export of revision `at` by the overview's rule.
fn read(runtime: &Runtime, at: Option<u64>) -> (Vec<u8>, Value, OcelExported) {
    read_with(runtime, at, &[])
}

/// The `ocel` member's own bytes, as a consumer writes the OCEL file.
fn ocel_file(value: &Value) -> Vec<u8> {
    serde_json::to_vec(&value["ocel"]).expect("the log re-encodes")
}

fn attribute(property: u64, value_type: &str) -> Value {
    json!({"name": uuid(property), "type": value_type})
}

fn relationship(object: u64, qualifier: u64) -> Value {
    json!({"objectId": node(object), "qualifier": uuid(qualifier)})
}

fn value(property: u64, value: &str) -> Value {
    json!({"name": uuid(property), "value": value})
}

fn initial(property: u64, value: &str) -> Value {
    json!({"name": uuid(property), "value": value, "time": "1970-01-01T00:00:00.000Z"})
}

fn case_attributes() -> Vec<Value> {
    vec![
        attribute(O_TITLE, "string"),
        attribute(O_SIZE, "integer"),
        attribute(O_TAGS, "string"),
    ]
}

fn subcase_attributes() -> Vec<Value> {
    let mut subcase = case_attributes();
    subcase.push(attribute(O_DEPTH, "integer"));
    subcase
}

fn step_attributes() -> Vec<Value> {
    vec![
        attribute(O_AT, "time"),
        attribute(O_OUTCOME, "string"),
        attribute(O_COST, "float"),
    ]
}

fn c1_attributes() -> Vec<Value> {
    vec![
        initial(O_TITLE, "first case"),
        initial(O_SIZE, "3"),
        initial(
            O_TAGS,
            r#"[{"kind":"String","value":"red"},{"kind":"String","value":"blue"}]"#,
        ),
    ]
}

const P1_AT: &str = "2027-01-01T11:00:00.000Z";
const P2_AT: &str = "2027-01-01T10:00:00.000Z";

/// The rule's log: `step` the one event type, p3 at `p3_time`, before or after p2.
fn log(p3_time: &str) -> Value {
    let p2 = json!({
        "id": node(0x12), "type": uuid(O_STEP), "time": P2_AT,
        "attributes": [value(O_AT, P2_AT), value(O_OUTCOME, "failed")],
        "relationships": [relationship(2, O_ABOUT)],
    });
    let p3 = json!({
        "id": node(0x13), "type": uuid(O_STEP), "time": p3_time,
        "attributes": [],
        "relationships": [relationship(1, O_TOUCHES), relationship(3, O_ABOUT)],
    });
    let p1 = json!({
        "id": node(0x11), "type": uuid(O_STEP), "time": P1_AT,
        "attributes": [value(O_AT, P1_AT), value(O_OUTCOME, "done"), value(O_COST, "12.50")],
        "relationships": [relationship(1, O_ABOUT)],
    });
    let events = if p3_time < P2_AT {
        [p3, p2, p1]
    } else {
        [p2, p3, p1]
    };
    json!({
        "eventTypes": [{"name": uuid(O_STEP), "attributes": step_attributes()}],
        "objectTypes": [
            {"name": uuid(O_CASE), "attributes": case_attributes()},
            {"name": uuid(O_SUBCASE), "attributes": subcase_attributes()},
            {"name": uuid(O_NOTE), "attributes": [attribute(O_TEXT, "string")]},
            {"name": uuid(O_UNUSED), "attributes": []},
        ],
        "events": events,
        "objects": [
            {
                "id": node(1), "type": uuid(O_CASE),
                "attributes": c1_attributes(),
                "relationships": [relationship(0x21, O_HOLDS), relationship(0x22, O_HOLDS)],
            },
            {
                "id": node(2), "type": uuid(O_CASE),
                "attributes": [initial(O_TITLE, "second case")],
                "relationships": [],
            },
            {
                "id": node(3), "type": uuid(O_SUBCASE),
                "attributes": [initial(O_TITLE, "a sub case"), initial(O_DEPTH, "2")],
                "relationships": [],
            },
            {
                "id": node(0x21), "type": uuid(O_NOTE),
                "attributes": [initial(O_TEXT, "seen")],
                "relationships": [],
            },
            {"id": node(0x22), "type": uuid(O_NOTE), "attributes": [], "relationships": []},
        ],
    })
}

/// The log with `note` named as the event type, at revisions 0 to 2: n1 at T0 + 3 h, n2 with no
/// time and so left out, and every step an object.
fn notes_log() -> Value {
    json!({
        "eventTypes": [{"name": uuid(O_NOTE), "attributes": [attribute(O_TEXT, "string")]}],
        "objectTypes": [
            {"name": uuid(O_CASE), "attributes": case_attributes()},
            {"name": uuid(O_SUBCASE), "attributes": subcase_attributes()},
            {"name": uuid(O_STEP), "attributes": step_attributes()},
            {"name": uuid(O_UNUSED), "attributes": []},
        ],
        "events": [
            {
                "id": node(0x21), "type": uuid(O_NOTE), "time": "2027-01-01T12:00:00.000Z",
                "attributes": [value(O_TEXT, "seen")],
                "relationships": [relationship(1, O_HOLDS)],
            },
        ],
        "objects": [
            {
                "id": node(1), "type": uuid(O_CASE),
                "attributes": c1_attributes(),
                "relationships": [relationship(0x13, O_TOUCHES)],
            },
            {
                "id": node(2), "type": uuid(O_CASE),
                "attributes": [initial(O_TITLE, "second case")],
                "relationships": [],
            },
            {
                "id": node(3), "type": uuid(O_SUBCASE),
                "attributes": [initial(O_TITLE, "a sub case"), initial(O_DEPTH, "2")],
                "relationships": [],
            },
            {
                "id": node(0x11), "type": uuid(O_STEP),
                "attributes": [
                    initial(O_AT, P1_AT),
                    initial(O_OUTCOME, "done"),
                    initial(O_COST, "12.50"),
                ],
                "relationships": [relationship(1, O_ABOUT), relationship(0x12, O_FOLLOWS)],
            },
            {
                "id": node(0x12), "type": uuid(O_STEP),
                "attributes": [initial(O_AT, P2_AT), initial(O_OUTCOME, "failed")],
                "relationships": [relationship(2, O_ABOUT)],
            },
            {
                "id": node(0x13), "type": uuid(O_STEP),
                "attributes": [],
                "relationships": [relationship(3, O_ABOUT)],
            },
        ],
    })
}

/// Every type and property id of the fixture, with its name.
fn fixture_names() -> Value {
    let named = |n: u64, name: &str| json!({"id": uuid(n), "name": name});
    json!({
        "node_types": [
            named(O_CASE, "case"),
            named(O_SUBCASE, "subcase"),
            named(O_STEP, "step"),
            named(O_NOTE, "note"),
            named(O_UNUSED, "unused"),
        ],
        "edge_types": [
            named(O_ABOUT, "about"),
            named(fixtures::O_TOUCHES, "touches"),
            named(O_HOLDS, "holds"),
            named(O_FOLLOWS, "follows"),
        ],
        "properties": [
            named(O_TITLE, "title"),
            named(O_SIZE, "size"),
            named(O_TAGS, "tags"),
            named(O_AT, "at"),
            named(O_DEPTH, "depth"),
            named(O_OUTCOME, "outcome"),
            named(O_COST, "cost"),
            named(O_TEXT, "text"),
        ],
    })
}

fn rule_summary(revision: u64, hash: &[u8]) -> OcelExported {
    OcelExported {
        revision,
        event_types: 1,
        object_types: 4,
        events: 3,
        objects: 5,
        event_object_relationships: 4,
        object_object_relationships: 2,
        edges_between_events: 1,
        undated_events: 0,
        edges_of_undated_events: 0,
        parallel_edges_merged: 1,
        attribute_values_out_of_range: 0,
        ocel_hash: hex(hash),
    }
}

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn by_the_overviews_rule_each_revision_exports_the_fixtures_log_and_the_heads_bytes_are_pinned() {
    for provider in PROVIDERS {
        let name = provider.name();
        let (_work, runtime) = built(provider);
        let (bytes, document, event) = read(&runtime, None);
        assert_eq!(
            document["meta"],
            json!({"format": "ekr.ocel/1", "revision": 2}),
            "{name}"
        );
        assert_eq!(document["names"], fixture_names(), "{name}");
        // p3's dated fact of revision 1 counts at revision 2 too, retracted: the rule reads a
        // dated fact of any lifecycle.
        assert_eq!(document["ocel"], log("2026-12-31T09:00:00.000Z"), "{name}");
        assert_eq!(event, rule_summary(2, &bytes), "{name}");
        assert_eq!(hex(&bytes), HEAD_HASH, "{name}");

        let (bytes, document, event) = read(&runtime, Some(0));
        assert_eq!(document["meta"]["revision"], 0);
        assert_eq!(document["ocel"], log("2027-01-01T10:00:00.000Z"), "{name}");
        assert_eq!(event, rule_summary(0, &bytes), "{name}");

        let (bytes, document, event) = read(&runtime, Some(1));
        assert_eq!(document["ocel"], log("2026-12-31T09:00:00.000Z"), "{name}");
        assert_eq!(event, rule_summary(1, &bytes), "{name}");
    }
}

#[test]
fn the_event_types_are_exactly_those_the_overview_marks() {
    let (_work, runtime) = built(Provider::File);
    for at in [0, 1, 2] {
        let index = Index::load(&runtime, Some(RevisionNumber::new(at))).expect("an index");
        let overview: Value = serde_json::from_slice(
            &index
                .overview(&ekr_views::OverviewRequest::new(None).unwrap())
                .unwrap()
                .bytes,
        )
        .expect("JSON");
        let marked: Vec<String> = overview["roles"]["types"]
            .as_array()
            .expect("the roles")
            .iter()
            .filter(|timing| timing["event"] == true)
            .map(|timing| timing["type"].as_str().expect("a type").to_owned())
            .collect();
        let (_, document, _) = read(&runtime, Some(at));
        let exported: Vec<String> = document["ocel"]["eventTypes"]
            .as_array()
            .expect("event types")
            .iter()
            .map(|declared| declared["name"].as_str().expect("a name").to_owned())
            .collect();
        assert_eq!(exported, marked, "revision {at}");
        assert_eq!(exported, [uuid(O_STEP)], "revision {at}");
    }
}

#[test]
fn named_event_types_replace_the_rules_and_leave_undated_events_out() {
    for provider in PROVIDERS {
        let name = provider.name();
        let (_work, runtime) = built(provider);
        // Naming the rule's own event type answers the rule's bytes.
        assert_eq!(
            read_with(&runtime, None, &["step"]).0,
            read(&runtime, None).0,
            "{name}"
        );

        let (bytes, document, event) = read_with(&runtime, None, &["note"]);
        assert_eq!(document["names"], fixture_names(), "{name}");
        assert_eq!(document["ocel"], notes_log(), "{name}");
        assert_eq!(
            event,
            OcelExported {
                revision: 2,
                event_types: 1,
                object_types: 4,
                events: 1,
                objects: 6,
                event_object_relationships: 1,
                object_object_relationships: 5,
                edges_between_events: 0,
                undated_events: 1,
                edges_of_undated_events: 1,
                parallel_edges_merged: 1,
                attribute_values_out_of_range: 0,
                ocel_hash: hex(&bytes),
            },
            "{name}"
        );

        // Both named: step and note are event types, a name given twice counts once.
        let (_, document, event) = read_with(&runtime, Some(0), &["note", "step", "note"]);
        assert_eq!(
            (
                event.event_types,
                event.object_types,
                event.events,
                event.undated_events
            ),
            (2, 3, 4, 1),
            "{name}"
        );
        assert_eq!(
            document["ocel"]["eventTypes"][0]["name"],
            uuid(O_STEP),
            "{name}"
        );
    }
}

#[test]
fn a_name_no_node_type_holds_is_refused_naming_the_first_such_name() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        match ekr_views::export_ocel(
            &runtime,
            Some(RevisionNumber::new(1)),
            &names(&["step", "Step", "missing"]),
        ) {
            Err(OcelError::EventTypeNotFound { name, revision }) => {
                assert_eq!(
                    (name.as_str(), revision.get()),
                    ("Step", 1),
                    "{}",
                    provider.name()
                );
            }
            other => panic!("{}: {other:?}", provider.name()),
        }
    }
}

#[test]
fn an_ocel_2_0_reader_reads_the_log_back_with_its_events_objects_and_relationships() {
    let (_work, runtime) = built(Provider::File);
    let (_, document, _) = read(&runtime, Some(0));
    let file = ocel_file(&document);
    let log = import_ocel_json_slice(&file).expect("process_mining reads the OCEL 2.0 JSON log");
    assert_eq!(log.event_types.len(), 1);
    assert_eq!(log.object_types.len(), 4);
    assert_eq!(log.events.len(), 3);
    assert_eq!(log.objects.len(), 5);
    let times: Vec<String> = log
        .events
        .iter()
        .map(|event| event.time.to_rfc3339())
        .collect();
    assert_eq!(
        times,
        [
            "2027-01-01T10:00:00+00:00",
            "2027-01-01T10:00:00+00:00",
            "2027-01-01T11:00:00+00:00",
        ]
    );
    let p1 = &log.events[2];
    assert_eq!(p1.id, node(0x11));
    assert_eq!(
        p1.time.timestamp_millis(),
        JANUARY_FIRST_0900_MS + 2 * fixtures::HOUR_MS
    );
    let event_object: usize = log
        .events
        .iter()
        .map(|event| event.relationships.len())
        .sum();
    let object_object: usize = log
        .objects
        .iter()
        .map(|object| object.relationships.len())
        .sum();
    assert_eq!((event_object, object_object), (4, 2));
    let declared: Vec<&str> = log
        .object_types
        .iter()
        .map(|declared| declared.name.as_str())
        .collect();
    for object in &log.objects {
        assert!(
            declared.contains(&object.object_type.as_str()),
            "{object:?}"
        );
    }
    let c1 = &log.objects[0];
    assert_eq!(c1.attributes.len(), 3);
    assert_eq!(c1.attributes[0].value.to_string(), "first case");

    // The flag's log reads back too.
    let (_, document, _) = read_with(&runtime, Some(0), &["note"]);
    let log = import_ocel_json_slice(&ocel_file(&document)).expect("the flag's log reads");
    assert_eq!((log.events.len(), log.objects.len()), (1, 6));
}

/// What the OCEL 2.0 JSON schema (https://www.ocel-standard.org/2.0/ocel20-schema-json.json)
/// requires of a log, which a reader that coerces values does not check: the four top-level
/// arrays, each type's name and attributes, each event's id, type and time, each object's id and
/// type, and every attribute value and relationship field a string.
#[test]
fn the_log_holds_what_the_ocel_2_0_json_schema_requires() {
    let (_work, runtime) = built(Provider::Sqlite);
    fixtures::commit_later_ocel(&runtime);
    for (at, events) in [
        (Some(0), &[][..]),
        (Some(1), &[][..]),
        (None, &[][..]),
        (Some(0), &["note"][..]),
        (None, &["note", "case"][..]),
    ] {
        let (_, document, _) = read_with(&runtime, at, events);
        let log = document["ocel"].as_object().expect("an object");
        for key in ["eventTypes", "objectTypes", "events", "objects"] {
            assert!(log[key].is_array(), "{key}");
        }
        for declared in log["eventTypes"]
            .as_array()
            .into_iter()
            .chain(log["objectTypes"].as_array())
            .flatten()
        {
            assert!(declared["name"].is_string(), "{declared}");
            for attribute in declared["attributes"].as_array().expect("attributes") {
                assert!(attribute["name"].is_string() && attribute["type"].is_string());
                assert!(
                    ["string", "time", "integer", "float", "boolean"]
                        .contains(&attribute["type"].as_str().unwrap_or_default()),
                    "{attribute}"
                );
            }
        }
        // RFC 3339's date-time in UTC with milliseconds: YYYY-MM-DDTHH:MM:SS.mmmZ.
        let rfc3339 = |time: &Value| {
            let text = time.as_str().expect("a time is a string").as_bytes();
            let digits = [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18, 20, 21, 22];
            let marks = [
                (4, b'-'),
                (7, b'-'),
                (10, b'T'),
                (13, b':'),
                (16, b':'),
                (19, b'.'),
                (23, b'Z'),
            ];
            assert_eq!(text.len(), 24, "{time}");
            assert!(digits.iter().all(|at| text[*at].is_ascii_digit()), "{time}");
            assert!(marks.iter().all(|(at, mark)| text[*at] == *mark), "{time}");
        };
        for event in log["events"].as_array().expect("events") {
            assert!(
                event["id"].is_string() && event["type"].is_string(),
                "{event}"
            );
            rfc3339(&event["time"]);
            for attribute in event["attributes"].as_array().expect("attributes") {
                assert!(attribute["name"].is_string() && attribute["value"].is_string());
            }
            for related in event["relationships"].as_array().expect("relationships") {
                assert!(related["objectId"].is_string() && related["qualifier"].is_string());
            }
        }
        for object in log["objects"].as_array().expect("objects") {
            assert!(
                object["id"].is_string() && object["type"].is_string(),
                "{object}"
            );
            for attribute in object["attributes"].as_array().expect("attributes") {
                assert!(attribute["name"].is_string() && attribute["value"].is_string());
                rfc3339(&attribute["time"]);
            }
            for related in object["relationships"].as_array().expect("relationships") {
                assert!(related["objectId"].is_string() && related["qualifier"].is_string());
            }
        }
    }
}

#[test]
fn two_reads_of_one_request_are_byte_identical_on_both_providers_before_and_after_a_commit() {
    let requests: [(Option<u64>, &[&str]); 4] = [
        (Some(0), &[]),
        (Some(1), &[]),
        (Some(2), &[]),
        (Some(2), &["note"]),
    ];
    let mut per_provider = Vec::new();
    for provider in PROVIDERS {
        let name = provider.name();
        let (_work, runtime) = built(provider);
        let before: Vec<Vec<u8>> = requests
            .iter()
            .map(|(at, events)| read_with(&runtime, *at, events).0)
            .collect();
        for ((at, events), bytes) in requests.iter().zip(&before) {
            assert_eq!(
                &read_with(&runtime, *at, events).0,
                bytes,
                "{name}: {at:?} {events:?} read twice"
            );
        }
        assert_eq!(read(&runtime, None).0, before[2], "{name}");
        fixtures::commit_later_ocel(&runtime);
        for ((at, events), bytes) in requests.iter().zip(&before) {
            assert_eq!(
                &read_with(&runtime, *at, events).0,
                bytes,
                "{name}: {at:?} {events:?} after a later commit"
            );
        }
        // Revision 3 dates n2. The rule still marks no note type an event type — n2 has one
        // dated fact, which is not judged — but named, note's n2 is now an event.
        let (_, _, by_rule) = read(&runtime, None);
        assert_eq!(
            (by_rule.revision, by_rule.event_types, by_rule.events),
            (3, 1, 3),
            "{name}"
        );
        let (bytes, document, event) = read_with(&runtime, None, &["note"]);
        assert_eq!(
            event,
            OcelExported {
                revision: 3,
                event_types: 1,
                object_types: 4,
                events: 2,
                objects: 6,
                event_object_relationships: 2,
                object_object_relationships: 5,
                edges_between_events: 0,
                undated_events: 0,
                edges_of_undated_events: 0,
                parallel_edges_merged: 1,
                attribute_values_out_of_range: 0,
                ocel_hash: hex(&bytes),
            },
            "{name}"
        );
        assert_eq!(
            document["ocel"]["events"][1],
            json!({
                "id": node(0x22), "type": uuid(O_NOTE), "time": "2027-01-01T13:00:00.000Z",
                "attributes": [], "relationships": [relationship(1, O_HOLDS)],
            }),
            "{name}"
        );
        per_provider.push((before, bytes));
    }
    assert_eq!(per_provider[0], per_provider[1], "file and sqlite");
}

#[test]
fn an_unseeded_store_and_a_revision_beyond_the_head_are_refused_by_name() {
    for provider in PROVIDERS {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        assert!(matches!(
            ekr_views::export_ocel(&runtime, None, &[]),
            Err(OcelError::Project(ProjectError::NotSeeded {
                requested: None
            }))
        ));
        Fixture::Ocel.build(&runtime);
        match ekr_views::export_ocel(&runtime, Some(RevisionNumber::new(3)), &names(&["step"])) {
            Err(OcelError::Project(ProjectError::RevisionNotFound { requested, head })) => {
                assert_eq!((requested.get(), head.get()), (3, 2), "{}", provider.name());
            }
            other => panic!("{}: {other:?}", provider.name()),
        }
    }
}

/// `ekr_views::ocel`, the pure half, renders an index of a revision to exactly the bytes
/// `export_ocel` answers for it, under the format literal `OCEL_FORMAT`.
#[test]
fn the_pure_half_renders_an_index_to_the_exported_bytes() {
    let (_work, runtime) = built(Provider::File);
    for at in [0, 1, 2] {
        let loaded =
            ekr_views::load(&runtime, Some(RevisionNumber::new(at))).expect("the revision loads");
        let index = Index::build(loaded).expect("the revision indexes");
        for events in [&[][..], &["note"][..]] {
            let rendered = ekr_views::ocel(&index, &names(events)).expect("the revision renders");
            let (bytes, document, event) = read_with(&runtime, Some(at), events);
            assert_eq!(rendered.bytes, bytes, "revision {at} {events:?}");
            assert_eq!(rendered.summary, event, "revision {at} {events:?}");
            assert_eq!(document["meta"]["format"], ekr_views::OCEL_FORMAT);
        }
    }
}
