//! Adversary pass 1 on `task:store-exports-ocel-event-log` (wave ops-05, unit O): ExportOcel held
//! against `systems/ekr/domains/views.yaml` (`ekr.views.OcelLogV1`) and the OCEL 2.0
//! specification (§ 3, Definitions 1 and 2) on stores the unit's own fixture does not build:
//!
//! * a node of an event type whose time the time form cannot write (after 9999-12-31, before
//!   0000-01-01) is left out and counted, which no case of the unit's suite reaches;
//! * an attribute a type declares `time` holds a time;
//! * a property a node holds no value of is not an attribute (`views.yaml`: "one attribute per
//!   property it holds a value of");
//! * an event does not precede the time its objects' attribute values start (OCEL 2.0
//!   Definition 2: `oaval` at a time before every assignment is undefined);
//! * an attribute name is one type's (OCEL 2.0 Definition 2: `eatype` and `oatype` are functions);
//! * `names` is the requested revision's own before a rename, and `--events` names node types
//!   only.

mod support;

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{PropertyId, RevisionNumber, Timestamp, TransactionId, TypeId};
use ekr_graph::{Edge, Node};
use ekr_kernel::{
    CommitCommandResult, GraphOperation, GraphTransaction, PropertyMutation, Runtime, SeedDocument,
    ValidationCommandResult,
};
use ekr_ontology::{Cardinality, EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use ekr_views::{OcelError, OcelExported};
use serde::Serialize;
use serde_json::Value as Json;

use support::fixtures::{self, context, id, Fixture, Provider};

/// This file's ids: the node types `evt` and `thing`, the edge type `rel` (evt → thing).
const EVT: u64 = 0xad_0001;
const THING: u64 = 0xad_0002;
const REL: u64 = 0xad_0010;
/// `evt`'s `at` (Timestamp, One); `thing`'s `label` (String, One), `when` (Timestamp, One) and
/// `remark` (String, One, optional).
const AT: u64 = 0xad_0020;
const LABEL: u64 = 0xad_0021;
const WHEN: u64 = 0xad_0022;
const REMARK: u64 = 0xad_0023;
/// Nodes: events `old` (1900), `ok` (2027), `far` (2^53 ms) and `before` (-2^53 ms); things `t1`
/// and `t2`.
const OLD: u64 = 0xad_0101;
const OK: u64 = 0xad_0102;
const FAR: u64 = 0xad_0103;
const BEFORE: u64 = 0xad_0104;
const T1: u64 = 0xad_0111;
const T2: u64 = 0xad_0112;
const EDGES: u64 = 0xad_0200;

/// 1900-01-01T00:00:00.000Z.
const YEAR_1900_MS: i64 = -2_208_988_800_000;
/// 2027-01-01T09:00:00.000Z.
const YEAR_2027_MS: i64 = 1_798_794_000_000;
/// 2^53 ms, after year 285 000: no four-digit year writes it.
const TWO_TO_53_MS: i64 = 9_007_199_254_740_992;

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

/// One seed node: its id, its type, its name and its property values.
type SeedNode = (u64, u64, &'static str, Vec<(u64, Vec<Value>)>);

fn empty_seed() -> SeedDocument {
    SeedDocument::from_yaml(&support::read(
        "crates/ekr-views/tests/fixtures/seed-empty.yaml",
    ))
    .expect("the empty seed parses")
}

/// The store: `old`, `ok`, `far` and `before` each `rel` → `t1`; `t1` with a label and a `when`
/// of 2^53 ms; `t2` with a label. Then revision 1 sets `t2`'s optional `remark` to no value.
fn build(runtime: &Runtime) {
    let mut document = empty_seed();
    let root = document.graph.root.id;
    let property = |n: u64, name: &str, value_type: ValueType| {
        PropertyDefinition::new(id(n), name, value_type)
    };
    let mut evt = NodeType::new(id(EVT), "evt");
    evt.properties
        .insert(id(AT), property(AT, "at", ValueType::Timestamp));
    let mut thing = NodeType::new(id(THING), "thing");
    for declared in [
        property(LABEL, "label", ValueType::String),
        property(WHEN, "when", ValueType::Timestamp),
        property(REMARK, "remark", ValueType::String),
    ] {
        thing.properties.insert(declared.id, declared);
    }
    document.ontology.node_types.extend([evt, thing]);
    let mut rel = EdgeType::new(id(REL), "rel");
    rel.source_types = [id::<TypeId>(EVT)].into_iter().collect();
    rel.target_types = [id::<TypeId>(THING)].into_iter().collect();
    rel.cardinality = Cardinality::Many;
    document.ontology.edge_types.push(rel);

    let at = |millis: i64| Value::Timestamp(Timestamp::from_millis(millis));
    let nodes: [SeedNode; 6] = [
        (OLD, EVT, "old", vec![(AT, vec![at(YEAR_1900_MS)])]),
        (OK, EVT, "ok", vec![(AT, vec![at(YEAR_2027_MS)])]),
        (FAR, EVT, "far", vec![(AT, vec![at(TWO_TO_53_MS)])]),
        (BEFORE, EVT, "before", vec![(AT, vec![at(-TWO_TO_53_MS)])]),
        (
            T1,
            THING,
            "t1",
            vec![
                (LABEL, vec![Value::String("one".into())]),
                (WHEN, vec![at(TWO_TO_53_MS)]),
            ],
        ),
        (
            T2,
            THING,
            "t2",
            vec![(LABEL, vec![Value::String("two".into())])],
        ),
    ];
    for (n, type_id, name, properties) in nodes {
        let mut held = Node::<Value>::new(id(n), root, id(type_id), name);
        held.properties = properties
            .into_iter()
            .map(|(property, values)| (id::<PropertyId>(property), values))
            .collect();
        document.graph.nodes.insert(held.id, held);
    }
    for (n, source) in (1..).zip([OLD, OK, FAR, BEFORE]) {
        let edge = Edge::<Value> {
            id: id(EDGES + n),
            root_id: root,
            type_id: id(REL),
            source: id(source),
            target: id(T1),
            properties: BTreeMap::new(),
        };
        document.graph.edges.insert(edge.id, edge);
    }
    runtime
        .seed(document, || Timestamp::from_millis(1_800_000_000_001))
        .expect("the seed is admitted");

    commit(
        runtime,
        vec![GraphOperation::UpdateProperty(PropertyMutation {
            node: id(T2),
            property: id(REMARK),
            values: Vec::new(),
        })],
    );
}

fn commit(runtime: &Runtime, operations: Vec<GraphOperation>) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let transaction = GraphTransaction {
        id: id::<TransactionId>(0xad_0300),
        proposer: context().operator,
        operations,
        evidence: BTreeSet::new(),
        schema_version: None,
    };
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: &transaction,
    })
    .expect("a transaction document")
    .into_bytes();
    let now = Timestamp::from_millis(1_800_000_001_000);
    runtime
        .propose(&bytes, context().operator, || now)
        .expect("proposed");
    let head = runtime.head().expect("head").expect("seeded").revision;
    let verdict = runtime
        .validate(transaction.id, head, || now)
        .expect("validation runs");
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let result = runtime
        .commit(transaction.id, context().operator, || now)
        .expect("commit runs");
    assert!(
        matches!(result, CommitCommandResult::Committed(_)),
        "{result:?}"
    );
}

fn built() -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    build(&runtime);
    (work, runtime)
}

fn export(runtime: &Runtime, at: Option<u64>, events: &[&str]) -> (Json, OcelExported) {
    let events: Vec<String> = events.iter().map(|name| (*name).to_owned()).collect();
    let answer = ekr_views::export_ocel(runtime, at.map(RevisionNumber::new), &events)
        .expect("the revision is exported");
    (
        serde_json::from_slice(&answer.bytes).expect("JSON"),
        answer.summary,
    )
}

fn array<'a>(value: &'a Json, key: &str) -> &'a Vec<Json> {
    value[key]
        .as_array()
        .unwrap_or_else(|| panic!("{key} is an array"))
}

/// `YYYY-MM-DDTHH:MM:SS.mmmZ`.
fn is_time(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 24
        && [4, 7].iter().all(|at| bytes[*at] == b'-')
        && bytes[10] == b'T'
        && [13, 16].iter().all(|at| bytes[*at] == b':')
        && bytes[19] == b'.'
        && bytes[23] == b'Z'
        && [0, 1, 2, 3, 5, 6, 8, 9, 11, 12, 14, 15, 17, 18, 20, 21, 22]
            .iter()
            .all(|at| bytes[*at].is_ascii_digit())
}

/// `views.yaml`: "A node of an event type ... with a time the time form below cannot write (a
/// year outside 0000–9999), is not in the log", and `undated_events` / `edges_of_undated_events`
/// count it and its edges. The unit's fixture has no such node, so removing the range test at
/// `ocel.rs:274` leaves its suite green while `rfc3339(..).unwrap_or_default()` at `:378` writes
/// an event whose `time` is `""`.
#[test]
fn an_event_time_no_four_digit_year_writes_is_left_out_and_counted() {
    let (_work, runtime) = built();
    let (document, summary) = export(&runtime, Some(0), &["evt"]);
    let events = array(&document["ocel"], "events");
    let ids: Vec<&str> = events
        .iter()
        .map(|event| event["id"].as_str().expect("id"))
        .collect();
    assert_eq!(ids, [uuid(OLD), uuid(OK)], "{document}");
    for event in events {
        assert!(is_time(event["time"].as_str().expect("time")), "{event}");
    }
    assert_eq!(
        (
            summary.events,
            summary.undated_events,
            summary.edges_of_undated_events
        ),
        (2, 2, 2)
    );
}

/// OCEL 2.0 types an attribute so a reader can read its values as that type; the spec's attribute
/// types are string, time, integer, float and boolean. `views.yaml` types a One Timestamp
/// declaration `time` and writes a value it cannot put in the time form as "its decimal
/// milliseconds" — so the log declares `time` and holds `9007199254740992`.
#[test]
#[ignore = "defect: an attribute typed `time` holds decimal milliseconds for a Timestamp outside years 0000-9999"]
fn an_attribute_typed_time_holds_a_time() {
    let (_work, runtime) = built();
    let (document, _) = export(&runtime, Some(0), &["evt"]);
    let log = &document["ocel"];
    let mut typed: BTreeMap<(String, String), String> = BTreeMap::new();
    for declared in array(log, "eventTypes")
        .iter()
        .chain(array(log, "objectTypes"))
    {
        for attribute in array(declared, "attributes") {
            typed.insert(
                (
                    declared["name"].as_str().expect("name").to_owned(),
                    attribute["name"].as_str().expect("name").to_owned(),
                ),
                attribute["type"].as_str().expect("type").to_owned(),
            );
        }
    }
    let mut wrong = Vec::new();
    for item in array(log, "events").iter().chain(array(log, "objects")) {
        let owner = item["type"].as_str().expect("type").to_owned();
        for attribute in array(item, "attributes") {
            let name = attribute["name"].as_str().expect("name").to_owned();
            let value = attribute["value"].as_str().expect("value");
            if typed.get(&(owner.clone(), name)).map(String::as_str) == Some("time")
                && !is_time(value)
            {
                wrong.push(attribute.clone());
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "time-typed attributes without a time: {wrong:?}"
    );
}

/// `views.yaml` (`ekr.views.OcelLogV1`, Attributes): "A node's attributes are its property values,
/// one attribute per property it holds a value of"; `docs/cli.md`: "each property a node holds a
/// value of". `ocel.rs:364` writes every entry of the node's property map, so an entry with no
/// value would be an attribute `[]`; `UpdateProperty` with no values on an optional property is
/// admitted, and the kernel drops the entry (`apply.rs:243`) — a seed carrying one is refused
/// (`seed-empty-property-values`).
#[test]
fn a_property_a_node_holds_no_value_of_is_not_an_attribute() {
    let (_work, runtime) = built();
    let (document, _) = export(&runtime, Some(1), &["evt"]);
    let t2 = array(&document["ocel"], "objects")
        .iter()
        .find(|object| object["id"] == uuid(T2))
        .expect("t2 is an object");
    let names: Vec<&str> = array(t2, "attributes")
        .iter()
        .map(|attribute| attribute["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, [uuid(LABEL)], "{t2}");
}

/// OCEL 2.0 Definition 2: `oaval_t(o)` is the latest assignment at or before `t`, and undefined
/// when there is none; § 6.4 and the export give every object attribute the time
/// 1970-01-01T00:00:00.000Z. The export also writes events before 1970 (years 0000-1969), so at
/// such an event every object it relates to has no attribute value at all — and Definition 1
/// makes 1970 (`0`) the earliest timestamp. `old` (1900) relates `t1`, whose label starts in 1970.
#[test]
#[ignore = "defect: an event before 1970 precedes every attribute value of the objects it relates"]
fn no_event_precedes_the_attribute_values_of_the_objects_it_relates() {
    let (_work, runtime) = built();
    let (document, _) = export(&runtime, Some(0), &["evt"]);
    let log = &document["ocel"];
    let starts: BTreeMap<&str, Vec<&str>> = array(log, "objects")
        .iter()
        .map(|object| {
            (
                object["id"].as_str().expect("id"),
                array(object, "attributes")
                    .iter()
                    .map(|attribute| attribute["time"].as_str().expect("time"))
                    .collect(),
            )
        })
        .collect();
    let mut early = Vec::new();
    for event in array(log, "events") {
        let time = event["time"].as_str().expect("time");
        for related in array(event, "relationships") {
            let object = related["objectId"].as_str().expect("objectId");
            if starts[object].iter().any(|start| *start > time) {
                early.push((event["id"].clone(), object.to_owned(), time.to_owned()));
            }
        }
    }
    assert!(early.is_empty(), "{early:?}");
}

/// OCEL 2.0 Definition 2: `eatype : EA → Uetype` and `oatype : OA → Uotype` are functions — "the
/// set of attributes is distinct and non-overlapping for each individual event and object type".
/// The export names an attribute by its property id and lists an inherited property on every
/// type that inherits it, so the unit's own `ocel` fixture gives `case`'s title, size and tags to
/// `subcase` as well.
#[test]
#[ignore = "defect: an inherited property's attribute name is declared by two OCEL types"]
fn each_attribute_name_is_one_types() {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    Fixture::Ocel.build(&runtime);
    let (document, _) = export(&runtime, Some(0), &[]);
    let log = &document["ocel"];
    let mut owners: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for declared in array(log, "eventTypes")
        .iter()
        .chain(array(log, "objectTypes"))
    {
        for attribute in array(declared, "attributes") {
            owners
                .entry(attribute["name"].as_str().expect("name").to_owned())
                .or_default()
                .push(declared["name"].as_str().expect("name").to_owned());
        }
    }
    owners.retain(|_, types| types.len() > 1);
    assert!(owners.is_empty(), "{owners:#?}");
}

/// `names` holds the names the requested revision's ontology gives: the schema-changes fixture
/// renames `label` (0x101) to `title` at revision 3.
#[test]
fn names_are_the_requested_revisions_own_across_a_rename() {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::Sqlite);
    Fixture::SchemaChanges.build(&runtime);
    let named = |at: u64| -> Vec<String> {
        let (document, _) = export(&runtime, Some(at), &[]);
        array(&document["names"], "properties")
            .iter()
            .filter(|entry| entry["id"] == uuid(0x101))
            .map(|entry| entry["name"].as_str().expect("name").to_owned())
            .collect()
    };
    assert_eq!(named(2), ["label"]);
    assert_eq!(named(3), ["title"]);
}

/// `--events` names node types: an edge type's name or a property's is refused as
/// `ekr.views.EventTypeNotFound`, naming it.
#[test]
fn events_naming_an_edge_type_or_a_property_is_refused() {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    Fixture::Ocel.build(&runtime);
    for name in ["about", "title"] {
        match ekr_views::export_ocel(&runtime, None, &[name.to_owned()]) {
            Err(OcelError::EventTypeNotFound { name: refused, .. }) => assert_eq!(refused, name),
            other => panic!("{name}: {other:?}"),
        }
    }
}
