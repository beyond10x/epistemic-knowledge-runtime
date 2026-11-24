//! A named event time (`--event-time Type.property`) read from the revision's active assertions,
//! not only from the node's record (review of 0.0.42, finding 17).
//!
//! An extraction records a fact about a node as an assertion: `Project.started_at` lands as an
//! active, evidenced `Timestamp` assertion about the project, and the node's record holds no value.
//! The log dates such a node by it, as `ekr.node-detail/2` lists the record's values and the active
//! value assertions together; a retracted or superseded assertion dates nothing at a revision that
//! holds it withdrawn, while a read of an earlier revision still sees it.
//!
//! A store of its own, on both providers. The seed: node types `Project` (declaring `started_at`,
//! a `Timestamp`) and `Team`, an edge type `runs` from `Team` to `Project`, one team running three
//! projects, `alpha`, `beta` and `gamma`, none with a recorded `started_at`, and one statement.
//!
//! * revision 1: `alpha` started at `ALPHA_MS`, `beta` at `BETA_MS`, `gamma` at `GAMMA_MS`, each an
//!   assertion citing the statement, as an extraction commits it;
//! * revision 2: `beta`'s assertion retracted; `gamma`'s superseded by one starting at
//!   `GAMMA_LATER_MS`.

mod support;

use std::collections::BTreeSet;
use std::path::Path;

use ekr_core::{
    AssertionId, ContentHash, EdgeId, EvidenceId, PropertyId, RevisionNumber, Timestamp,
    TransactionId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, Confidence, Edge, Evidence, EvidenceSource, Node,
    Object, Predicate, RetractionReason, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    CommitCommandResult, GraphOperation, GraphTransaction, Retraction, Runtime, SeedDocument,
    Supersession, ValidationCommandResult,
};
use ekr_ontology::{Cardinality, EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use serde_json::Value as Json;

use support::fixtures::{self, context, id, Provider};

const PROVIDERS: [Provider; 2] = [Provider::File, Provider::Sqlite];

const PROJECT: u64 = 0xe7_0001;
const TEAM: u64 = 0xe7_0002;
const RUNS: u64 = 0xe7_0003;
const STARTED_AT: u64 = 0xe7_0010;
const CREW: u64 = 0xe7_0100;
const ALPHA: u64 = 0xe7_0101;
const BETA: u64 = 0xe7_0102;
const GAMMA: u64 = 0xe7_0103;
const EDGES: u64 = 0xe7_0200;
const STATEMENT: u64 = 0xe7_0300;
const CLAIMS: u64 = 0xe7_0400;
const TRANSACTIONS: u64 = 0xe7_0500;

/// 2026-03-12T00:01:00Z, the finding's time.
const ALPHA_MS: i64 = 1_773_273_660_000;
const BETA_MS: i64 = ALPHA_MS + 3_600_000;
const GAMMA_MS: i64 = ALPHA_MS + 7_200_000;
const GAMMA_LATER_MS: i64 = ALPHA_MS + 86_400_000;

const SELECTOR: &str = "Project.started_at";

fn uuid(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

fn statement() -> (Evidence, Vec<u8>) {
    let payload = b"the kickoff notes".to_vec();
    (
        Evidence {
            id: id(STATEMENT),
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

/// Claim `n`: project `node` started at `at`, citing the statement.
fn started(n: u64, node: u64, at: i64) -> Assertion<Value> {
    Assertion {
        id: id::<AssertionId>(CLAIMS + n),
        root_id: id(2),
        subject: Subject::Node(id(node)),
        predicate: Predicate::Property(id::<PropertyId>(STARTED_AT)),
        object: Object::Value(Value::Timestamp(Timestamp::from_millis(at))),
        evidence: [id::<EvidenceId>(STATEMENT)].into_iter().collect(),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::from_millis(at)),
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

    fn commit(&mut self, operations: Vec<GraphOperation>) {
        #[derive(Serialize)]
        struct Wire<'a> {
            format: &'static str,
            transaction: &'a GraphTransaction,
        }
        let evidence: BTreeSet<EvidenceId> = operations
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

fn build(provider: Provider) -> (tempfile::TempDir, Runtime) {
    let work = tempfile::tempdir().expect("work directory");
    let runtime = fixtures::open(work.path(), provider);
    let path = Path::new(&std::env::var("CARGO_MANIFEST_DIR").expect("the manifest dir"))
        .join("tests/fixtures/seed-empty.yaml");
    let mut document =
        SeedDocument::from_yaml(&std::fs::read_to_string(path).expect("the empty seed"))
            .expect("the empty seed parses");
    let root = document.graph.root.id;
    let mut project = NodeType::new(id(PROJECT), "Project");
    project.properties.insert(
        id(STARTED_AT),
        PropertyDefinition::new(id(STARTED_AT), "started_at", ValueType::Timestamp),
    );
    let team = NodeType::new(id(TEAM), "Team");
    let mut runs = EdgeType::new(id(RUNS), "runs");
    runs.source_types = [id(TEAM)].into_iter().collect();
    runs.target_types = [id(PROJECT)].into_iter().collect();
    runs.cardinality = Cardinality::Many;
    document.ontology.node_types.extend([project, team]);
    document.ontology.edge_types.push(runs);
    let crew = Node::<Value>::new(id(CREW), root, id(TEAM), "crew");
    document.graph.nodes.insert(crew.id, crew);
    for (n, (node, name)) in [(ALPHA, "alpha"), (BETA, "beta"), (GAMMA, "gamma")]
        .into_iter()
        .enumerate()
    {
        let held = Node::<Value>::new(id(node), root, id(PROJECT), name);
        document.graph.nodes.insert(held.id, held);
        let edge = Edge::<Value>::new(
            id::<EdgeId>(EDGES + n as u64),
            root,
            id(RUNS),
            id(CREW),
            id(node),
        );
        document.graph.edges.insert(edge.id, edge);
    }
    let (record, payload) = statement();
    document
        .evidence_payloads
        .insert(record.content_hash, payload.into());
    document.graph.evidence.insert(record.id, record);
    let mut writer = Writer {
        runtime: &runtime,
        clock: fixtures::CLOCK_START_MS,
        transactions: TRANSACTIONS,
    };
    let now = writer.tick();
    runtime
        .seed(document, || now)
        .expect("the seed is admitted");

    writer.commit(vec![
        GraphOperation::AddAssertion(Box::new(started(1, ALPHA, ALPHA_MS))),
        GraphOperation::AddAssertion(Box::new(started(2, BETA, BETA_MS))),
        GraphOperation::AddAssertion(Box::new(started(3, GAMMA, GAMMA_MS))),
    ]);
    writer.commit(vec![
        GraphOperation::RetractAssertion(Retraction {
            assertion: id(CLAIMS + 2),
            reason: RetractionReason::new("beta never started"),
        }),
        GraphOperation::AddAssertion(Box::new(started(4, GAMMA, GAMMA_LATER_MS))),
        GraphOperation::SupersedeAssertion(Supersession {
            assertion: id(CLAIMS + 3),
            by: id(CLAIMS + 4),
            effective_from: Timestamp::from_millis(GAMMA_LATER_MS),
        }),
    ]);
    (work, runtime)
}

/// The log of revision `at` with the one selector: its events as (id, time), and its summary.
fn events(runtime: &Runtime, at: u64) -> (Vec<(String, String)>, ekr_views::OcelExported) {
    let answer = ekr_views::export_ocel_with_event_time(
        runtime,
        Some(RevisionNumber::new(at)),
        &[],
        &[SELECTOR.to_owned()],
    )
    .expect("the log is exported");
    let value: Json = serde_json::from_slice(&answer.bytes).expect("JSON");
    let listed = value["ocel"]["events"]
        .as_array()
        .expect("events")
        .iter()
        .map(|event| {
            (
                event["id"].as_str().expect("an id").to_owned(),
                event["time"].as_str().expect("a time").to_owned(),
            )
        })
        .collect();
    (listed, answer.summary)
}

fn cases(runtime: &Runtime, at: u64) -> ekr_views::ProcessMapped {
    ekr_views::export_process_map(
        runtime,
        Some(RevisionNumber::new(at)),
        &[],
        &[SELECTOR.to_owned()],
    )
    .expect("the map is exported")
    .summary
}

#[test]
fn an_extracted_event_time_dates_the_event_in_the_log_and_the_process_map() {
    for provider in PROVIDERS {
        let (_work, runtime) = build(provider);
        let (listed, summary) = events(&runtime, 1);
        assert_eq!(
            listed,
            vec![
                (uuid(ALPHA), "2026-03-12T00:01:00.000Z".to_owned()),
                (uuid(BETA), "2026-03-12T01:01:00.000Z".to_owned()),
                (uuid(GAMMA), "2026-03-12T02:01:00.000Z".to_owned()),
            ],
            "{provider:?}"
        );
        assert_eq!(
            (summary.events, summary.undated_events, summary.objects),
            (3, 0, 1),
            "{provider:?}"
        );
        let mapped = cases(&runtime, 1);
        assert_eq!((mapped.cases, mapped.variants), (1, 1), "{provider:?}");
    }
}

#[test]
fn a_retracted_or_superseded_event_time_dates_nothing_from_its_withdrawal_on() {
    for provider in PROVIDERS {
        let (_work, runtime) = build(provider);
        let (listed, summary) = events(&runtime, 2);
        assert_eq!(
            listed,
            vec![
                (uuid(ALPHA), "2026-03-12T00:01:00.000Z".to_owned()),
                (uuid(GAMMA), "2026-03-13T00:01:00.000Z".to_owned()),
            ],
            "{provider:?}"
        );
        assert_eq!(
            (summary.events, summary.undated_events),
            (2, 1),
            "{provider:?}"
        );
        // Revision 1 read after the withdrawal still holds the times it held.
        let (earlier, _) = events(&runtime, 1);
        assert_eq!(earlier.len(), 3, "{provider:?}");
        let mapped = cases(&runtime, 2);
        assert_eq!(mapped.cases, 1, "{provider:?}");
    }
}
