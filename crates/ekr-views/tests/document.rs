//! The bytes of `ekr.graph-projection/1`, held against `systems/ekr/domains/views.yaml`'s
//! determinism rules on the six-revision fixture: order, key order, encoding, presence, and
//! scoping to the projected revision.

mod support;

use ekr_core::RevisionNumber;
use ekr_views::ProjectError;
use serde_json::Value;

use support::fixtures::{self, id, Fixture, Provider};

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn render(at: u64) -> (String, Value) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    Fixture::Evolved.build(&runtime);
    let rendered = ekr_views::project(&runtime, Some(RevisionNumber::new(at))).expect("render");
    let text = String::from_utf8(rendered.bytes).expect("UTF-8");
    let value = serde_json::from_str(&text).expect("JSON");
    (text, value)
}

fn ids(value: &Value) -> Vec<String> {
    value
        .as_array()
        .expect("an array")
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map_or_else(|| entry["id"].as_str().unwrap().to_owned(), str::to_owned)
        })
        .collect()
}

#[test]
fn the_head_projection_opens_with_its_meta_in_declared_order_and_no_whitespace() {
    let (text, _) = render(5);
    assert!(
        text.starts_with(
            "{\"meta\":{\"format\":\"ekr.graph-projection/1\",\"revision\":5,\"head\":5,\
             \"node_count\":4,\"edge_count\":1,\"assertion_count\":2,\"evidence_count\":3},\
             \"ontology\":{\"node_types\":["
        ),
        "{text}"
    );
    assert!(!text.contains('\n') && !text.contains("\": ") && !text.contains(", \""));
    // Each key occurs once at the top level; an assertion's own `evidence` list precedes the
    // top-level `evidence`, which is therefore its last occurrence.
    let top: Vec<usize> = [
        "\"meta\":",
        "\"ontology\":",
        "\"nodes\":",
        "\"edges\":",
        "\"evidence\":[",
        "\"schema\":",
    ]
    .iter()
    .map(|key| text.rfind(key).unwrap_or_else(|| panic!("{key} missing")))
    .collect();
    assert!(top.windows(2).all(|pair| pair[0] < pair[1]), "{top:?}");
}

#[test]
fn the_ontology_is_the_projected_revisions_sorted_by_id() {
    let (_, head) = render(5);
    let ontology = &head["ontology"];
    assert_eq!(ids(&ontology["node_types"]), [uuid(0x100), uuid(0x104)]);
    assert_eq!(
        ids(&ontology["node_types"][0]["properties"]),
        [uuid(0x101), uuid(0x106)]
    );
    assert_eq!(
        ids(&ontology["node_types"][1]["properties"]),
        [uuid(0x108), uuid(0x10b)]
    );
    assert_eq!(ids(&ontology["edge_types"]), [uuid(0x105)]);
    let observes = &ontology["edge_types"][0];
    assert_eq!(ids(&observes["source_types"]), [uuid(0x104)]);
    assert_eq!(ids(&observes["target_types"]), [uuid(0x100)]);
    assert_eq!(ids(&observes["properties"]), [uuid(0x109)]);
    assert_eq!(
        ids(&ontology["properties"]),
        [0x101, 0x106, 0x108, 0x109, 0x10b].map(uuid)
    );
    assert_eq!(ontology["properties"][0]["name"], "label");
    assert_eq!(ontology["properties"][0]["value_kind"], "String");

    let (_, seed) = render(0);
    assert_eq!(ids(&seed["ontology"]["node_types"]), [uuid(0x100)]);
    assert_eq!(seed["ontology"]["edge_types"], serde_json::json!([]));
}

#[test]
fn nodes_carry_their_assertions_aliases_in_store_order_and_no_absent_optional() {
    let (text, head) = render(5);
    assert_eq!(
        ids(&head["nodes"]),
        [0x110, 0x111, 0x112, fixtures::DESCRIBED].map(uuid)
    );
    let alpha = format!(
        "{{\"id\":\"{}\",\"name\":\"alpha\",\"type\":\"{}\",\
         \"aliases\":[\"alpha-alias-b\",\"alpha-alias-a\"],\"props\":{{\"{}\":\
         [{{\"kind\":\"String\",\"value\":\"annotated\"}}]}},\"assertions\":[",
        uuid(0x110),
        uuid(0x100),
        uuid(0x106)
    );
    assert!(text.contains(&alpha), "{text}");
    assert!(
        !text.contains("null"),
        "an absent Optional is omitted, never null"
    );
    let claim = &head["nodes"][0]["assertions"][0];
    assert_eq!(claim["id"], uuid(0x130));
    assert_eq!(claim["predicate_kind"], "Property");
    assert_eq!(claim["predicate"], uuid(0x101));
    assert_eq!(claim["object_kind"], "Value");
    assert_eq!(
        claim["object_value"],
        serde_json::json!({"kind": "String", "value": "alpha"})
    );
    assert!(claim.get("object_ref").is_none());
    assert_eq!(claim["valid_from"], 500);
    assert!(claim.get("valid_to").is_none());
    assert_eq!(ids(&claim["evidence"]), [0x120, 0x121, 0x122].map(uuid));
    assert!(text.contains(
        "\"lifecycle\":{\"kind\":\"Retracted\",\"at_revision\":3,\"reason\":\"withdrawn by the operator\"}"
    ));
    assert_eq!(head["nodes"][1]["assertions"], serde_json::json!([]));

    let (_, before) = render(2);
    assert_eq!(
        before["nodes"][0]["assertions"][0]["lifecycle"],
        serde_json::json!({"kind": "Active"})
    );
}

#[test]
fn edges_carry_the_assertions_about_them_and_evidence_is_identity_only() {
    let (text, head) = render(5);
    let edge = &head["edges"][0];
    assert_eq!(edge["id"], uuid(0x141));
    assert_eq!(
        (edge["source"].as_str(), edge["target"].as_str()),
        (Some(uuid(0x112).as_str()), Some(uuid(0x110).as_str()))
    );
    assert_eq!(edge["type"], uuid(0x105));
    assert_eq!(ids(&edge["assertions"]), [uuid(0x133)]);
    assert_eq!(ids(&edge["assertions"][0]["evidence"]), [uuid(0x121)]);

    assert_eq!(ids(&head["evidence"]), [0x120, 0x121, 0x122].map(uuid));
    let first = &head["evidence"][0];
    assert_eq!(first["kind"], "HumanStatement");
    assert_eq!(first["locator"], "operator");
    assert!(first.get("section").is_none());
    assert_eq!(first["observed_at"], 1000);
    assert_eq!(first["confidence_bp"], 9000);
    assert_eq!(first["retained"], true);
    assert_eq!(first["content_hash"].as_str().map(str::len), Some(64));
    assert!(
        !text.contains("synthetic statement"),
        "evidence bytes are never projected"
    );
}

#[test]
fn the_schema_lists_each_version_from_its_first_revision_and_every_revision_up_to_the_projected_one(
) {
    let (_, head) = render(5);
    let versions = head["schema"]["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(
        versions[0],
        serde_json::json!({
            "number": 0, "id": uuid(1), "revision": 0, "added": [uuid(0x100), uuid(0x101)]
        })
    );
    assert_eq!(
        versions[1],
        serde_json::json!({
            "number": 1, "id": uuid(0x107), "parent": uuid(1), "revision": 1,
            "added": ([0x104, 0x105, 0x106, 0x108, 0x109].map(uuid))
        })
    );
    assert_eq!(
        versions[2],
        serde_json::json!({
            "number": 2, "id": uuid(0x10a), "parent": uuid(0x107), "revision": 4, "added": [uuid(0x10b)]
        })
    );
    let revisions = head["schema"]["revisions"].as_array().unwrap();
    assert_eq!(revisions.len(), 6);
    assert!(revisions[0].get("transaction_id").is_none());
    for (n, revision) in revisions.iter().enumerate() {
        assert_eq!(revision["number"], n);
        assert!(revision["committed_at"].as_i64().is_some());
        if n > 0 {
            assert_eq!(revision["transaction_id"], uuid(0x200 + n as u64));
        }
        let version = match n {
            0 => uuid(1),
            1..=3 => uuid(0x107),
            _ => uuid(0x10a),
        };
        assert_eq!(revision["schema_version"], version, "revision {n}");
    }

    let (_, third) = render(3);
    assert_eq!(third["schema"]["versions"].as_array().unwrap().len(), 2);
    assert_eq!(third["schema"]["revisions"].as_array().unwrap().len(), 4);
    assert_eq!(third["meta"]["head"], 5);
}

#[test]
fn an_unseeded_store_and_an_absent_revision_are_refused_by_name() {
    let work = tempfile::tempdir().expect("work directory");
    let empty = fixtures::open(&work.path().join("empty"), Provider::Sqlite);
    for requested in [
        None,
        Some(RevisionNumber::new(0)),
        Some(RevisionNumber::new(3)),
    ] {
        match ekr_views::project(&empty, requested) {
            Err(ProjectError::NotSeeded { requested: echoed }) => assert_eq!(echoed, requested),
            other => panic!("{requested:?}: {other:?}"),
        }
    }
    let seeded = fixtures::open(&work.path().join("seeded"), Provider::Sqlite);
    Fixture::SeedOnly.build(&seeded);
    match ekr_views::project(&seeded, Some(RevisionNumber::new(1))) {
        Err(ProjectError::RevisionNotFound { requested, head }) => {
            assert_eq!((requested.get(), head.get()), (1, 0));
        }
        other => panic!("{other:?}"),
    }
    let _: ekr_core::NodeId = id(0x110);
}
