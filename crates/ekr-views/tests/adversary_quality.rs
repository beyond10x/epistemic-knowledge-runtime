//! Adversary pass 1 on `story:store-quality-report` (`ekr.store-quality/1`): figures the unit's
//! fixtures do not reach, each held against an independent count written out here.
//!
//! * A share whose whole does not divide the count. Every share the unit's suite checks divides
//!   exactly (0, 2500, 4000, 5000, 7500, 10000), so a share rounded to nearest, or up, passes it.
//!   `views.yaml` and `docs/cli.md` say "rounded down".
//! * A whole of 0. Every store the unit's suite reads has an active assertion and a declared
//!   property, so a share emitted for a whole of 0 (as 0, or as null) passes it. `views.yaml` says
//!   "a share of a whole of 0 is omitted".
//! * A subtype of a type declaring a constrained property: counted once, at its declarer
//!   (`views.yaml`, `ekr.views.PropertyQuality`).
//! * The property figures across schema changes, read at each revision.
//! * One name two thousand nodes of one type share: one entry, its nodes by id.

mod support;

use std::collections::BTreeSet;
use std::path::Path;

use ekr_core::{
    AssertionId, ContentHash, EvidenceId, NodeId, RevisionNumber, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Evidence, EvidenceSource, Node, Object,
    Predicate, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    CommitCommandResult, EvidenceAddition, GraphOperation, GraphTransaction, Runtime, SeedDocument,
    ValidationCommandResult,
};
use ekr_ontology::{EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use serde_json::{json, Value as Json};

use support::fixtures::{self, context, id, Fixture, Provider};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

const ITEM: u64 = 0xad_0001;
const GUARDED: u64 = 0xad_0002;
const CHILD: u64 = 0xad_0003;
const RELATES: u64 = 0xad_0004;
const TITLE: u64 = 0xad_0010;
const CODE: u64 = 0xad_0011;
const WEIGHT: u64 = 0xad_0012;
const NODES: u64 = 0xad_0100;
const SEED_EVIDENCE: u64 = 0xad_0300;
const ADDED_EVIDENCE: u64 = 0xad_0310;
const ASSERTIONS: u64 = 0xad_0400;
const TRANSACTIONS: u64 = 0xad_0500;
const MANY: u64 = 2_000;

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn read(runtime: &Runtime, at: Option<u64>) -> (Vec<u8>, Json) {
    let answer = ekr_views::report_quality(runtime, at.map(RevisionNumber::new))
        .expect("the revision is reported");
    let value = serde_json::from_slice(&answer.bytes).expect("JSON");
    (answer.bytes, value)
}

fn empty_seed() -> SeedDocument {
    let path = Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("the manifest dir"))
        .join("tests/fixtures/seed-empty.yaml");
    SeedDocument::from_yaml(&std::fs::read_to_string(path).expect("the empty seed"))
        .expect("the empty seed parses")
}

fn evidence(n: u64) -> (Evidence, Vec<u8>) {
    let payload = format!("adversary statement {n:x}").into_bytes();
    (
        Evidence {
            id: id(n),
            source: EvidenceSource::HumanStatement {
                identity: Some("operator".into()),
            },
            content_hash: ContentHash::of_bytes(&payload),
            extracted_by: context().operator,
            observed_at: Timestamp::from_millis(1_000),
            confidence: Confidence::from_basis_points(9_000).expect("basis points"),
        },
        payload,
    )
}

/// A `title` claim about node `node`, citing `cited`.
fn titled(assertion: u64, node: u64, cited: &[u64]) -> Assertion<Value> {
    Assertion {
        id: id::<AssertionId>(ASSERTIONS + assertion),
        root_id: id(2),
        subject: Subject::Node(id(NODES + node)),
        predicate: Predicate::Property(id(TITLE)),
        object: Object::Value(Value::String(format!("title {assertion}"))),
        evidence: cited.iter().map(|n| id::<EvidenceId>(*n)).collect(),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::from_millis(500)),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}

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
            .expect("the adversary seed is admitted");
    }

    fn commit(&mut self, operations: Vec<GraphOperation>) {
        #[derive(Serialize)]
        struct Wire<'a> {
            format: &'static str,
            transaction: &'a GraphTransaction,
        }
        let evidence = operations
            .iter()
            .filter_map(|operation| match operation {
                GraphOperation::AddAssertion(assertion) => Some(assertion.evidence.clone()),
                _ => None,
            })
            .flatten()
            .collect();
        self.transactions += 1;
        let transaction = GraphTransaction {
            id: id::<TransactionId>(self.transactions),
            proposer: context().operator,
            operations,
            evidence,
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
            .expect("retained");
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

fn writer(runtime: &Runtime) -> Writer<'_> {
    Writer {
        runtime,
        clock: fixtures::CLOCK_START_MS,
        transactions: TRANSACTIONS,
    }
}

/// Seed: `item` declares `title`; `guarded` declares `code` under a constraint; `child`
/// specialises `guarded` and declares nothing itself; the edge type `relates` declares `weight`
/// under a constraint. Three declarations, two constrained: 20000 / 3 = 6666.67, so the share is
/// 6666 rounded down and 6667 rounded to nearest. Nodes n1 and n2 of `item`, one seed evidence
/// record E and one assertion s1 citing it.
///
/// Revision 1: `AddEvidence` X, a2 citing X and E, a3 citing X. Three active assertions, two
/// citing evidence added after the seed: 6666 again.
fn build_thirds(runtime: &Runtime) {
    let mut document = empty_seed();
    let root = document.graph.root.id;
    let constrained = |property: u64, name: &str| {
        let mut definition = PropertyDefinition::new(id(property), name, ValueType::String);
        definition.constraints = vec!["matches [A-Z]+".into()];
        definition
    };
    let mut item = NodeType::new(id(ITEM), "item");
    item.properties.insert(
        id(TITLE),
        PropertyDefinition::new(id(TITLE), "title", ValueType::String),
    );
    let mut guarded = NodeType::new(id(GUARDED), "guarded");
    guarded
        .properties
        .insert(id(CODE), constrained(CODE, "code"));
    let mut child = NodeType::new(id(CHILD), "child");
    child.parents = [id::<TypeId>(GUARDED)].into_iter().collect();
    let mut relates = EdgeType::new(id(RELATES), "relates");
    relates.source_types = [id(ITEM)].into_iter().collect();
    relates.target_types = [id(ITEM)].into_iter().collect();
    relates
        .properties
        .insert(id(WEIGHT), constrained(WEIGHT, "weight"));
    document.ontology.node_types.extend([item, guarded, child]);
    document.ontology.edge_types.push(relates);
    for (n, name) in [(1, "first"), (2, "second")] {
        let node = Node::<Value>::new(id::<NodeId>(NODES + n), root, id(ITEM), name);
        document.graph.nodes.insert(node.id, node);
    }
    let (record, payload) = evidence(SEED_EVIDENCE);
    document
        .evidence_payloads
        .insert(record.content_hash, payload.into());
    document.graph.evidence.insert(record.id, record);
    let s1 = titled(1, 1, &[SEED_EVIDENCE]);
    document.graph.assertions.insert(s1.id, s1);

    let mut writer = writer(runtime);
    writer.seed(document);
    let (added, payload) = evidence(ADDED_EVIDENCE);
    writer.commit(vec![
        GraphOperation::AddEvidence(Box::new(EvidenceAddition {
            evidence: added,
            payload,
        })),
        GraphOperation::AddAssertion(Box::new(titled(2, 1, &[ADDED_EVIDENCE, SEED_EVIDENCE]))),
        GraphOperation::AddAssertion(Box::new(titled(3, 2, &[ADDED_EVIDENCE]))),
    ]);
}

/// A share is 10000 times the count divided by its whole, rounded down (`views.yaml`,
/// `ekr.views.BasisPoints`; `docs/cli.md`, `ekr quality`): two of three is 6666, not 6667. The
/// subtype `child` inherits `code` and declares nothing, so it adds no declaration.
#[test]
fn a_share_that_does_not_divide_is_rounded_down_and_an_inherited_property_counts_at_its_declarer() {
    for provider in PROVIDERS {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        build_thirds(&runtime);
        let properties = json!({"declared": 3, "constrained": 2, "constrained_share": 6666});

        let (_, seeded) = read(&runtime, Some(0));
        assert_eq!(
            seeded["assertions"],
            json!({
                "active": 1,
                "with_evidence": 1,
                "with_item_evidence": 0,
                "with_evidence_share": 10000,
                "with_item_evidence_share": 0,
            }),
            "{provider:?} revision 0"
        );
        assert_eq!(seeded["properties"], properties, "{provider:?} revision 0");

        let (bytes, head) = read(&runtime, None);
        assert_eq!(head["meta"]["revision"], 1, "{provider:?}");
        assert_eq!(
            head["assertions"],
            json!({
                "active": 3,
                "with_evidence": 3,
                "with_item_evidence": 2,
                "with_evidence_share": 10000,
                "with_item_evidence_share": 6666,
            }),
            "{provider:?} revision 1"
        );
        assert_eq!(head["properties"], properties, "{provider:?} revision 1");
        assert_eq!(head["shared_names"], json!([]), "{provider:?}");
        assert_eq!(head["sharing_nodes"], 0, "{provider:?}");
        let text = String::from_utf8(bytes).expect("UTF-8");
        assert!(
            text.contains(r#""with_item_evidence_share":6666}"#)
                && text.contains(r#""constrained_share":6666}"#),
            "{text}"
        );
    }
}

/// A share of a whole of 0 is omitted, never emitted as 0 or null (`views.yaml`, rule (4) and
/// `ekr.views.AssertionQuality` / `PropertyQuality`). The `hub` fixture declares no property and
/// holds no assertion.
#[test]
fn a_whole_of_zero_omits_its_share() {
    for provider in PROVIDERS {
        let work = tempfile::tempdir().expect("work directory");
        let runtime = fixtures::open(work.path(), provider);
        Fixture::Hub.build(&runtime);
        let (bytes, value) = read(&runtime, None);
        assert_eq!(
            value["assertions"],
            json!({"active": 0, "with_evidence": 0, "with_item_evidence": 0}),
            "{provider:?}"
        );
        assert_eq!(
            value["properties"],
            json!({"declared": 0, "constrained": 0}),
            "{provider:?}"
        );
        let text = String::from_utf8(bytes).expect("UTF-8");
        assert!(
            text.starts_with(concat!(
                r#"{"meta":{"format":"ekr.store-quality/1","revision":0},"#,
                r#""assertions":{"active":0,"with_evidence":0,"with_item_evidence":0},"#,
                r#""properties":{"declared":0,"constrained":0},"#,
                r#""shared_names":[],"sharing_nodes":0}"#
            )),
            "{text}"
        );
    }
}

/// The property figures are the revision's own schema: the `store` fixture declares `label` at
/// the seed; version 1 at revision 1 adds `summary`, `strength` and `note`; version 2 at revision
/// 4 adds `source`. Its one seeded claim cites seed evidence, an edge claim citing seed evidence
/// arrives at 2, and the seeded claim is retracted at 3.
#[test]
fn the_property_and_assertion_figures_follow_each_revisions_schema_and_lifecycle() {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::Sqlite);
    Fixture::Evolved.build(&runtime);
    let declared = [1, 4, 4, 4, 5, 5];
    let active = [1, 1, 2, 1, 1, 1];
    for at in 0..6_usize {
        let (_, value) = read(&runtime, Some(at as u64));
        assert_eq!(
            value["properties"],
            json!({"declared": declared[at], "constrained": 0, "constrained_share": 0}),
            "revision {at}"
        );
        assert_eq!(
            value["assertions"],
            json!({
                "active": active[at],
                "with_evidence": active[at],
                "with_item_evidence": 0,
                "with_evidence_share": 10000,
                "with_item_evidence_share": 0,
            }),
            "revision {at}"
        );
        assert_eq!(value["shared_names"], json!([]), "revision {at}");
    }
}

/// Two thousand nodes of one type named `Same`, inserted in descending id order, and one named
/// `Other`: one entry, every node by id ascending, and `sharing_nodes` 2000.
#[test]
fn one_name_two_thousand_nodes_share_is_one_entry_listing_every_node_by_id() {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), Provider::File);
    let mut document = empty_seed();
    let root = document.graph.root.id;
    document
        .ontology
        .node_types
        .push(NodeType::new(id(ITEM), "item"));
    for n in (1..=MANY).rev() {
        let node = Node::<Value>::new(id::<NodeId>(NODES + n), root, id(ITEM), "Same");
        document.graph.nodes.insert(node.id, node);
    }
    let other = Node::<Value>::new(id::<NodeId>(NODES + MANY + 1), root, id(ITEM), "Other");
    document.graph.nodes.insert(other.id, other);
    writer(&runtime).seed(document);

    let (bytes, value) = read(&runtime, None);
    let nodes: Vec<String> = (1..=MANY).map(|n| uuid(NODES + n)).collect();
    assert_eq!(
        value["shared_names"],
        json!([{"type": uuid(ITEM), "name": "Same", "nodes": nodes}])
    );
    assert_eq!(value["sharing_nodes"], MANY);
    let ids: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
    assert_eq!(ids.len() as u64, MANY);
    // The document grows with the nodes it lists: 39 bytes a node id and its separator.
    assert!(bytes.len() > 39 * MANY as usize, "{} bytes", bytes.len());
}
