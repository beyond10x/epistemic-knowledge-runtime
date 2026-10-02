//! Adversary pass 1 on `story:add-evidence-operation` (wave ingest-02, unit A), from the views'
//! side: evidence an `AddEvidence` commit brings is in the projection as retained, in the node
//! detail of the node an assertion citing it is about, and in the ChangesSince row of that
//! assertion — on both providers.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use ekr_views::{ChangesRequest, Index, SinceKind};
use serde::Serialize;
use serde_json::Value as Json;
use std::collections::BTreeSet;

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}

fn anchor() -> AuthorityStateV1 {
    let c = context();
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [(c.operator, "operator"), (c.validator, "validator")]
            .into_iter()
            .map(|(id, name)| {
                (
                    id,
                    Agent {
                        id,
                        name: name.into(),
                        capabilities: BTreeSet::new(),
                    },
                )
            })
            .collect(),
        validation_profile: ValidationProfileV1::deterministic(c.validator),
    }
}

fn at(millis: i64) -> impl FnOnce() -> Timestamp {
    move || Timestamp::from_millis(millis)
}

fn human_evidence(id: EvidenceId, payload: &[u8]) -> Evidence {
    Evidence {
        id,
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: ContentHash::of_bytes(payload),
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(5),
        confidence: Confidence::CERTAIN,
    }
}

const SEED: &str = "format: ekr-seed/2
ontology:
  version:
    id: 00000000-0000-4000-8000-000000000001
    number: 0
    parent: null
    created_at: 0
  node_types: []
  edge_types: []
graph:
  format: ekr.graph-document/2
  graph:
    root:
      id: 00000000-0000-4000-8000-000000000002
      space: Canonical
      schema_version_id: 00000000-0000-4000-8000-000000000001
      parent: null
      created_at: 0
    revision: 0
    nodes: {}
    edges: {}
    assertions: {}
    evidence: {}
evidence_payloads: {}
";

/// A store holding `n` evidence entries that one commit added, reopened.
fn store_with_added_evidence(directory: &std::path::Path, file: bool, n: usize) -> Runtime {
    let open = || {
        if file {
            Runtime::file(directory, "test", context(), anchor())
        } else {
            Runtime::sqlite(&directory.join("state.db"), "test", context(), anchor())
        }
        .unwrap()
    };
    let runtime = open();
    runtime
        .seed(SeedDocument::from_yaml(SEED).unwrap(), at(10))
        .unwrap();
    let operations = (0..n)
        .map(|i| {
            let payload = format!("statement {i}").into_bytes();
            GraphOperation::AddEvidence(Box::new(EvidenceAddition {
                evidence: human_evidence(EvidenceId::mint(), &payload),
                payload,
            }))
        })
        .collect();
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: BTreeSet::new(),
        schema_version: None,
    };
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: &tx,
    })
    .unwrap();
    runtime
        .propose(bytes.as_bytes(), context().operator, at(20))
        .unwrap();
    assert!(matches!(
        runtime
            .validate(tx.id, RevisionNumber::SEED, at(21))
            .unwrap(),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        runtime.commit(tx.id, context().operator, at(22)).unwrap(),
        CommitCommandResult::Committed(_)
    ));
    drop(runtime);
    open()
}

/// The provider reads one render of the head of `runtime`'s store makes on this thread, by kind,
/// counted by the store (`ekr_kernel::read_work` and `ekr_kernel::stream_reads`). The runtime
/// drives its store on the calling thread (a current-thread Tokio runtime), so the count is the
/// render's, and the other cases this binary runs at the same time, each on its own thread, do not
/// reach it. The store's blobs-hashed and bytes-compared counts are left out: which of the two a
/// blob costs depends on the verified copies other live handles in the process hold.
fn render_reads(runtime: &Runtime, evidence: u64) -> [(&'static str, u64); 6] {
    let _ = (ekr_kernel::read_work(), ekr_kernel::stream_reads());
    let rendered = ekr_views::project(runtime, None).unwrap();
    let (work, streams) = (ekr_kernel::read_work(), ekr_kernel::stream_reads());
    assert_eq!(rendered.summary.retained_evidence, evidence);
    [
        ("blobs read", work.blobs_read),
        ("revision occurrences read", work.occurrences_read),
        ("revision-stream reads", streams.revision),
        ("checkpoint-stream reads", streams.checkpoint),
        ("object-stream reads", streams.object),
        ("tenant-log reads", streams.feed),
    ]
}

/// The reads that find the head: a render that reads it once makes the same number of these
/// whatever the number of entries it projects.
const HEAD_READS: [&str; 3] = [
    "revision-stream reads",
    "checkpoint-stream reads",
    "tenant-log reads",
];

/// `AddEvidence` makes the number of evidence entries grow with every ingest, so what a view
/// costs per entry is what it costs per commit. The projection reads the head once
/// ([`Runtime::schema_history`]) and must not cost more than linear work in the entries it
/// projects. The work is the provider reads the store counts, not a clock, so the machine's load
/// cannot move it (`AGENTS.md`):
///
/// - the first render of a reopened store verifies its history, and eight times the entries may
///   cost at most sixteen times the reads of every kind (linear is eight, quadratic sixty-four) —
///   twice linear, the slack `command_bench` gives kernel commands;
/// - every later render finds the head with the same number of reads at 400 entries as at 50.
///   Asking the runtime once per entry, which made a render quadratic before wave ingest-02 (400
///   entries 13.66 s), adds reads of the head per entry.
#[test]
fn rendering_costs_linear_time_in_the_evidence_commits_added() {
    for file in [false, true] {
        let small = tempfile::tempdir().unwrap();
        let large = tempfile::tempdir().unwrap();
        let small_runtime = store_with_added_evidence(small.path(), file, 50);
        let large_runtime = store_with_added_evidence(large.path(), file, 400);
        let [small_reads, large_reads] = [(&small_runtime, 50), (&large_runtime, 400)]
            .map(|(runtime, evidence)| [(); 3].map(|()| render_reads(runtime, evidence)));
        println!("file={file}: 50 entries {small_reads:?}\n  400 entries {large_reads:?}");
        for (entries, reads) in [(50, &small_reads[0]), (400, &large_reads[0])] {
            assert!(
                reads[0].1 >= entries,
                "file={file}: the first render of {entries} entries read {reads:?}, not every \
                 retained payload, so the count is not counting this render"
            );
        }
        for ((what, small), (_, large)) in small_reads[0].iter().zip(&large_reads[0]) {
            assert!(
                *large <= small * 16,
                "file={file}: the first render of 400 added evidence entries made {large} {what}, \
                 {:.1} times the {small} of 50",
                *large as f64 / *small as f64
            );
        }
        for render in 1..3 {
            for ((what, small), (_, large)) in small_reads[render].iter().zip(&large_reads[render])
            {
                if HEAD_READS.contains(what) {
                    assert_eq!(
                        small, large,
                        "file={file}: render {render} made {small} {what} at 50 entries and \
                         {large} at 400; a render reads the head once, not once per entry"
                    );
                }
            }
        }
    }
}

#[test]
fn added_evidence_is_projected_retained_described_and_listed_as_a_change() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let runtime = if file {
            Runtime::file(directory.path(), "test", context(), anchor())
        } else {
            Runtime::sqlite(
                &directory.path().join("state.db"),
                "test",
                context(),
                anchor(),
            )
        }
        .unwrap();
        let mut document = SeedDocument::from_yaml(SEED).unwrap();
        let type_id: TypeId = "00000000-0000-4000-8000-000000000005".parse().unwrap();
        let property: PropertyId = "00000000-0000-4000-8000-000000000006".parse().unwrap();
        let mut declared = NodeType::new(type_id, "Subject");
        declared.properties.insert(
            property,
            PropertyDefinition::new(property, "label", ValueType::String),
        );
        document.ontology.node_types.push(declared);
        let node = Node::<Value>::new(NodeId::mint(), document.graph.root.id, type_id, "subject");
        document.graph.nodes.insert(node.id, node.clone());
        runtime.seed(document.clone(), at(10)).unwrap();

        let payload = b"a statement added after the seed".to_vec();
        let evidence = human_evidence(EvidenceId::mint(), &payload);
        let claim = Assertion {
            id: AssertionId::mint(),
            root_id: document.graph.root.id,
            subject: Subject::Node(node.id),
            predicate: Predicate::Property(property),
            object: Object::Value(Value::String("labelled".into())),
            evidence: BTreeSet::from([evidence.id]),
            proposed_by: context().operator,
            assessment: Assessment::Proposed,
            lifecycle: AssertionLifecycle::Active,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        };
        let tx = GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            operations: vec![
                GraphOperation::AddEvidence(Box::new(EvidenceAddition {
                    evidence: evidence.clone(),
                    payload: payload.clone(),
                })),
                GraphOperation::AddAssertion(Box::new(claim.clone())),
            ],
            evidence: BTreeSet::from([evidence.id]),
            schema_version: None,
        };
        #[derive(Serialize)]
        struct Wire<'a> {
            format: &'static str,
            transaction: &'a GraphTransaction,
        }
        let bytes = serde_yaml_ng::to_string(&Wire {
            format: "ekr.transaction-document/2",
            transaction: &tx,
        })
        .unwrap();
        runtime
            .propose(bytes.as_bytes(), context().operator, at(20))
            .unwrap();
        assert!(matches!(
            runtime
                .validate(tx.id, RevisionNumber::SEED, at(21))
                .unwrap(),
            ValidationCommandResult::Validated(_)
        ));
        assert!(matches!(
            runtime.commit(tx.id, context().operator, at(22)).unwrap(),
            CommitCommandResult::Committed(_)
        ));

        // The projection: the added entry, retained.
        let rendered = ekr_views::project(&runtime, None).unwrap();
        assert_eq!(rendered.summary.evidence, 1, "file={file}");
        assert_eq!(rendered.summary.retained_evidence, 1, "file={file}");
        let projected: Json = serde_json::from_slice(&rendered.bytes).unwrap();
        let entry = projected["evidence"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["id"] == evidence.id.to_string())
            .unwrap_or_else(|| panic!("no entry for the added evidence: {projected}"));
        assert_eq!(entry["retained"], true);
        assert_eq!(entry["content_hash"], evidence.content_hash.to_string());

        // The node detail: the assertion about the node cites it.
        let index = Index::load(&runtime, None).unwrap();
        let described = index.describe(node.id).unwrap();
        assert_eq!(described.summary.assertions, 1);
        let detail = String::from_utf8(described.bytes).unwrap();
        assert!(detail.contains(&evidence.id.to_string()), "{detail}");

        // ChangesSince revision 0: the assertion added, citing it.
        let changes = index
            .changes(
                &runtime,
                &ChangesRequest::new(SinceKind::Revision, 0, None, None).unwrap(),
            )
            .unwrap();
        let listed: Json = serde_json::from_slice(&changes.bytes).unwrap();
        let row = listed["changes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["id"] == claim.id.to_string())
            .unwrap_or_else(|| panic!("no change for the assertion: {listed}"));
        assert_eq!(row["change"], "AssertionAdded");
        assert_eq!(
            row["evidence"],
            serde_json::json!([evidence.id.to_string()])
        );
    }
}
