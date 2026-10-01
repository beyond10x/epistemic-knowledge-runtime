//! Adversary, wave extract-06 unit J (task:projection-carries-per-type-property-definitions).
//!
//! The unit gives `ekr.graph-projection/1`'s `ontology.properties` one entry per definition of a
//! property id, naming in `owners` the types whose declaration each entry is. `views.yaml`
//! (`ekr.views.ProjectedProperty`) tells a reader how to use it: "A reader takes type T's
//! definition of property P from the entry with id P whose `owners` holds T, else from P's only
//! entry." These cases drive the projection against that rule over the redeclaration patterns
//! the ontology admits.

mod support;

use std::path::Path;

use ekr_core::{NodeId, PropertyId, SchemaVersionId, Timestamp, TransactionId, TypeId};
use ekr_graph::Node;
use ekr_kernel::{
    CommitCommandResult, GraphOperation, GraphTransaction, PropertyModification, Runtime,
    SeedDocument, ValidationCommandResult,
};
use ekr_ontology::{EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use ekr_views::{Index, OverviewRequest};
use serde::Serialize;
use serde_json::{json, Value as Json};

use support::fixtures::{self, context, id, Provider};

const BASE: u64 = 0x700;
const MID: u64 = 0x701;
const LEAF: u64 = 0x702;
const OTHER: u64 = 0x703;
const MEASURE: u64 = 0x710;
const LEAF_NODE: u64 = 0x720;
const UNIFIED_VERSION: u64 = 0x730;

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn empty_seed() -> SeedDocument {
    SeedDocument::from_yaml(
        &std::fs::read_to_string(
            Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir"))
                .join("tests/fixtures/seed-empty.yaml"),
        )
        .expect("the empty seed fixture"),
    )
    .expect("the empty seed parses")
}

fn node_type(n: u64, name: &str, parents: &[u64], declares: Option<(&str, ValueType)>) -> NodeType {
    let mut declared = NodeType::new(id::<TypeId>(n), name);
    for parent in parents {
        declared.parents.insert(id(*parent));
    }
    if let Some((property, value_type)) = declares {
        let measure: PropertyId = id(MEASURE);
        declared.properties.insert(
            measure,
            PropertyDefinition::new(measure, property, value_type),
        );
    }
    declared
}

fn seeded(document: SeedDocument) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    runtime
        .seed(document, || Timestamp::from_millis(1_800_000_000_001))
        .expect("the ontology admits the seed");
    (work, runtime)
}

fn projected(runtime: &Runtime, revision: Option<u64>) -> Json {
    let rendered = ekr_views::project(runtime, revision.map(ekr_core::RevisionNumber::new))
        .unwrap_or_else(|error| panic!("the revision renders: {error}"));
    serde_json::from_slice(&rendered.bytes).expect("JSON")
}

fn entries(document: &Json) -> Vec<Json> {
    document["ontology"]["properties"]
        .as_array()
        .expect("ontology.properties")
        .iter()
        .filter(|entry| entry["id"] == Json::String(uuid(MEASURE)))
        .cloned()
        .collect()
}

/// `views.yaml`'s reader rule for type `owner`'s definition of `measure`: the entry whose
/// `owners` holds it, else the id's only entry. `None` when the rule names no entry.
fn definition_for(document: &Json, owner: u64) -> Option<(String, String)> {
    let all = entries(document);
    let owned: Vec<&Json> = all
        .iter()
        .filter(|entry| {
            entry["owners"]
                .as_array()
                .is_some_and(|owners| owners.contains(&Json::String(uuid(owner))))
        })
        .collect();
    let chosen = match (owned.as_slice(), all.as_slice()) {
        ([one], _) => *one,
        ([], [only]) => only,
        _ => return None,
    };
    Some((
        chosen["name"].as_str()?.to_owned(),
        chosen["value_kind"].as_str()?.to_owned(),
    ))
}

/// A three-level chain: `Base` declares `measure` as String, `Mid` (child of `Base`) redeclares
/// it as the Integer `reading`, and `Leaf` (child of `Mid`) declares nothing and inherits Mid's
/// definition, which the kernel resolves for it (`Ontology::properties_of`). A `Leaf` node holds
/// the Integer 7 under that id. The projection lists two entries — `owners: [Base]` and
/// `owners: [Mid]` — and `Leaf` is in neither, so the documented reader rule names no definition
/// for `Leaf`, and the projection carries no `parents` from which a reader could find one. The
/// value 7 under the `Leaf` node cannot be named from the document that holds it.
#[test]
#[ignore = "adversary x6-j F1: an inheriting subtype of a redeclaring type is in no entry's owners; the projection cannot name its property"]
fn an_inheriting_subtype_of_a_redeclaring_type_has_a_definition_the_projection_names() {
    let mut document = empty_seed();
    document.ontology.node_types.push(node_type(
        BASE,
        "Base",
        &[],
        Some(("measure", ValueType::String)),
    ));
    document.ontology.node_types.push(node_type(
        MID,
        "Mid",
        &[BASE],
        Some(("reading", ValueType::Integer)),
    ));
    document
        .ontology
        .node_types
        .push(node_type(LEAF, "Leaf", &[MID], None));
    let mut leaf = Node::<Value>::new(
        id::<NodeId>(LEAF_NODE),
        document.graph.root.id,
        id(LEAF),
        "leaf one",
    );
    leaf.properties.insert(id(MEASURE), vec![Value::Integer(7)]);
    document.graph.nodes.insert(leaf.id, leaf);
    let (_work, runtime) = seeded(document);
    let projection = projected(&runtime, None);

    // The kernel's own resolution, for the record: Leaf's definition is Mid's.
    let loaded = Index::load(&runtime, None).expect("the head loads");
    let resolved = loaded.loaded().graph.ontology.properties_of(id(LEAF));
    let mid = resolved
        .get(&id::<PropertyId>(MEASURE))
        .expect("Leaf inherits measure");
    assert_eq!(
        (
            mid.name.as_str(),
            mid.value_type.kind().to_string().as_str()
        ),
        ("reading", "Integer")
    );

    assert_eq!(
        definition_for(&projection, LEAF),
        Some(("reading".to_owned(), "Integer".to_owned())),
        "Leaf's definition of measure, by views.yaml's reader rule, from entries {:?}",
        entries(&projection)
    );
}

/// A diamond: `Base` String `measure`; `Left` and `Right` both children of `Base` redeclaring it
/// differently; `Bottom` a child of both, redeclaring it again (without which the ontology
/// refuses the ambiguity). Every declaring type gets exactly its own definition.
#[test]
fn a_diamond_with_two_differing_parents_and_a_resolving_child_projects_each_definition() {
    const LEFT: u64 = MID;
    const RIGHT: u64 = LEAF;
    const BOTTOM: u64 = OTHER;
    let mut document = empty_seed();
    document.ontology.node_types.push(node_type(
        BASE,
        "Base",
        &[],
        Some(("measure", ValueType::String)),
    ));
    document.ontology.node_types.push(node_type(
        LEFT,
        "Left",
        &[BASE],
        Some(("left", ValueType::Integer)),
    ));
    document.ontology.node_types.push(node_type(
        RIGHT,
        "Right",
        &[BASE],
        Some(("right", ValueType::Integer)),
    ));
    document.ontology.node_types.push(node_type(
        BOTTOM,
        "Bottom",
        &[LEFT, RIGHT],
        Some(("left", ValueType::Integer)),
    ));
    let (_work, runtime) = seeded(document);
    let projection = projected(&runtime, None);
    assert_eq!(
        entries(&projection),
        vec![
            json!({"id": uuid(MEASURE), "name": "measure", "value_kind": "String", "owners": [uuid(BASE)]}),
            json!({"id": uuid(MEASURE), "name": "left", "value_kind": "Integer", "owners": [uuid(LEFT), uuid(BOTTOM)]}),
            json!({"id": uuid(MEASURE), "name": "right", "value_kind": "Integer", "owners": [uuid(RIGHT)]}),
        ]
    );
    for (owner, expected) in [
        (BASE, ("measure", "String")),
        (LEFT, ("left", "Integer")),
        (RIGHT, ("right", "Integer")),
        (BOTTOM, ("left", "Integer")),
    ] {
        assert_eq!(
            definition_for(&projection, owner),
            Some((expected.0.to_owned(), expected.1.to_owned()))
        );
    }
}

/// A redeclaration that changes only the name, one that changes only the value kind, and one
/// that changes only `required` (which the entry does not carry), on an abstract parent.
#[test]
fn name_only_kind_only_and_required_only_redeclarations_on_an_abstract_parent() {
    let cases: [(&str, ValueType, bool, Json); 3] = [
        (
            "reading",
            ValueType::String,
            false,
            json!([
                {"id": uuid(MEASURE), "name": "measure", "value_kind": "String", "owners": [uuid(BASE)]},
                {"id": uuid(MEASURE), "name": "reading", "value_kind": "String", "owners": [uuid(MID)]},
            ]),
        ),
        (
            "measure",
            ValueType::Boolean,
            false,
            json!([
                {"id": uuid(MEASURE), "name": "measure", "value_kind": "String", "owners": [uuid(BASE)]},
                {"id": uuid(MEASURE), "name": "measure", "value_kind": "Boolean", "owners": [uuid(MID)]},
            ]),
        ),
        (
            "measure",
            ValueType::String,
            true,
            json!([{"id": uuid(MEASURE), "name": "measure", "value_kind": "String"}]),
        ),
    ];
    for (name, value_type, required, expected) in cases {
        let mut document = empty_seed();
        let mut base = node_type(BASE, "Base", &[], Some(("measure", ValueType::String)));
        base.abstract_type = true;
        document.ontology.node_types.push(base);
        let mut mid = node_type(MID, "Mid", &[BASE], Some((name, value_type)));
        mid.properties
            .get_mut(&id::<PropertyId>(MEASURE))
            .expect("declared")
            .required = required;
        document.ontology.node_types.push(mid);
        let (_work, runtime) = seeded(document);
        assert_eq!(
            Json::Array(entries(&projected(&runtime, None))),
            expected,
            "{name} {required}"
        );
    }
}

/// The order of `ontology.properties` does not follow declaration order: the same ontology with
/// its types declared in reverse, and with an edge type whose id sorts before every node type
/// sharing a definition, renders the same bytes.
#[test]
fn per_type_entries_do_not_depend_on_declaration_order() {
    const EDGE: u64 = 0x6ff;
    let build = |reversed: bool| {
        let mut document = empty_seed();
        let mut types = vec![
            node_type(BASE, "Base", &[], Some(("measure", ValueType::String))),
            node_type(MID, "Mid", &[BASE], Some(("reading", ValueType::Integer))),
            node_type(OTHER, "Other", &[], Some(("reading", ValueType::Integer))),
        ];
        if reversed {
            types.reverse();
        }
        document.ontology.node_types = types;
        let measure: PropertyId = id(MEASURE);
        let mut edge = EdgeType::new(id::<TypeId>(EDGE), "edge");
        edge.source_types.insert(id(BASE));
        edge.target_types.insert(id(BASE));
        edge.properties.insert(
            measure,
            PropertyDefinition::new(measure, "reading", ValueType::Integer),
        );
        document.ontology.edge_types.push(edge);
        let (_work, runtime) = seeded(document);
        ekr_views::project(&runtime, None).expect("renders").bytes
    };
    let (forward, backward) = (build(false), build(true));
    assert_eq!(forward, backward);
    let document: Json = serde_json::from_slice(&forward).expect("JSON");
    assert_eq!(
        entries(&document),
        vec![
            json!({"id": uuid(MEASURE), "name": "reading", "value_kind": "Integer",
                   "owners": [uuid(EDGE), uuid(MID), uuid(OTHER)]}),
            json!({"id": uuid(MEASURE), "name": "measure", "value_kind": "String", "owners": [uuid(BASE)]}),
        ]
    );
}

fn encode(transaction: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction,
    })
    .expect("a transaction document")
    .into_bytes()
}

fn commit(runtime: &Runtime, n: u64, operations: Vec<GraphOperation>, version: u64, clock: i64) {
    let transaction = GraphTransaction {
        id: id::<TransactionId>(0x7f0 + n),
        proposer: context().operator,
        operations,
        evidence: Default::default(),
        schema_version: Some(id::<SchemaVersionId>(version)),
    };
    runtime
        .propose(&encode(&transaction), context().operator, || {
            Timestamp::from_millis(clock)
        })
        .expect("proposed");
    let head = runtime.head().expect("head").expect("seeded").revision;
    let verdict = runtime
        .validate(transaction.id, head, || Timestamp::from_millis(clock + 1))
        .expect("validation runs");
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let result = runtime
        .commit(transaction.id, context().operator, || {
            Timestamp::from_millis(clock + 2)
        })
        .expect("commit runs");
    assert!(
        matches!(result, CommitCommandResult::Committed(_)),
        "{result:?}"
    );
}

/// A subtype redeclares, then a later revision unifies it with its parent again: the redeclaring
/// revision has two entries with owners, the unified one a single entry and no `owners`, and the
/// overview's ontology at each revision is the projection's.
#[test]
fn a_redeclaration_unified_again_returns_to_one_entry_without_owners() {
    let mut document = empty_seed();
    document.ontology.node_types.push(node_type(
        BASE,
        "Base",
        &[],
        Some(("measure", ValueType::String)),
    ));
    document.ontology.node_types.push(node_type(
        MID,
        "Mid",
        &[BASE],
        Some(("reading", ValueType::Integer)),
    ));
    let (_work, runtime) = seeded(document);
    commit(
        &runtime,
        1,
        vec![GraphOperation::ModifyProperty(PropertyModification {
            owner: Some(id(MID)),
            property: PropertyDefinition::new(id(MEASURE), "measure", ValueType::String),
        })],
        UNIFIED_VERSION,
        1_800_000_000_100,
    );
    let before = projected(&runtime, Some(0));
    let after = projected(&runtime, Some(1));
    assert_eq!(
        entries(&before),
        vec![
            json!({"id": uuid(MEASURE), "name": "measure", "value_kind": "String", "owners": [uuid(BASE)]}),
            json!({"id": uuid(MEASURE), "name": "reading", "value_kind": "Integer", "owners": [uuid(MID)]}),
        ]
    );
    assert_eq!(
        entries(&after),
        vec![json!({"id": uuid(MEASURE), "name": "measure", "value_kind": "String"})]
    );
    for (at, projection) in [(0, &before), (1, &after)] {
        let overview = Index::load(&runtime, Some(ekr_core::RevisionNumber::new(at)))
            .expect("loads")
            .overview(&OverviewRequest::new(None).expect("default limit"))
            .expect("the overview answers");
        let overview: Json = serde_json::from_slice(&overview.bytes).expect("JSON");
        assert_eq!(
            overview["ontology"], projection["ontology"],
            "revision {at}"
        );
    }
}
