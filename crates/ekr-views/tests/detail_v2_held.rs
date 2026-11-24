//! `ekr.node-detail/2` answers what the store holds (story:ekr-views-detail-fixes, review of EKR
//! 0.0.42, findings 27, 28 and 30), on both native providers:
//!
//! - 27: a relation recorded only as Active Relation assertions, with no edge, is a relation group
//!   of both its ends, as `ekr.node-detail/1` lists it (`docs/cli.md`, "Relations: assertion, edge
//!   or both");
//! - 28: an `edge_type` that parses as an id but names no edge type by id is looked up by name, so
//!   an edge type whose name is UUID-shaped is found by that name;
//! - 30: every String a value holds, at any depth of a List or Record, is cut at 2,000 bytes and
//!   marks the value `truncated`.
//!
//! The seed: `Person` and `Organization`; `CEO_OF` (Person to Organization) and an edge type whose
//! name is the text of an id no type has; Alice with a `CEO_OF` edge to Acme and the assertion
//! that matches it, Bob with two `CEO_OF` assertions to Acme and no edge (the shipped example
//! seed's shape), an edge of the UUID-named type from Alice to Acme; and Acme holding a 2,403-byte
//! text as a String, inside a List and inside a Record.

mod support;

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AssertionId, ContentHash, EvidenceId, NodeId, Timestamp};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Edge, Evidence, EvidenceSource, Node,
    Object, Predicate, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::SeedDocument;
use ekr_ontology::{Cardinality, EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use ekr_views::{DetailRequest, Index};
use serde_json::Value as Json;

use support::fixtures::{self, context, id, Provider};

const PERSON: u64 = 0xd7_0001;
const ORGANIZATION: u64 = 0xd7_0002;
const CEO_OF: u64 = 0xd7_0003;
/// The edge type whose name is the text of [`UNTYPED`].
const UUID_NAMED: u64 = 0xd7_0206;
/// An id no type has; [`UUID_NAMED`]'s name is its text.
const UNTYPED: u64 = 0xd7_0999;
const BIO: u64 = 0xd7_0011;
const NOTES: u64 = 0xd7_0012;
const PROFILE: u64 = 0xd7_0013;
const ALICE: u64 = 0xd7_0101;
const BOB: u64 = 0xd7_0102;
const ACME: u64 = 0xd7_0103;
const ALICE_CEO_EDGE: u64 = 0xd7_0201;
const ALICE_NAMED_EDGE: u64 = 0xd7_0202;
const ALICE_CLAIM: u64 = 0xd7_0301;
/// Bob is CEO of Acme from 2026-03-12.
const BOB_CLAIM_2026: u64 = 0xd7_0302;
/// Bob is CEO of Acme from 2020-01-01.
const BOB_CLAIM_2020: u64 = 0xd7_0303;
const STATEMENT: u64 = 0xd7_0401;
/// The bytes of the long text: over 2,000, all ASCII, so the cut is exactly 2,000.
const LONG: usize = 2_403;

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn long_text() -> String {
    "x".repeat(LONG)
}

fn relation(assertion: u64, subject: u64, valid_from: i64) -> Assertion<Value> {
    Assertion {
        id: id::<AssertionId>(assertion),
        root_id: id(2),
        subject: Subject::Node(id(subject)),
        predicate: Predicate::Relation(id(CEO_OF)),
        object: Object::Node(id(ACME)),
        evidence: [id::<EvidenceId>(STATEMENT)].into_iter().collect(),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::from_millis(valid_from)),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}

fn seed() -> SeedDocument {
    let empty = support::workspace_root().join("crates/ekr-views/tests/fixtures/seed-empty.yaml");
    let mut document =
        SeedDocument::from_yaml(&std::fs::read_to_string(empty).expect("the empty seed"))
            .expect("the empty seed parses");
    let root = document.graph.root.id;

    let mut organization = NodeType::new(id(ORGANIZATION), "Organization");
    for (property, name, value_type) in [
        (BIO, "bio", ValueType::String),
        (NOTES, "notes", ValueType::List(Box::new(ValueType::String))),
        (
            PROFILE,
            "profile",
            ValueType::Record(BTreeMap::from([("summary".to_owned(), ValueType::String)])),
        ),
    ] {
        organization.properties.insert(
            id(property),
            PropertyDefinition::new(id(property), name, value_type),
        );
    }
    document
        .ontology
        .node_types
        .extend([NodeType::new(id(PERSON), "Person"), organization]);
    for (type_id, name) in [(CEO_OF, "CEO_OF".to_owned()), (UUID_NAMED, uuid(UNTYPED))] {
        let mut edge_type = EdgeType::new(id(type_id), name);
        edge_type.source_types = BTreeSet::from([id(PERSON)]);
        edge_type.target_types = BTreeSet::from([id(ORGANIZATION)]);
        edge_type.cardinality = Cardinality::Many;
        document.ontology.edge_types.push(edge_type);
    }

    for (node, type_id, name) in [
        (ALICE, PERSON, "Alice"),
        (BOB, PERSON, "Bob"),
        (ACME, ORGANIZATION, "Acme"),
    ] {
        let mut held = Node::<Value>::new(id(node), root, id(type_id), name);
        if node == ACME {
            held.properties = BTreeMap::from([
                (id(BIO), vec![Value::String(long_text())]),
                (
                    id(NOTES),
                    vec![Value::List(vec![
                        Value::String("short".into()),
                        Value::String(long_text()),
                    ])],
                ),
                (
                    id(PROFILE),
                    vec![Value::Record(BTreeMap::from([(
                        "summary".to_owned(),
                        Value::String(long_text()),
                    )]))],
                ),
            ]);
        }
        document.graph.nodes.insert(held.id, held);
    }
    for (edge, type_id) in [(ALICE_CEO_EDGE, CEO_OF), (ALICE_NAMED_EDGE, UUID_NAMED)] {
        let held = Edge::<Value> {
            id: id(edge),
            root_id: root,
            type_id: id(type_id),
            source: id(ALICE),
            target: id(ACME),
            properties: BTreeMap::new(),
        };
        document.graph.edges.insert(held.id, held);
    }

    let bytes = b"Bob became CEO of Acme on 2026-03-12.".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let statement = Evidence {
        id: id(STATEMENT),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(1_000),
        confidence: Confidence::from_basis_points(9_000).expect("basis points"),
    };
    document.graph.evidence.insert(statement.id, statement);
    document.evidence_payloads.insert(hash, bytes.into());
    for claim in [
        relation(ALICE_CLAIM, ALICE, 1_577_836_800_000),
        relation(BOB_CLAIM_2026, BOB, 1_773_273_600_000),
        relation(BOB_CLAIM_2020, BOB, 1_577_836_800_000),
    ] {
        document.graph.assertions.insert(claim.id, claim);
    }
    document
}

/// The seeded store's revision 0 on `provider`.
fn index(provider: Provider) -> Index {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    runtime
        .seed(seed(), || Timestamp::from_millis(1))
        .expect("the seed is admitted");
    Index::load(&runtime, None).expect("revision 0 indexes")
}

fn detail(index: &Index, node: u64, edge_type: Option<String>) -> Json {
    let request = DetailRequest::new(None, edge_type, None).expect("bounded");
    let answer = index
        .detail_v2(id::<NodeId>(node), &request)
        .expect("detailed");
    serde_json::from_slice(&answer.bytes).expect("JSON")
}

/// A listed neighbour: its name, its `assertion` and its `assertions`.
type Neighbour = (String, Option<String>, u64);

/// A relation group: its name, direction, total and listed neighbours.
type Group = (String, String, u64, Vec<Neighbour>);

/// Each relation group of `detail`, in its order.
fn groups(detail: &Json) -> Vec<Group> {
    detail["relations"]
        .as_array()
        .expect("relations")
        .iter()
        .map(|group| {
            (
                group["name"].as_str().expect("named").to_owned(),
                group["direction"].as_str().expect("direction").to_owned(),
                group["total"].as_u64().expect("total"),
                group["nodes"]
                    .as_array()
                    .expect("nodes")
                    .iter()
                    .map(|node| {
                        (
                            node["name"].as_str().expect("name").to_owned(),
                            node["assertion"].as_str().map(str::to_owned),
                            node["assertions"].as_u64().expect("assertions"),
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}

fn ceo_of_by_assertions_alone(provider: Provider) {
    let index = index(provider);
    let bob = detail(&index, BOB, None);
    assert_eq!(
        groups(&bob),
        [(
            "CEO_OF".to_owned(),
            "out".to_owned(),
            1,
            vec![("Acme".to_owned(), Some(uuid(BOB_CLAIM_2026)), 2)]
        )],
        "{bob}"
    );
    assert_eq!(bob["meta"]["truncated"], false, "{bob}");
    // Bob has no edge: his degree and edge count stay 0.
    assert_eq!(bob["meta"]["edges"], 0);
    assert_eq!(bob["node"]["degree"], 0);
    // Narrowed to CEO_OF, by name, the group is answered too.
    assert_eq!(
        groups(&detail(&index, BOB, Some("ceo_of".into()))),
        groups(&bob)
    );
    // Acme's incoming CEO_OF group holds Alice (an edge and an assertion) and Bob (assertions).
    let acme = detail(&index, ACME, Some("CEO_OF".into()));
    assert_eq!(
        groups(&acme),
        [(
            "CEO_OF".to_owned(),
            "in".to_owned(),
            2,
            vec![
                ("Alice".to_owned(), Some(uuid(ALICE_CLAIM)), 1),
                ("Bob".to_owned(), Some(uuid(BOB_CLAIM_2026)), 2),
            ]
        )],
        "{acme}"
    );
}

/// Finding 27: Bob's two Active `CEO_OF` assertions, with no edge, are his `CEO_OF` group.
#[test]
fn a_relation_held_only_as_assertions_is_a_group_on_the_file_store() {
    ceo_of_by_assertions_alone(Provider::File);
}

/// Finding 27, on SQLite.
#[test]
fn a_relation_held_only_as_assertions_is_a_group_on_the_sqlite_store() {
    ceo_of_by_assertions_alone(Provider::Sqlite);
}

fn uuid_shaped_name(provider: Provider) {
    let index = index(provider);
    assert_eq!(index.edge_type_named(&uuid(UNTYPED)), Some(id(UUID_NAMED)));
    assert_eq!(
        index.edge_type_named(&uuid(UUID_NAMED)),
        Some(id(UUID_NAMED))
    );
    // A node type's id is no edge type's id and no edge type's name.
    assert_eq!(index.edge_type_named(&uuid(PERSON)), None);
    let by_id = detail(&index, ALICE, Some(uuid(UUID_NAMED)));
    let by_name = detail(&index, ALICE, Some(uuid(UNTYPED)));
    let expected = [(
        uuid(UNTYPED),
        "out".to_owned(),
        1,
        vec![("Acme".to_owned(), None, 0)],
    )];
    assert_eq!(groups(&by_id), expected, "{by_id}");
    assert_eq!(groups(&by_name), expected, "{by_name}");
    assert_eq!(by_name["meta"]["edge_type"], uuid(UNTYPED));
    // An id-shaped text that names nothing answers no group.
    assert_eq!(groups(&detail(&index, ALICE, Some(uuid(0xd7_0998)))), []);
}

/// Finding 28: an edge type whose name is the text of another id is found by that name.
#[test]
fn an_edge_type_with_a_uuid_shaped_name_is_found_by_it_on_the_file_store() {
    uuid_shaped_name(Provider::File);
}

/// Finding 28, on SQLite.
#[test]
fn an_edge_type_with_a_uuid_shaped_name_is_found_by_it_on_the_sqlite_store() {
    uuid_shaped_name(Provider::Sqlite);
}

/// Every String's bytes in a projected value, at any depth.
fn strings(value: &Json, found: &mut Vec<usize>) {
    match (value["kind"].as_str(), &value["value"]) {
        (Some("String"), Json::String(text)) => found.push(text.len()),
        (Some("List"), Json::Array(items)) => items.iter().for_each(|item| strings(item, found)),
        (Some("Record"), Json::Object(fields)) => {
            fields.values().for_each(|field| strings(field, found));
        }
        _ => {}
    }
}

fn one_text_limit(provider: Provider) {
    let acme = detail(&index(provider), ACME, None);
    let properties: Vec<(String, bool, Vec<usize>)> = acme["properties"]
        .as_array()
        .expect("properties")
        .iter()
        .map(|group| {
            let value = &group["values"][0];
            let mut found = Vec::new();
            strings(&value["value"], &mut found);
            (
                group["name"].as_str().expect("named").to_owned(),
                value["truncated"].as_bool().expect("truncated"),
                found,
            )
        })
        .collect();
    assert_eq!(
        properties,
        [
            ("bio".to_owned(), true, vec![2_000]),
            ("notes".to_owned(), true, vec![5, 2_000]),
            ("profile".to_owned(), true, vec![2_000]),
        ]
    );
    assert_eq!(acme["meta"]["truncated"], true);
}

/// Finding 30: a 2,403-byte String is cut to 2,000 bytes and marked whether it is the value, an
/// element of a List or a field of a Record.
#[test]
fn every_string_in_a_value_is_cut_at_2000_bytes_on_the_file_store() {
    one_text_limit(Provider::File);
}

/// Finding 30, on SQLite.
#[test]
fn every_string_in_a_value_is_cut_at_2000_bytes_on_the_sqlite_store() {
    one_text_limit(Provider::Sqlite);
}
