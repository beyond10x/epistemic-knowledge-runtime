//! Adversary, story:graph-projection-renderer, pass 2.
//!
//! Attacks the pass-1 correction (the render refuses a property id two types declare with
//! different definitions) and the lifecycle path pass 1 did not reach.
//!
//! * The refusal compares whole `PropertyDefinition`s, but `ekr.graph-projection/1` carries only
//!   `id`, `name` and `value_kind` of one (views.yaml `ekr.views.ProjectedProperty`). Two
//!   declarations that agree on all three project exactly, whatever their `required`,
//!   `cardinality` or `constraints`, so refusing them makes a store unprojectable that the format
//!   represents without loss.
//! * A name-only difference *is* lost, and must stay refused: this guards the mutant that would
//!   narrow the comparison to the value kind.
//! * A superseded assertion projects its lifecycle and its closed valid time, and the revision
//!   before the supersession still projects it active and open.

mod support;

use std::path::Path;

use ekr_core::{
    AssertionId, ContentHash, EdgeId, EvidenceId, NodeId, PropertyId, Timestamp, TransactionId,
    TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Node, Object,
    Predicate, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    CommitCommandResult, GraphOperation, GraphTransaction, Runtime, SeedDocument, Supersession,
    ValidationCommandResult,
};
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use ekr_views::ProjectError;
use serde::Serialize;
use serde_json::Value as Json;

use support::fixtures::{self, context, id, Provider};

const BASE: u64 = 0x400;
const REFINED: u64 = 0x401;
const MEASURE: u64 = 0x402;
const ALPHA: u64 = 0x410;
const FIRST: u64 = 0x420;
const SECOND: u64 = 0x421;
const EVIDENCE: u64 = 0x430;

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

/// `Base` declares `measure` as `base`; `Refined`, a child of `Base`, redeclares it as `refined`.
fn hierarchy(base: PropertyDefinition, refined: PropertyDefinition) -> SeedDocument {
    let mut document = empty_seed();
    let mut parent = NodeType::new(id::<TypeId>(BASE), "Base");
    parent.properties.insert(base.id, base);
    let mut child = NodeType::new(id::<TypeId>(REFINED), "Refined");
    child.parents.insert(id(BASE));
    child.properties.insert(refined.id, refined);
    document.ontology.node_types.push(parent);
    document.ontology.node_types.push(child);
    document
}

fn seeded(document: SeedDocument) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    runtime
        .seed(document, || Timestamp::from_millis(1_800_000_000_001))
        .expect("the ontology admits a child type redeclaring its parent's property");
    (work, runtime)
}

/// A child that only tightens `required` agrees with its parent on every field the format
/// carries — id, name, value kind — so the projection loses nothing by listing the definition
/// once, and the render must not refuse.
#[test]
fn a_redeclaration_that_differs_only_in_what_the_format_does_not_carry_is_projected() {
    let measure: PropertyId = id(MEASURE);
    let base = PropertyDefinition::new(measure, "measure", ValueType::String);
    let mut refined = base.clone();
    refined.required = true;
    let (_work, runtime) = seeded(hierarchy(base, refined));

    let rendered = match ekr_views::project(&runtime, None) {
        Ok(rendered) => rendered,
        Err(error) => panic!(
            "Base and Refined both declare `measure` with name \"measure\" and value kind \
             String, which is all ekr.views.ProjectedProperty carries; the format represents \
             this revision exactly, yet the render refused: {error}"
        ),
    };
    let document: Json = serde_json::from_slice(&rendered.bytes).expect("JSON");
    let entries: Vec<&Json> = document["ontology"]["properties"]
        .as_array()
        .expect("ontology.properties")
        .iter()
        .filter(|property| property["id"] == Json::String(uuid(MEASURE)))
        .collect();
    assert_eq!(entries.len(), 1, "{entries:?}");
    assert_eq!(entries[0]["name"], "measure");
    assert_eq!(entries[0]["value_kind"], "String");
}

/// A redeclaration that renames the property keeps the value kind and changes a field the
/// format does carry: listing either name misleads a reader of the other type, so it is refused.
#[test]
fn a_redeclaration_that_differs_only_in_name_is_refused() {
    let measure: PropertyId = id(MEASURE);
    let base = PropertyDefinition::new(measure, "measure", ValueType::String);
    let refined = PropertyDefinition::new(measure, "refined measure", ValueType::String);
    let (_work, runtime) = seeded(hierarchy(base, refined));
    match ekr_views::project(&runtime, None) {
        Err(ProjectError::Inconsistent(message)) => {
            assert!(message.contains(&uuid(MEASURE)), "{message}");
        }
        other => panic!("two names for one property id cannot both be projected: {other:?}"),
    }
}

// ---- supersession through the real kernel ----------------------------------------------------

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

    fn commit(&mut self, operations: Vec<GraphOperation>) {
        #[derive(Serialize)]
        struct Wire<'a> {
            format: &'static str,
            transaction: &'a GraphTransaction,
        }
        self.transactions += 1;
        let transaction = GraphTransaction {
            id: id::<TransactionId>(0x500 + self.transactions),
            proposer: context().operator,
            operations,
            evidence: [id::<EvidenceId>(EVIDENCE)].into_iter().collect(),
            schema_version: None,
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
            .expect("proposal retained");
        let head = self.runtime.head().expect("head").expect("seeded").revision;
        let verdict = self
            .runtime
            .validate(transaction.id, head, || validated)
            .expect("validation runs");
        assert!(
            matches!(verdict, ValidationCommandResult::Validated(_)),
            "{verdict:?}"
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

fn claim(assertion: u64, value: &str, from: i64) -> Assertion<Value> {
    Assertion {
        id: id::<AssertionId>(assertion),
        root_id: id(2),
        subject: Subject::<NodeId, EdgeId>::Node(id(ALPHA)),
        predicate: Predicate::Property(id::<PropertyId>(MEASURE)),
        object: Object::Value(Value::String(value.into())),
        evidence: [id::<EvidenceId>(EVIDENCE)].into_iter().collect(),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::from_millis(from)),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}

fn assertion_at(runtime: &Runtime, at: u64, assertion: u64) -> Json {
    let rendered = ekr_views::project(runtime, Some(ekr_core::RevisionNumber::new(at)))
        .unwrap_or_else(|error| panic!("revision {at} renders: {error}"));
    let document: Json = serde_json::from_slice(&rendered.bytes).expect("JSON");
    document["nodes"][0]["assertions"]
        .as_array()
        .expect("assertions")
        .iter()
        .find(|entry| entry["id"] == Json::String(uuid(assertion)))
        .cloned()
        .unwrap_or_else(|| panic!("revision {at} lists assertion {}", uuid(assertion)))
}

#[test]
fn a_superseded_assertion_projects_its_lifecycle_and_closed_valid_time_only_from_its_revision() {
    let mut document = empty_seed();
    let root = document.graph.root.id;
    let measure: PropertyId = id(MEASURE);
    let mut subject = NodeType::new(id::<TypeId>(BASE), "Base");
    subject.properties.insert(
        measure,
        PropertyDefinition::new(measure, "measure", ValueType::String),
    );
    document.ontology.node_types.push(subject);
    let node = Node::<Value>::new(id(ALPHA), root, id(BASE), "alpha");
    document.graph.nodes.insert(node.id, node);
    let bytes = b"synthetic supersession statement".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let record = Evidence {
        id: id(EVIDENCE),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(1_000),
        confidence: Confidence::from_basis_points(9_000).expect("basis points"),
    };
    document.graph.evidence.insert(record.id, record);
    document.evidence_payloads.insert(hash, bytes.into());
    let (_work, runtime) = seeded(document);

    let mut writer = Writer {
        runtime: &runtime,
        clock: 1_800_000_000_100,
        transactions: 0,
    };
    writer.commit(vec![GraphOperation::AddAssertion(Box::new(claim(
        FIRST, "first", 500,
    )))]);
    writer.commit(vec![
        GraphOperation::AddAssertion(Box::new(claim(SECOND, "second", 2_000))),
        GraphOperation::SupersedeAssertion(Supersession {
            assertion: id(FIRST),
            by: id(SECOND),
            effective_from: Timestamp::from_millis(2_000),
        }),
    ]);

    let before = assertion_at(&runtime, 1, FIRST);
    assert_eq!(
        serde_json::to_string(&before["lifecycle"]).expect("json"),
        "{\"kind\":\"Active\"}",
        "{before}"
    );
    assert!(before.get("valid_to").is_none(), "{before}");
    assert!(before.get("recorded_to").is_none(), "{before}");

    let after = assertion_at(&runtime, 2, FIRST);
    assert_eq!(after["lifecycle"]["kind"], "Superseded", "{after}");
    assert_eq!(after["lifecycle"]["at_revision"], 2, "{after}");
    assert_eq!(after["lifecycle"]["by"], uuid(SECOND), "{after}");
    assert_eq!(after["lifecycle"]["effective_from"], 2_000, "{after}");
    assert!(after["lifecycle"].get("reason").is_none(), "{after}");
    let raw = String::from_utf8(
        ekr_views::project(&runtime, None)
            .expect("head renders")
            .bytes,
    )
    .expect("UTF-8");
    let ordered = format!(
        "\"lifecycle\":{{\"kind\":\"Superseded\",\"at_revision\":2,\"by\":\"{}\",\
         \"effective_from\":2000}},\"valid_from\":500,\"valid_to\":2000,",
        uuid(SECOND)
    );
    assert!(raw.contains(&ordered), "{ordered} not in {raw}");
    assert_eq!(after["valid_from"], 500, "{after}");
    assert_eq!(after["valid_to"], 2_000, "{after}");
    assert!(after.get("recorded_to").is_some(), "{after}");
}
