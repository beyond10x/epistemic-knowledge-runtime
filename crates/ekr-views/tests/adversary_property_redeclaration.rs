//! Adversary, story:graph-projection-renderer, pass 1, finding F1, as
//! task:projection-carries-per-type-property-definitions closes it.
//!
//! The ontology admits one property id declared by two types with two different definitions: a
//! child type redeclares a property its parent declares (`ekr-ontology`
//! `tests/inheritance_and_declaration_coherence.rs`, grand `String` <- parent `Integer`). Until
//! the task landed, the projection's `ontology.properties` held one entry per id and the render
//! refused such a revision with `ProjectError::Inconsistent`. It now holds one entry per distinct
//! definition of an id; when an id has more than one, each entry's `owners` names the types whose
//! declaration it is, so each type's own definition is projected. An id every declaring type
//! defines alike keeps its single entry and carries no `owners`, so the bytes of every revision
//! the earlier renderer rendered are unchanged.

mod support;

use std::path::Path;

use ekr_core::{PropertyId, Timestamp, TypeId};
use ekr_kernel::SeedDocument;
use ekr_ontology::{NodeType, PropertyDefinition, ValueType};
use ekr_views::{Index, OverviewRequest};
use serde_json::{json, Value};

use support::fixtures::{self, id, Provider};

const BASE: u64 = 0x300;
const REFINED: u64 = 0x301;
const MEASURE: u64 = 0x302;
const LINKS: u64 = 0x303;

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

/// `Base` declares `measure` as String; `Refined`, a child of `Base`, declares it as `refined`.
fn seed_with(refined: ValueType) -> SeedDocument {
    let mut document = SeedDocument::from_yaml(
        &std::fs::read_to_string(
            Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"))
                .join("tests/fixtures/seed-empty.yaml"),
        )
        .expect("the empty seed fixture"),
    )
    .expect("the empty seed parses");
    let measure: PropertyId = id(MEASURE);

    let mut base = NodeType::new(id::<TypeId>(BASE), "Base");
    base.properties.insert(
        measure,
        PropertyDefinition::new(measure, "measure", ValueType::String),
    );
    let mut child = NodeType::new(id::<TypeId>(REFINED), "Refined");
    child.parents.insert(id(BASE));
    child.properties.insert(
        measure,
        PropertyDefinition::new(measure, "measure", refined),
    );
    document.ontology.node_types.push(base);
    document.ontology.node_types.push(child);
    document
}

fn seeded(document: SeedDocument, provider: Provider) -> (tempfile::TempDir, ekr_kernel::Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    runtime
        .seed(document, || Timestamp::from_millis(1_800_000_000_001))
        .expect("the ontology admits a child type declaring its parent's property");
    (work, runtime)
}

/// The `ontology.properties` entries for `measure`, in document order.
fn measure_entries(document: &Value) -> Vec<Value> {
    document["ontology"]["properties"]
        .as_array()
        .expect("ontology.properties")
        .iter()
        .filter(|property| property["id"] == Value::String(uuid(MEASURE)))
        .cloned()
        .collect()
}

/// Every type in `document` that lists `measure` among its properties.
fn listing_measure(document: &Value) -> Vec<String> {
    let measure = Value::String(uuid(MEASURE));
    ["node_types", "edge_types"]
        .iter()
        .flat_map(|kind| {
            document["ontology"][kind]
                .as_array()
                .expect("types")
                .iter()
                .filter(|declared| {
                    declared["properties"]
                        .as_array()
                        .expect("properties")
                        .contains(&measure)
                })
                .map(|declared| declared["id"].as_str().expect("id").to_owned())
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn a_property_redeclared_by_a_child_type_is_projected_with_each_types_own_definition() {
    for provider in [Provider::File, Provider::Sqlite] {
        let (_work, runtime) = seeded(seed_with(ValueType::Integer), provider);
        let rendered = ekr_views::project(&runtime, None).unwrap_or_else(|error| {
            panic!(
                "Refined redeclares `measure` as Integer where Base declares String; the format \
                 gives each type its own definition, so the render must not refuse: {error}"
            )
        });
        let document: Value = serde_json::from_slice(&rendered.bytes).expect("JSON");
        assert_eq!(
            measure_entries(&document),
            vec![
                json!({"id": uuid(MEASURE), "name": "measure", "value_kind": "String",
                       "owners": [uuid(BASE)]}),
                json!({"id": uuid(MEASURE), "name": "measure", "value_kind": "Integer",
                       "owners": [uuid(REFINED)]}),
            ],
            "{provider:?}: one entry per definition, each naming the type it is the declaration of"
        );
        assert_eq!(
            listing_measure(&document),
            vec![uuid(BASE), uuid(REFINED)],
            "{provider:?}: both types still list the property by id"
        );
        assert_eq!(rendered.summary.properties, 2, "{provider:?}");

        // The overview the viewer reads carries the same ontology, so it no longer refuses either.
        let index = Index::load(&runtime, None).expect("the revision loads");
        let overview = index
            .overview(&OverviewRequest::new(None).expect("default limit"))
            .unwrap_or_else(|error| panic!("{provider:?}: the overview must not refuse: {error}"));
        let overview: Value = serde_json::from_slice(&overview.bytes).expect("JSON");
        assert_eq!(
            overview["ontology"], document["ontology"],
            "{provider:?}: the overview's ontology is the projection's"
        );
    }
}

/// The other member of the class: a node type and an edge type declaring one property id with
/// two definitions. The two node types that agree share one entry, and the edge type has its own.
#[test]
fn a_property_a_node_type_and_an_edge_type_declare_differently_is_projected_for_each() {
    let mut document = seed_with(ValueType::String);
    let measure: PropertyId = id(MEASURE);
    let mut links = ekr_ontology::EdgeType::new(id::<TypeId>(LINKS), "links");
    links.source_types.insert(id(BASE));
    links.target_types.insert(id(BASE));
    links.properties.insert(
        measure,
        PropertyDefinition::new(measure, "measure", ValueType::Integer),
    );
    document.ontology.edge_types.push(links);
    let (_work, runtime) = seeded(document, Provider::Sqlite);
    let rendered = ekr_views::project(&runtime, None)
        .unwrap_or_else(|error| panic!("one id, a node and an edge type's definitions: {error}"));
    let document: Value = serde_json::from_slice(&rendered.bytes).expect("JSON");
    assert_eq!(
        measure_entries(&document),
        vec![
            json!({"id": uuid(MEASURE), "name": "measure", "value_kind": "String",
                   "owners": [uuid(BASE), uuid(REFINED)]}),
            json!({"id": uuid(MEASURE), "name": "measure", "value_kind": "Integer",
                   "owners": [uuid(LINKS)]}),
        ]
    );
    assert_eq!(
        listing_measure(&document),
        vec![uuid(BASE), uuid(REFINED), uuid(LINKS)]
    );
}

#[test]
fn a_property_two_types_declare_identically_renders_once_without_owners() {
    let (_work, runtime) = seeded(seed_with(ValueType::String), Provider::File);
    let rendered = ekr_views::project(&runtime, None).expect("identical definitions render");
    let document: Value = serde_json::from_slice(&rendered.bytes).expect("JSON");
    assert_eq!(listing_measure(&document), vec![uuid(BASE), uuid(REFINED)]);
    assert_eq!(
        measure_entries(&document),
        vec![json!({"id": uuid(MEASURE), "name": "measure", "value_kind": "String"})],
        "an id every declaring type defines alike has one entry and no `owners`"
    );
    assert_eq!(rendered.summary.properties, 1);
}

/// The store `property-redeclared` that
/// `a-property-a-subtype-redeclares-is-projected-for-each-type.yaml` names, read directly: schema
/// evolution — `ModifyProperty` on a new child type — produces the redeclaration, the head
/// projects one entry per definition and its overview answers, and revision 0 projects as before.
#[test]
fn the_redeclaring_schema_change_projects_per_type_from_its_revision_only() {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    fixtures::Fixture::named("property-redeclared")
        .expect("the scenario's store")
        .build(&runtime);
    let label = uuid(0x101);
    let head = ekr_views::project(&runtime, None)
        .unwrap_or_else(|error| panic!("the redeclaring revision renders: {error}"));
    assert_eq!(
        (head.summary.revision, head.summary.schema_versions),
        (1, 2)
    );
    assert_eq!(
        (
            head.summary.node_types,
            head.summary.edge_types,
            head.summary.properties
        ),
        (2, 0, 2)
    );
    let document: Value = serde_json::from_slice(&head.bytes).expect("JSON");
    let entries: Vec<&Value> = document["ontology"]["properties"]
        .as_array()
        .expect("ontology.properties")
        .iter()
        .filter(|entry| entry["id"] == Value::String(label.clone()))
        .collect();
    assert_eq!(
        entries,
        vec![
            &json!({"id": label, "name": "label", "value_kind": "String", "owners": [uuid(0x100)]}),
            &json!({"id": label, "name": "rank", "value_kind": "Integer",
                    "owners": [uuid(fixtures::REFINED_TYPE)]}),
        ]
    );
    let overview = Index::load(&runtime, None)
        .expect("the head loads")
        .overview(&OverviewRequest::new(None).expect("default limit"))
        .unwrap_or_else(|error| panic!("the overview of the redeclaring revision: {error}"));
    assert_eq!(
        (overview.summary.revision, overview.summary.node_types),
        (1, 2)
    );

    let before = ekr_views::project(&runtime, Some(ekr_core::RevisionNumber::new(0)))
        .expect("revision 0 renders");
    assert_eq!(
        (before.summary.node_types, before.summary.properties),
        (1, 1)
    );
    let document: Value = serde_json::from_slice(&before.bytes).expect("JSON");
    assert_eq!(
        document["ontology"]["properties"],
        json!([{"id": label, "name": "label", "value_kind": "String"}])
    );
}
