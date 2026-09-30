//! Adversary, pass 1, `task:lineage-shows-widened-ends-and-modified-properties`.
//!
//! The unit's own fixture changes one thing per version and exercises `Name`, `Cardinality` and
//! `Declared` only. These cases drive `ekr.views.OverviewSchemaVersion` (`views.yaml`) with every
//! schema operation at once, and compare each version's `added`, `removed`, `widened` and
//! `modified` with a diff of the version's ontology against its parent's computed here, from the
//! ontologies `ekr_views::load` reads.
//!
//! `views.yaml` also states that a committed version lists nothing in all four exactly when its
//! ontology is its parent's; `Ontology::evolve` refuses a change without effect, so every committed
//! version after the seed must list at least one entry.

mod support;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ekr_core::{PropertyId, SchemaVersionId, Timestamp, TransactionId, TypeId};
use ekr_kernel::{
    CommitCommandResult, EdgeWidening, GraphOperation, GraphTransaction, PropertyModification,
    Runtime, SeedDocument, ValidationCommandResult,
};
use ekr_ontology::{
    Cardinality, EdgeType, NodeType, OntologyDocument, PropertyDefinition, ValueType,
};
use serde::Serialize;
use serde_json::{json, Value};

use support::fixtures::{self, context, id, Provider};

const ALPHA: u64 = 0x600;
const BETA: u64 = 0x601;
const GAMMA: u64 = 0x602;
const DELTA: u64 = 0x603;
const E: u64 = 0x610;
const F: u64 = 0x611;
const G: u64 = 0x612;
const H: u64 = 0x613;
const P1: u64 = 0x620;
const P2: u64 = 0x621;
const P3: u64 = 0x622;
const P4: u64 = 0x623;
const P5: u64 = 0x624;
const P6: u64 = 0x625;
const P7: u64 = 0x626;
const Q: u64 = 0x630;
const V1: u64 = 0x6f1;
const V2: u64 = 0x6f2;

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn string(n: u64, name: &str) -> PropertyDefinition {
    PropertyDefinition::new(id(n), name, ValueType::String)
}

fn edge(n: u64, name: &str) -> EdgeType {
    let mut declared = EdgeType::new(id(n), name);
    declared.source_types = [id(ALPHA)].into_iter().collect();
    declared.target_types = [id(ALPHA)].into_iter().collect();
    declared
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

/// Commits through propose, validate and commit, as `support::fixtures`' private writer does.
struct Writer<'a> {
    runtime: &'a Runtime,
    clock: i64,
    transactions: u64,
}

impl Writer<'_> {
    fn tick(&mut self) -> Timestamp {
        self.clock += 1;
        Timestamp::from_millis(self.clock)
    }

    fn seed(&mut self, document: SeedDocument) {
        let now = self.tick();
        self.runtime
            .seed(document, || now)
            .expect("the seed is admitted");
    }

    fn commit(&mut self, operations: Vec<GraphOperation>, version: u64) {
        #[derive(Serialize)]
        struct Wire<'a> {
            format: &'static str,
            transaction: &'a GraphTransaction,
        }
        self.transactions += 1;
        let transaction = GraphTransaction {
            id: id::<TransactionId>(0x6e00 + self.transactions),
            proposer: context().operator,
            operations,
            evidence: BTreeSet::new(),
            schema_version: Some(id::<SchemaVersionId>(version)),
        };
        let bytes = serde_yaml_ng::to_string(&Wire {
            format: "ekr.transaction-document/1",
            transaction: &transaction,
        })
        .expect("a transaction document")
        .into_bytes();
        let (proposed, validated, committed) = (self.tick(), self.tick(), self.tick());
        self.runtime
            .propose(&bytes, context().operator, || proposed)
            .expect("the proposal is retained");
        let head = self.runtime.head().expect("head").expect("seeded").revision;
        let verdict = self
            .runtime
            .validate(transaction.id, head, || validated)
            .expect("validation runs");
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "the transaction validates: {verdict:?}"
        );
        let result = self
            .runtime
            .commit(transaction.id, context().operator, || committed)
            .expect("commit runs");
        assert!(
            matches!(result, CommitCommandResult::Committed(_)),
            "{result:?}"
        );
    }
}

fn modify(owner: u64, property: PropertyDefinition) -> GraphOperation {
    GraphOperation::ModifyProperty(PropertyModification {
        owner: Some(id(owner)),
        property,
    })
}

/// The seed: `Alpha` with `one` to `five` and `seven`, `Beta`, `Gamma`; edge types `e` (with
/// `q`), `f` and `g`, each `Alpha` → `Alpha`. Version 1, one commit: every schema operation.
fn build_every_operation(runtime: &Runtime) {
    let mut document = empty_seed();
    let mut alpha = NodeType::new(id(ALPHA), "Alpha");
    for (n, name) in [
        (P1, "one"),
        (P2, "two"),
        (P3, "three"),
        (P4, "four"),
        (P5, "five"),
        (P7, "seven"),
    ] {
        alpha.properties.insert(id(n), string(n, name));
    }
    document.ontology.node_types.push(alpha);
    document
        .ontology
        .node_types
        .push(NodeType::new(id(BETA), "Beta"));
    document
        .ontology
        .node_types
        .push(NodeType::new(id(GAMMA), "Gamma"));
    let mut e = edge(E, "e");
    e.properties.insert(id(Q), string(Q, "q"));
    document.ontology.edge_types.push(e);
    document.ontology.edge_types.push(edge(F, "f"));
    document.ontology.edge_types.push(edge(G, "g"));

    let mut writer = Writer {
        runtime,
        clock: fixtures::CLOCK_START_MS,
        transactions: 0,
    };
    writer.seed(document);

    let mut h = EdgeType::new(id(H), "h");
    h.source_types = [id(ALPHA)].into_iter().collect();
    h.target_types = [id(BETA)].into_iter().collect();
    let mut two = string(P2, "two");
    two.value_type = ValueType::Integer;
    let mut three = string(P3, "three");
    three.cardinality = Cardinality::Many;
    let mut four = string(P4, "four");
    four.required = true;
    let mut five = string(P5, "five");
    five.constraints = vec!["opaque".into()];
    let mut seven = string(P7, "seven-b");
    seven.value_type = ValueType::Integer;
    seven.cardinality = Cardinality::Many;
    seven.required = true;
    seven.constraints = vec!["opaque".into()];
    let mut q = string(Q, "q");
    q.value_type = ValueType::Integer;
    let set = |ids: &[u64]| {
        ids.iter()
            .map(|n| id::<TypeId>(*n))
            .collect::<BTreeSet<_>>()
    };
    writer.commit(
        vec![
            GraphOperation::DefineNodeType(Box::new(NodeType::new(id(DELTA), "Delta"))),
            GraphOperation::DefineEdgeType(Box::new(h)),
            modify(ALPHA, string(P1, "one-renamed")),
            modify(ALPHA, two),
            modify(ALPHA, three),
            modify(ALPHA, four),
            modify(ALPHA, five),
            modify(ALPHA, seven),
            modify(E, q),
            modify(BETA, string(P1, "one-renamed")),
            modify(GAMMA, string(P6, "six")),
            GraphOperation::WidenEdgeType(EdgeWidening {
                edge_type: id(E),
                source_types: set(&[ALPHA, BETA]),
                target_types: set(&[ALPHA]),
            }),
            GraphOperation::WidenEdgeType(EdgeWidening {
                edge_type: id(F),
                source_types: set(&[ALPHA]),
                target_types: set(&[ALPHA, GAMMA]),
            }),
            GraphOperation::WidenEdgeType(EdgeWidening {
                edge_type: id(G),
                source_types: set(&[ALPHA, BETA, GAMMA]),
                target_types: set(&[ALPHA, BETA, GAMMA]),
            }),
        ],
        V1,
    );
}

fn overview(runtime: &Runtime) -> (Vec<u8>, ekr_views::GraphOverviewed) {
    let index = ekr_views::Index::load(runtime, None).expect("the index loads");
    let answer = index
        .overview(
            &ekr_views::OverviewRequest::new(Some(ekr_views::OverviewRequest::MAX_LIMIT))
                .expect("the largest limit"),
        )
        .expect("the overview answers");
    (answer.bytes, answer.summary)
}

/// Every version's lineage lists, written independently of `index.rs` from the two ontology
/// documents, as `views.yaml` specifies them.
fn expected(parent: &OntologyDocument, version: &OntologyDocument) -> Value {
    /// Members by id, then kind in `ekr.views.SchemaMemberKind`'s order: a member is its kind and
    /// its id together, so one UUID that is a type's and a property's is two members.
    fn ids(document: &OntologyDocument) -> BTreeMap<(String, u8), String> {
        let mut out = BTreeMap::new();
        for t in &document.node_types {
            out.insert((t.id.to_string(), 0), t.name.clone());
        }
        for t in &document.edge_types {
            out.insert((t.id.to_string(), 1), t.name.clone());
        }
        for p in document
            .node_types
            .iter()
            .flat_map(|t| t.properties.values())
            .chain(
                document
                    .edge_types
                    .iter()
                    .flat_map(|t| t.properties.values()),
            )
        {
            out.entry((p.id.to_string(), 2)).or_insert(p.name.clone());
        }
        out
    }
    fn declarations(document: &OntologyDocument) -> BTreeMap<(String, String), PropertyDefinition> {
        let mut out = BTreeMap::new();
        for (owner, properties) in document
            .node_types
            .iter()
            .map(|t| (t.id, &t.properties))
            .chain(document.edge_types.iter().map(|t| (t.id, &t.properties)))
        {
            for (property, definition) in properties {
                out.insert(
                    (owner.to_string(), property.to_string()),
                    definition.clone(),
                );
            }
        }
        out
    }
    let (was, is) = (ids(parent), ids(version));
    let added: Vec<&String> = is
        .keys()
        .filter(|k| !was.contains_key(*k))
        .map(|(id, _)| id)
        .collect();
    let removed: Vec<&String> = was
        .keys()
        .filter(|k| !is.contains_key(*k))
        .map(|(id, _)| id)
        .collect();

    let mut widened = Vec::new();
    let mut edges: Vec<&EdgeType> = version.edge_types.iter().collect();
    edges.sort_by_key(|t| t.id.to_string());
    for now in edges {
        let Some(had) = parent.edge_types.iter().find(|t| t.id == now.id) else {
            continue;
        };
        for (side, a, b) in [
            ("Source", &now.source_types, &had.source_types),
            ("Target", &now.target_types, &had.target_types),
        ] {
            let mut gained: Vec<String> = a.difference(b).map(ToString::to_string).collect();
            gained.sort();
            if !gained.is_empty() {
                widened.push(
                    json!({"edge_type": now.id.to_string(), "side": side, "node_types": gained}),
                );
            }
        }
    }

    let property_ids = |d: &OntologyDocument| -> BTreeSet<String> {
        declarations(d).keys().map(|(_, p)| p.clone()).collect()
    };
    let in_both: BTreeSet<String> = property_ids(parent)
        .intersection(&property_ids(version))
        .cloned()
        .collect();
    let (before, after) = (declarations(parent), declarations(version));
    let keys: BTreeSet<&(String, String)> = before.keys().chain(after.keys()).collect();
    let mut modified = Vec::new();
    for key in keys {
        if !in_both.contains(&key.1) {
            continue;
        }
        let (changed, name): (Vec<&str>, &str) = match (before.get(key), after.get(key)) {
            (None, Some(new)) => (vec!["Declared"], &new.name),
            (Some(old), None) => (vec!["Undeclared"], &old.name),
            (Some(old), Some(new)) => (
                [
                    ("Name", old.name != new.name),
                    ("ValueType", old.value_type != new.value_type),
                    ("Cardinality", old.cardinality != new.cardinality),
                    ("Required", old.required != new.required),
                    ("Constraints", old.constraints != new.constraints),
                ]
                .into_iter()
                .filter(|(_, differs)| *differs)
                .map(|(aspect, _)| aspect)
                .collect(),
                &new.name,
            ),
            (None, None) => unreachable!(),
        };
        if !changed.is_empty() {
            modified
                .push(json!({"owner": key.0, "property": key.1, "name": name, "changed": changed}));
        }
    }
    json!({"added": added, "removed": removed, "widened": widened, "modified": modified})
}

/// The lineage a version lists, in the shape [`expected`] builds: ids only for `added` and
/// `removed`, and an absent `widened` or `modified` as empty.
fn listed(version: &Value) -> Value {
    let ids = |key: &str| -> Vec<Value> {
        version[key]
            .as_array()
            .expect("always written")
            .iter()
            .map(|member| member["id"].clone())
            .collect()
    };
    let or_empty = |key: &str| version.get(key).cloned().unwrap_or_else(|| json!([]));
    json!({"added": ids("added"), "removed": ids("removed"),
        "widened": or_empty("widened"), "modified": or_empty("modified")})
}

/// Each version's four lists against a diff of `ekr_views::load`'s ontologies, independently
/// computed; and a version after the seed lists at least one entry.
fn compare_every_version(runtime: &Runtime, bytes: &[u8]) -> Vec<String> {
    let loaded = ekr_views::load(runtime, None).expect("the store loads");
    let by_id: BTreeMap<String, OntologyDocument> = loaded
        .schemas
        .values()
        .map(|(_, ontology)| (ontology.version().id.to_string(), ontology.to_document()))
        .collect();
    let document: Value = serde_json::from_slice(bytes).expect("JSON");
    let mut wrong = Vec::new();
    for version in document["schema"]["versions"].as_array().expect("versions") {
        let Some(parent) = version["parent"].as_str() else {
            continue;
        };
        let number = &version["number"];
        let want = expected(&by_id[parent], &by_id[version["id"].as_str().expect("id")]);
        let got = listed(version);
        if want != got {
            wrong.push(format!("v{number}: expected {want}, listed {got}"));
        }
        let entries: usize = ["added", "removed", "widened", "modified"]
            .iter()
            .map(|key| got[key].as_array().map_or(0, Vec::len))
            .sum();
        if entries == 0 {
            wrong.push(format!(
                "v{number} changed its ontology (evolve refuses a change without effect) and \
                 lists nothing: {version}"
            ));
        }
    }
    wrong
}

/// Every operation in one version, on both providers: the literal lists `views.yaml` specifies,
/// the independent diff, the event's counts, and one set of bytes for both providers.
#[test]
fn one_version_with_every_schema_operation_lists_each_change_once_on_both_providers() {
    let mut bytes_per_provider = Vec::new();
    for provider in [Provider::File, Provider::Sqlite] {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        build_every_operation(&runtime);
        let (bytes, summary) = overview(&runtime);
        let document: Value = serde_json::from_slice(&bytes).expect("JSON");
        let version = &document["schema"]["versions"][1];
        let (a, b, c) = (uuid(ALPHA), uuid(BETA), uuid(GAMMA));
        assert_eq!(
            version["added"],
            json!([
                {"id": uuid(DELTA), "kind": "NodeType", "name": "Delta"},
                {"id": uuid(H), "kind": "EdgeType", "name": "h"},
                {"id": uuid(P6), "kind": "Property", "name": "six"},
            ]),
            "{}",
            provider.name()
        );
        assert_eq!(version["removed"], json!([]));
        assert_eq!(
            version["widened"],
            json!([
                {"edge_type": uuid(E), "side": "Source", "node_types": [b]},
                {"edge_type": uuid(F), "side": "Target", "node_types": [c]},
                {"edge_type": uuid(G), "side": "Source", "node_types": [b, c]},
                {"edge_type": uuid(G), "side": "Target", "node_types": [b, c]},
            ]),
            "{}",
            provider.name()
        );
        let m = |owner: &str, p: u64, name: &str, changed: &[&str]| json!({"owner": owner, "property": uuid(p), "name": name, "changed": changed});
        assert_eq!(
            version["modified"],
            json!([
                m(&a, P1, "one-renamed", &["Name"]),
                m(&a, P2, "two", &["ValueType"]),
                m(&a, P3, "three", &["Cardinality"]),
                m(&a, P4, "four", &["Required"]),
                m(&a, P5, "five", &["Constraints"]),
                m(
                    &a,
                    P7,
                    "seven-b",
                    &[
                        "Name",
                        "ValueType",
                        "Cardinality",
                        "Required",
                        "Constraints"
                    ]
                ),
                m(&b, P1, "one-renamed", &["Declared"]),
                m(&uuid(E), Q, "q", &["ValueType"]),
            ]),
            "{}",
            provider.name()
        );
        assert_eq!(
            (
                summary.added,
                summary.removed,
                summary.widened,
                summary.modified
            ),
            // the seed adds Alpha, Beta, Gamma, e, f, g, six properties and q: 13
            (13 + 3, 0, 4, 8),
            "{}",
            provider.name()
        );
        let wrong = compare_every_version(&runtime, &bytes);
        assert!(wrong.is_empty(), "{}: {wrong:#?}", provider.name());
        bytes_per_provider.push(bytes);
    }
    assert_eq!(bytes_per_provider[0], bytes_per_provider[1]);
}

/// A property id that is also a type id's UUID. `ekr-ontology` admits it (property and type ids
/// are separate kinds; `Ontology::load` checks neither against the other), and `ekr-views`'
/// shared-id fixtures show one UUID across two id kinds is a state the views are held to.
///
/// Version 1 declares, on `Alpha`, a new property whose id is `Beta`'s UUID; version 2 defines a
/// node type whose id is the property `one`'s UUID. Each changes the ontology. Keyed by the id's
/// text alone, `added`/`removed` found the id already in the parent's map and listed nothing, and
/// `modified` compares property ids only, so both versions listed nothing in all four — which
/// `views.yaml` says happens exactly when a version's ontology is its parent's. A member is its
/// kind and id together (`ekr.views.SchemaMember`), so each version adds one.
#[test]
fn a_version_adding_an_id_another_kind_already_uses_is_not_listed_as_empty() {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    let mut document = empty_seed();
    let mut alpha = NodeType::new(id(ALPHA), "Alpha");
    alpha.properties.insert(id(P1), string(P1, "one"));
    document.ontology.node_types.push(alpha);
    document
        .ontology
        .node_types
        .push(NodeType::new(id(BETA), "Beta"));
    let mut writer = Writer {
        runtime: &runtime,
        clock: fixtures::CLOCK_START_MS,
        transactions: 0,
    };
    writer.seed(document);
    let shared: PropertyId = id(BETA);
    writer.commit(
        vec![modify(
            ALPHA,
            PropertyDefinition::new(shared, "shares-beta", ValueType::String),
        )],
        V1,
    );
    writer.commit(
        vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            id(P1),
            "SharesOne",
        )))],
        V2,
    );
    let (bytes, _) = overview(&runtime);
    let wrong = compare_every_version(&runtime, &bytes);
    assert!(wrong.is_empty(), "{wrong:#?}");
    let document: Value = serde_json::from_slice(&bytes).expect("JSON");
    let versions = document["schema"]["versions"].as_array().expect("versions");
    assert_eq!(
        versions[1]["added"],
        json!([{"id": uuid(BETA), "kind": "Property", "name": "shares-beta"}])
    );
    assert_eq!(
        versions[2]["added"],
        json!([{"id": uuid(P1), "kind": "NodeType", "name": "SharesOne"}])
    );
    assert_eq!(versions[1]["removed"], json!([]));
    assert_eq!(versions[2]["removed"], json!([]));
}
