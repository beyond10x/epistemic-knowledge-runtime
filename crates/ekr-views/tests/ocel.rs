//! `task:store-exports-ocel-event-log`: ExportOcel and its format ekr.ocel/1, held against
//! `systems/ekr/domains/views.yaml` on the `ocel` fixture, whose log per revision is known
//! (an-ocel-export-takes-events-from-valid-time-and-objects-from-the-rest.yaml states it), on both
//! native providers:
//!
//! * the head's `ocel` member is exactly the log the format's rules give, and its bytes are
//!   pinned;
//! * an OCEL 2.0 reader — the `process_mining` crate (rust4pm) — reads that member back with the
//!   fixture's event types, object types, events, objects and relationships, and every value the
//!   OCEL 2.0 JSON schema requires to be a string is one;
//! * two reads of one revision are byte-identical, on one provider and across both, before and
//!   after a later commit that turns an object type into an event type;
//! * an unseeded store and a revision beyond the head are refused by name.

mod support;

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use ekr_views::{OcelExported, ProjectError};
use process_mining::core::event_data::object_centric::ocel_json::import_ocel_json_slice;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use support::fixtures::{
    self, Fixture, Provider, JANUARY_FIRST_0900_MS, O_ABOUT, O_CASE, O_COST, O_DEPTH, O_HOLDS,
    O_NODES, O_NOTE, O_OPENED, O_OUTCOME, O_SIZE, O_STEP, O_SUBCASE, O_TAGS, O_TEXT, O_TITLE,
    O_TOUCHES, O_UNUSED,
};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

/// The SHA-256 of the head's `ekr.ocel/1` document, pinned once its log equalled [`log`]: its
/// bytes do not change unless the format does.
const HEAD_HASH: &str = "225f62ec41fea2de53281725d45d90516f7bdd913124a856ef72866818f70f93";

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

/// The export of revision `at` (the head when `None`): its bytes, parsed, and its event.
fn read(runtime: &Runtime, at: Option<u64>) -> (Vec<u8>, Value, OcelExported) {
    let answer = ekr_views::export_ocel(runtime, at.map(RevisionNumber::new))
        .expect("the revision is exported");
    let value: Value = serde_json::from_slice(&answer.bytes).expect("JSON");
    (answer.bytes, value, answer.summary)
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
        attribute(O_OPENED, "time"),
    ]
}

/// The log of revisions 0 and 2, with p2 at `p2_time`, as the format's rules give it.
fn log(p2_time: &str) -> Value {
    let mut subcase = case_attributes();
    subcase.push(attribute(O_DEPTH, "integer"));
    json!({
        "eventTypes": [
            {"name": uuid(O_STEP), "attributes": [attribute(O_OUTCOME, "string"), attribute(O_COST, "float")]},
        ],
        "objectTypes": [
            {"name": uuid(O_CASE), "attributes": case_attributes()},
            {"name": uuid(O_SUBCASE), "attributes": subcase},
            {"name": uuid(O_NOTE), "attributes": [attribute(O_TEXT, "string")]},
            {"name": uuid(O_UNUSED), "attributes": []},
        ],
        "events": [
            {
                "id": node(0x12), "type": uuid(O_STEP), "time": p2_time,
                "attributes": [value(O_OUTCOME, "failed")],
                "relationships": [relationship(2, O_ABOUT)],
            },
            {
                "id": node(0x13), "type": uuid(O_STEP), "time": "2027-01-01T10:00:00.000Z",
                "attributes": [],
                "relationships": [relationship(1, O_TOUCHES), relationship(3, O_ABOUT)],
            },
            {
                "id": node(0x11), "type": uuid(O_STEP), "time": "2027-01-01T11:00:00.000Z",
                "attributes": [value(O_OUTCOME, "done"), value(O_COST, "12.50")],
                "relationships": [relationship(1, O_ABOUT)],
            },
        ],
        "objects": [
            {
                "id": node(1), "type": uuid(O_CASE),
                "attributes": [
                    initial(O_TITLE, "first case"),
                    initial(O_SIZE, "3"),
                    initial(O_TAGS, r#"[{"kind":"String","value":"red"},{"kind":"String","value":"blue"}]"#),
                    initial(O_OPENED, "2026-12-31T09:00:00.000Z"),
                ],
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

fn summary(revision: u64, hash: &[u8]) -> OcelExported {
    OcelExported {
        revision,
        event_types: 1,
        object_types: 4,
        events: 3,
        objects: 5,
        event_object_relationships: 4,
        object_object_relationships: 2,
        edges_between_events: 1,
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
fn each_revision_exports_the_fixtures_log_and_the_heads_bytes_are_pinned() {
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let (bytes, document, event) = read(&runtime, None);
        assert_eq!(
            document["meta"],
            json!({"format": "ekr.ocel/1", "revision": 2}),
            "{}",
            provider.name()
        );
        assert_eq!(
            document["ocel"],
            log("2027-01-01T10:00:00.000Z"),
            "{}",
            provider.name()
        );
        assert_eq!(event, summary(2, &bytes), "{}", provider.name());
        assert_eq!(hex(&bytes), HEAD_HASH, "{}", provider.name());

        let (bytes, document, event) = read(&runtime, Some(0));
        assert_eq!(document["meta"]["revision"], 0);
        assert_eq!(
            document["ocel"],
            log("2027-01-01T10:00:00.000Z"),
            "{}",
            provider.name()
        );
        assert_eq!(event, summary(0, &bytes), "{}", provider.name());

        // Revision 1's earlier valid time moves p2; revision 2 retracted it.
        let (bytes, document, event) = read(&runtime, Some(1));
        assert_eq!(
            document["ocel"],
            log("2026-12-31T09:00:00.000Z"),
            "{}",
            provider.name()
        );
        assert_eq!(event, summary(1, &bytes), "{}", provider.name());
    }
}

#[test]
fn an_ocel_2_0_reader_reads_the_log_back_with_its_events_objects_and_relationships() {
    let (_work, runtime) = built(Provider::File);
    let (_, document, _) = read(&runtime, None);
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
    assert_eq!(c1.attributes.len(), 4);
    assert_eq!(c1.attributes[0].value.to_string(), "first case");
}

/// What the OCEL 2.0 JSON schema (https://www.ocel-standard.org/2.0/ocel20-schema-json.json)
/// requires of a log, which a reader that coerces values does not check: the four top-level
/// arrays, each type's name and attributes, each event's id, type and time, each object's id and
/// type, and every attribute value and relationship field a string.
#[test]
fn the_log_holds_what_the_ocel_2_0_json_schema_requires() {
    let (_work, runtime) = built(Provider::Sqlite);
    commit(&runtime);
    for at in [Some(0), Some(1), None] {
        let (_, document, _) = read(&runtime, at);
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

fn commit(runtime: &Runtime) {
    fixtures::commit_later_ocel(runtime);
}

#[test]
fn two_reads_of_one_revision_are_byte_identical_on_both_providers_before_and_after_a_commit() {
    let mut per_provider = Vec::new();
    for provider in PROVIDERS {
        let (_work, runtime) = built(provider);
        let before: Vec<Vec<u8>> = [Some(0), Some(1), Some(2)]
            .into_iter()
            .map(|at| read(&runtime, at).0)
            .collect();
        for (at, bytes) in before.iter().enumerate() {
            assert_eq!(
                &read(&runtime, Some(at as u64)).0,
                bytes,
                "{}: revision {at} read twice",
                provider.name()
            );
        }
        assert_eq!(read(&runtime, None).0, before[2], "{}", provider.name());
        commit(&runtime);
        for (at, bytes) in before.iter().enumerate() {
            assert_eq!(
                &read(&runtime, Some(at as u64)).0,
                bytes,
                "{}: revision {at} after a later commit",
                provider.name()
            );
        }
        // Revision 3 dates n2, so every note has a valid-time start and note is an event type:
        // c1's two `holds` edges are now held by the events n1 and n2.
        let (bytes, document, event) = read(&runtime, None);
        assert_eq!(
            event,
            OcelExported {
                revision: 3,
                event_types: 2,
                object_types: 3,
                events: 5,
                objects: 3,
                event_object_relationships: 6,
                object_object_relationships: 0,
                edges_between_events: 1,
                ocel_hash: hex(&bytes),
            },
            "{}",
            provider.name()
        );
        assert_eq!(
            document["ocel"]["events"][4],
            json!({
                "id": node(0x22), "type": uuid(O_NOTE), "time": "2027-01-01T13:00:00.000Z",
                "attributes": [], "relationships": [relationship(1, O_HOLDS)],
            }),
            "{}",
            provider.name()
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
            ekr_views::export_ocel(&runtime, None),
            Err(ProjectError::NotSeeded { requested: None })
        ));
        Fixture::Ocel.build(&runtime);
        match ekr_views::export_ocel(&runtime, Some(RevisionNumber::new(3))) {
            Err(ProjectError::RevisionNotFound { requested, head }) => {
                assert_eq!((requested.get(), head.get()), (3, 2), "{}", provider.name());
            }
            other => panic!("{}: {other:?}", provider.name()),
        }
    }
}

/// `ekr_views::ocel`, the pure half, renders a loaded revision to exactly the bytes
/// `export_ocel` answers for it, under the format literal `OCEL_FORMAT`.
#[test]
fn the_pure_half_renders_a_loaded_revision_to_the_exported_bytes() {
    let (_work, runtime) = built(Provider::File);
    for at in [0, 1, 2] {
        let loaded =
            ekr_views::load(&runtime, Some(RevisionNumber::new(at))).expect("the revision loads");
        let rendered = ekr_views::ocel(&loaded).expect("the revision renders");
        let (bytes, document, event) = read(&runtime, Some(at));
        assert_eq!(rendered.bytes, bytes, "revision {at}");
        assert_eq!(rendered.summary, event, "revision {at}");
        assert_eq!(document["meta"]["format"], ekr_views::OCEL_FORMAT);
    }
}
