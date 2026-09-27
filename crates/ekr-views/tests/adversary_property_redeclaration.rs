//! Adversary, story:graph-projection-renderer, pass 1, finding F1, with the correction the
//! coordinator decided.
//!
//! The ontology admits one property id declared by two types with two different definitions: a
//! child type redeclares a property its parent declares (`ekr-ontology`
//! `tests/inheritance_and_declaration_coherence.rs`, grand `String` <- parent `Integer`). The
//! projection's `ontology.properties` is the one place a property definition is spelled out, one
//! entry per id, and each type's `properties` names ids into it. Keeping either definition tells
//! a reader of the other type the wrong one, so the render refuses with
//! `ProjectError::Inconsistent`, naming the property and both types.
//!
//! **This refusal is interim.** task:projection-carries-per-type-property-definitions gives each
//! type its own definition in the format. When it lands, the first case must assert that each
//! type's own definition is projected, not that the render refuses.

mod support;

use std::path::Path;

use ekr_core::{PropertyId, Timestamp, TypeId};
use ekr_kernel::SeedDocument;
use ekr_ontology::{NodeType, PropertyDefinition, ValueType};
use ekr_views::ProjectError;
use serde_json::Value;

use support::fixtures::{self, id, Provider};

const BASE: u64 = 0x300;
const REFINED: u64 = 0x301;
const MEASURE: u64 = 0x302;

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

fn seeded(refined: ValueType) -> (tempfile::TempDir, ekr_kernel::Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    runtime
        .seed(seed_with(refined), || {
            Timestamp::from_millis(1_800_000_000_001)
        })
        .expect("the ontology admits a child type declaring its parent's property");
    (work, runtime)
}

#[test]
fn a_property_redeclared_by_a_child_type_with_another_definition_is_refused_by_name() {
    let (_work, runtime) = seeded(ValueType::Integer);
    match ekr_views::project(&runtime, None) {
        Err(ProjectError::Inconsistent(message)) => {
            for named in [uuid(MEASURE), uuid(BASE), uuid(REFINED)] {
                assert!(message.contains(&named), "{message} does not name {named}");
            }
            assert!(
                message.contains("task:projection-carries-per-type-property-definitions"),
                "{message}"
            );
        }
        other => panic!(
            "Refined redeclares `measure` as Integer where Base declares String, and the format \
             carries one definition per id: the render must refuse, not keep one silently. \
             Interim until task:projection-carries-per-type-property-definitions lands; then \
             this case asserts each type's own definition instead. Got {other:?}"
        ),
    }
}

/// The other member of the class: a node type and an edge type declaring one property id with
/// two definitions collapse into the same single entry, and are refused the same way.
#[test]
fn a_property_a_node_type_and_an_edge_type_declare_differently_is_refused_by_name() {
    const LINKS: u64 = 0x303;
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
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::Sqlite);
    runtime
        .seed(document, || Timestamp::from_millis(1_800_000_000_001))
        .expect("the ontology admits an edge type declaring a node type's property id");
    match ekr_views::project(&runtime, None) {
        Err(ProjectError::Inconsistent(message)) => {
            for named in [uuid(MEASURE), uuid(LINKS)] {
                assert!(message.contains(&named), "{message} does not name {named}");
            }
        }
        other => panic!("one id, two definitions across a node and an edge type: {other:?}"),
    }
}

#[test]
fn a_property_two_types_declare_identically_renders_once() {
    let (_work, runtime) = seeded(ValueType::String);
    let rendered = ekr_views::project(&runtime, None).expect("identical definitions render");
    let document: Value = serde_json::from_slice(&rendered.bytes).expect("JSON");
    let measure = Value::String(uuid(MEASURE));
    for declared in document["ontology"]["node_types"]
        .as_array()
        .expect("node_types")
    {
        assert!(
            declared["properties"]
                .as_array()
                .expect("properties")
                .contains(&measure),
            "{} lists `measure`",
            declared["name"]
        );
    }
    let entries: Vec<&Value> = document["ontology"]["properties"]
        .as_array()
        .expect("ontology.properties")
        .iter()
        .filter(|property| property["id"] == measure)
        .collect();
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0]["value_kind"], "String");
    assert_eq!(rendered.summary.properties, 1);
}
