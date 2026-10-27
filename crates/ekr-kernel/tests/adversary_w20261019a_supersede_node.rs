//! Adversary pass 1, wave 2026-10-19a, `story:supersede-node`: same-transaction interactions,
//! chains, later operations and replay checkpoints, on both providers.
//!
//! The seed is `supersession-seed.yaml` (Tag, WorkItem, MergeRequest, TAGGED), with one more
//! WorkItem property, `relates`, a `NodeRef` to a MergeRequest: a reference inside a value is a
//! reference (`validate/reference.rs`, `Known::value`), so a value naming the node a supersession
//! retires names that node.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ekr_core::{
    AssertionId, ContentHash, EdgeId, EvidenceId, NodeId, RevisionNumber, Timestamp, TransactionId,
};
use ekr_graph::{AliasIndex, Confidence, Evidence, EvidenceSource, Root};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, CommitCommandResult, EdgeDraft, EntityMerge,
    GraphOperation, GraphTransaction, NodeDraft, NodeSupersession, PropertyMutation, Runtime,
    SeedDocument, ValidationCommandResult, ValidationProfileV1, ValidatorName,
};
use ekr_ontology::Value;
use ekr_store::RevisionLog;
use serde::Serialize;

const SEED: &str = include_str!("fixtures/supersession-seed.yaml");
const TENANT: &str = "test";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const TAG: &str = "00000000-0000-4000-8000-000000000201";
const WORK_ITEM: &str = "00000000-0000-4000-8000-000000000202";
const TAGGED: &str = "00000000-0000-4000-8000-000000000204";
const NOTE: &str = "00000000-0000-4000-8000-000000000212";
/// `relates`, a WorkItem property whose value is a NodeRef to a MergeRequest.
const RELATES: &str = "00000000-0000-4000-8000-000000000215";
/// WorkItem `group/app!18`.
const KEPT: &str = "00000000-0000-4000-8000-000000000301";
/// MergeRequest `group/app!18`, tagged `parser`.
const DUPLICATE: &str = "00000000-0000-4000-8000-000000000302";
/// Tag `parser`.
const PARSER: &str = "00000000-0000-4000-8000-000000000303";
/// WorkItem `tracker/18`.
const LATER: &str = "00000000-0000-4000-8000-000000000304";
/// MergeRequest `group/app!19`.
const OTHER: &str = "00000000-0000-4000-8000-000000000305";
/// TAGGED, DUPLICATE to PARSER.
const DUPLICATE_TAGGED: &str = "00000000-0000-4000-8000-000000000501";
const STATEMENT: &str = "00000000-0000-4000-8000-000000000401";
const STATEMENT_TEXT: &str = "group/app!18 is one merge request, tracked as one work item.";
const REASON: &str = "recorded twice";
const NAMED_IN_TRANSACTION: &str = "superseded-node-named-in-transaction";

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000101".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000102".parse().unwrap(),
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

fn open(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), anchor())
    }
    .unwrap()
}

fn id<T: std::str::FromStr>(text: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    text.parse().unwrap()
}

/// The fixture seed with WorkItem's `relates` property added, and the statement retained.
fn seed() -> SeedDocument {
    let anchor = "        constraints: []\n    abstract_type: false\n    lifecycle: null\n    \
                  operations: {}\n  - id: 00000000-0000-4000-8000-000000000203";
    assert_eq!(
        SEED.matches(anchor).count(),
        1,
        "the WorkItem block ends once"
    );
    let relates = format!(
        "        constraints: []
      {RELATES}:
        id: {RELATES}
        name: relates
        value_type:
          value_kind: NodeRef
          parameters:
            allowed_types:
            - 00000000-0000-4000-8000-000000000203
        cardinality: One
        required: false
        constraints: []
    abstract_type: false
    lifecycle: null
    operations: {{}}
  - id: 00000000-0000-4000-8000-000000000203"
    );
    let mut document = SeedDocument::from_yaml(&SEED.replace(anchor, &relates)).unwrap();
    let hash = ContentHash::of_bytes(STATEMENT_TEXT.as_bytes());
    let statement = Evidence {
        id: id(STATEMENT),
        source: EvidenceSource::HumanStatement {
            identity: Some("Runtime operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(1_773_273_600_000),
        confidence: Confidence::CERTAIN,
    };
    document.graph.evidence.insert(statement.id, statement);
    document.evidence_payloads.insert(
        hash,
        std::sync::Arc::new(STATEMENT_TEXT.as_bytes().to_vec()),
    );
    document
}

fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}

fn transaction(operations: Vec<GraphOperation>, evidence: &[&str]) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: evidence.iter().map(|text| id(text)).collect(),
        schema_version: None,
    }
}

fn supersede(node: &str, by: &str) -> GraphOperation {
    supersede_ids(id(node), id(by))
}

fn supersede_ids(node: NodeId, by: NodeId) -> GraphOperation {
    GraphOperation::SupersedeNode(NodeSupersession {
        node,
        by,
        reason: REASON.to_owned(),
        evidence: BTreeSet::from([id::<EvidenceId>(STATEMENT)]),
    })
}

fn node(node: NodeId, type_id: &str, name: &str, aliases: &[&str]) -> NodeDraft {
    NodeDraft {
        id: node,
        root_id: id(ROOT),
        type_id: id(type_id),
        canonical_name: name.to_owned(),
        properties: BTreeMap::new(),
        aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
    }
}

/// A `note` assertion about the edge `edge`, citing the statement.
fn about_edge(edge: &str) -> GraphOperation {
    let yaml = format!(
        "id: {}
root_id: {ROOT}
subject: !Edge {edge}
predicate: !Property {NOTE}
object: !Value
  value_kind: String
  value: imported
evidence:
- {STATEMENT}
proposed_by: {}
assessment: Proposed
lifecycle: Active
valid_time:
  from: 1577836800000
  to: null
transaction_time:
  recorded_from: 0
  recorded_to: null
",
        AssertionId::mint(),
        context().operator
    );
    GraphOperation::AddAssertion(Box::new(serde_yaml_ng::from_str(&yaml).unwrap()))
}

/// A Relation assertion `subject` TAGGED `object`, about a node and naming a node as its object.
fn relation(subject: &str, object: &str) -> GraphOperation {
    let yaml = format!(
        "id: {}
root_id: {ROOT}
subject: !Node {subject}
predicate: !Relation {TAGGED}
object: !Node {object}
evidence:
- {STATEMENT}
proposed_by: {}
assessment: Proposed
lifecycle: Active
valid_time:
  from: 1577836800000
  to: null
transaction_time:
  recorded_from: 0
  recorded_to: null
",
        AssertionId::mint(),
        context().operator
    );
    GraphOperation::AddAssertion(Box::new(serde_yaml_ng::from_str(&yaml).unwrap()))
}

fn submit(kernel: &Runtime, tx: &GraphTransaction, at: i64) -> ValidationCommandResult {
    kernel
        .propose(&encode(tx), context().operator, || {
            Timestamp::from_millis(at)
        })
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    let verdict = kernel
        .validate(tx.id, head, || Timestamp::from_millis(at + 1))
        .unwrap();
    if matches!(verdict, ValidationCommandResult::Validated(_)) {
        let committed = kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
            .unwrap();
        assert!(
            matches!(committed, CommitCommandResult::Committed(_)),
            "{committed:?}"
        );
    }
    verdict
}

fn committed(kernel: &Runtime, tx: &GraphTransaction, at: i64) {
    let verdict = submit(kernel, tx, at);
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
}

fn codes(verdict: &ValidationCommandResult) -> Vec<(ValidatorName, String)> {
    match verdict {
        ValidationCommandResult::Rejected(record) => record
            .issues
            .iter()
            .map(|issue| (issue.validator, issue.code.clone()))
            .collect(),
        ValidationCommandResult::Validated(_) => Vec::new(),
    }
}

fn roots(kernel: &Runtime) -> Vec<Root> {
    let head = kernel.head().unwrap().unwrap();
    (0..=head.revision.get())
        .map(|number| kernel.read(Some(RevisionNumber::new(number))).unwrap().root)
        .collect()
}

fn each(mut run: impl FnMut(&str, &Path, bool)) {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        run(if file { "file" } else { "sqlite" }, directory.path(), file);
    }
}

/// The seeded store.
fn seeded(path: &Path, file: bool) -> Runtime {
    let kernel = open(path, file);
    kernel.seed(seed(), || Timestamp::from_millis(10)).unwrap();
    kernel
}

/// A NodeRef value naming a node the same transaction supersedes is a reference to it, as the
/// same value in a later transaction is (`node-superseded`): it is refused as
/// `superseded-node-named-in-transaction`, whether it is a created node's value or an updated
/// one's, and the head does not move. Otherwise the kept node commits holding a reference to a
/// node no head read holds.
#[test]
fn adv_a_node_ref_value_naming_a_node_superseded_in_its_transaction_is_refused() {
    each(|at, path, file| {
        let kernel = seeded(path, file);
        let head = kernel.head().unwrap().unwrap();
        let updated = submit(
            &kernel,
            &transaction(
                vec![
                    supersede(OTHER, KEPT),
                    GraphOperation::UpdateProperty(PropertyMutation {
                        node: id(KEPT),
                        property: id(RELATES),
                        values: vec![Value::NodeRef(id(OTHER))],
                    }),
                ],
                &[STATEMENT],
            ),
            100,
        );
        // On a store of its own, so that the first case's outcome does not decide the second's.
        let second = tempfile::tempdir().unwrap();
        let other_kernel = seeded(second.path(), file);
        let mut created_node = node(NodeId::mint(), WORK_ITEM, "group/app!20", &[]);
        created_node
            .properties
            .insert(id(RELATES), vec![Value::NodeRef(id(OTHER))]);
        let created = submit(
            &other_kernel,
            &transaction(
                vec![
                    supersede(OTHER, KEPT),
                    GraphOperation::CreateNode(created_node),
                ],
                &[STATEMENT],
            ),
            200,
        );
        let refused = |codes: &[(ValidatorName, String)]| {
            codes.iter().any(|(validator, code)| {
                *validator == ValidatorName::Structural && code == NAMED_IN_TRANSACTION
            })
        };
        let after = kernel.snapshot().unwrap();
        assert!(
            refused(&codes(&updated)) && refused(&codes(&created)),
            "{at}: an UpdateProperty answered {:?} and a CreateNode {:?}; the head holds {} \
             with relates = {:?}",
            codes(&updated),
            codes(&created),
            if after.superseded.contains_key(&id(OTHER)) {
                "the supersession"
            } else {
                "no supersession"
            },
            after.nodes[&id::<NodeId>(KEPT)]
                .properties
                .get(&id(RELATES))
        );
        assert_eq!(kernel.head().unwrap().unwrap(), head, "{at}: nothing moved");
    });
}

/// The control for the case above: in a later transaction the same NodeRef value is
/// `node-superseded`.
#[test]
fn adv_a_later_node_ref_value_naming_a_superseded_node_is_node_superseded() {
    each(|at, path, file| {
        let kernel = seeded(path, file);
        committed(
            &kernel,
            &transaction(vec![supersede(OTHER, KEPT)], &[STATEMENT]),
            100,
        );
        let verdict = submit(
            &kernel,
            &transaction(
                vec![GraphOperation::UpdateProperty(PropertyMutation {
                    node: id(KEPT),
                    property: id(RELATES),
                    values: vec![Value::NodeRef(id(OTHER))],
                })],
                &[],
            ),
            200,
        );
        assert_eq!(
            codes(&verdict),
            vec![(ValidatorName::Reference, "node-superseded".to_owned())],
            "{at}"
        );
    });
}

/// An assertion about an edge the same transaction retires is refused once, by the structural
/// check that owns the question (`validate/candidate.rs`: "an operation naming it is refused once,
/// by the structural check that owns that question"), not also as an `unresolved-edge` the head
/// does hold. A deletion of that edge likewise.
#[test]
fn adv_an_operation_on_an_edge_retired_in_its_transaction_is_refused_once() {
    each(|at, path, file| {
        let kernel = seeded(path, file);
        let named = vec![(ValidatorName::Structural, NAMED_IN_TRANSACTION.to_owned())];
        let deleted = submit(
            &kernel,
            &transaction(
                vec![
                    supersede(DUPLICATE, KEPT),
                    GraphOperation::DeleteEdge(id(DUPLICATE_TAGGED)),
                ],
                &[STATEMENT],
            ),
            100,
        );
        assert_eq!(codes(&deleted), named, "{at}: DeleteEdge");
        let asserted = submit(
            &kernel,
            &transaction(
                vec![supersede(DUPLICATE, KEPT), about_edge(DUPLICATE_TAGGED)],
                &[STATEMENT],
            ),
            200,
        );
        assert_eq!(codes(&asserted), named, "{at}: AddAssertion about the edge");
    });
}

/// Two nodes superseded together with an edge between them, the second by a node the same
/// transaction creates: the transaction commits, the edge is held in exactly one record, no head
/// edge names either, both aliases resolve to the kept nodes, and a later supersession of a kept
/// node repoints its record; every reopen gives the same roots.
#[test]
fn adv_two_nodes_superseded_together_with_an_edge_between_them() {
    each(|at, path, file| {
        let kernel = seeded(path, file);
        let fresh = NodeId::mint();
        committed(
            &kernel,
            &transaction(
                vec![
                    GraphOperation::CreateNode(node(fresh, TAG, "parser, kept", &["parser-kept"])),
                    supersede(DUPLICATE, KEPT),
                    supersede_ids(id(PARSER), fresh),
                ],
                &[STATEMENT],
            ),
            100,
        );
        let head = kernel.snapshot().unwrap();
        let edge: EdgeId = id(DUPLICATE_TAGGED);
        let holding: Vec<NodeId> = head
            .superseded
            .values()
            .filter(|record| record.edges.contains_key(&edge))
            .map(|record| record.node.id)
            .collect();
        assert_eq!(holding.len(), 1, "{at}: {holding:?}");
        assert!(head.edges.is_empty(), "{at}");
        let aliases = AliasIndex::of(&head);
        assert_eq!(aliases.nodes(id(TAG), "parser"), &[fresh], "{at}");
        assert_eq!(
            aliases.nodes(id("00000000-0000-4000-8000-000000000203"), "group/app!18"),
            &[id::<NodeId>(KEPT)],
            "{at}"
        );
        // A chain: KEPT by LATER, then LATER by a new WorkItem.
        committed(
            &kernel,
            &transaction(vec![supersede(KEPT, LATER)], &[STATEMENT]),
            200,
        );
        let last = NodeId::mint();
        committed(
            &kernel,
            &transaction(
                vec![
                    GraphOperation::CreateNode(node(last, WORK_ITEM, "last", &[])),
                    supersede_ids(id(LATER), last),
                ],
                &[STATEMENT],
            ),
            300,
        );
        let head = kernel.snapshot().unwrap();
        for superseded in [DUPLICATE, KEPT, LATER] {
            assert_eq!(
                head.superseded_by(id(superseded)),
                Some(last),
                "{at}: {superseded}"
            );
        }
        let aliases = AliasIndex::of(&head);
        for (type_id, alias) in [
            ("00000000-0000-4000-8000-000000000203", "group/app!18"),
            (WORK_ITEM, "group/app!18"),
            (WORK_ITEM, "tracker/18"),
        ] {
            assert_eq!(aliases.nodes(id(type_id), alias), &[last], "{at}: {alias}");
        }
        let recorded = roots(&kernel);
        drop(kernel);
        let reopened = open(path, file);
        assert_eq!(roots(&reopened), recorded, "{at}: reopened");
        drop(reopened);
        let mut full = open(path, file);
        full.set_full_replay(true);
        assert_eq!(roots(&full), recorded, "{at}: full replay");
        assert_eq!(full.snapshot().unwrap(), head, "{at}");
    });
}

/// Later operations naming a superseded node that the unit's own refusal case does not cover:
/// a MergeEntity of it, an Invoke on it and a Relation assertion naming it as its object are each
/// refused as `node-superseded`.
#[test]
fn adv_merge_invoke_and_an_object_naming_a_superseded_node_are_node_superseded() {
    each(|at, path, file| {
        let kernel = seeded(path, file);
        committed(
            &kernel,
            &transaction(vec![supersede(DUPLICATE, KEPT)], &[STATEMENT]),
            100,
        );
        let superseded = (ValidatorName::Reference, "node-superseded".to_owned());
        for (what, tx) in [
            (
                "MergeEntity",
                transaction(
                    vec![GraphOperation::MergeEntity(EntityMerge {
                        absorbed: id(DUPLICATE),
                        into: id(KEPT),
                        reason: None,
                        evidence: None,
                    })],
                    &[],
                ),
            ),
            (
                "Invoke",
                transaction(
                    vec![GraphOperation::Invoke {
                        node: id(DUPLICATE),
                        operation: "close".into(),
                        arguments: BTreeMap::new(),
                    }],
                    &[],
                ),
            ),
            (
                "an object",
                transaction(vec![relation(KEPT, DUPLICATE)], &[STATEMENT]),
            ),
        ] {
            let verdict = submit(&kernel, &tx, 200);
            assert!(
                codes(&verdict).contains(&superseded),
                "{at}: {what}: {:?}",
                codes(&verdict)
            );
        }
    });
}

/// The bytes of the checkpoint the newest pointer names.
fn newest_checkpoint(kernel: &Runtime, path: &Path, file: bool) -> (serde_json::Value, Vec<u8>) {
    let pointer = kernel
        .published_events()
        .unwrap()
        .into_iter()
        .filter(|event| event.name == "ekr.store.CheckpointWritten")
        .map(|event| event.data)
        .next_back()
        .expect("a checkpoint pointer");
    let digest = format!(
        "ekr.private.checkpoint.{}",
        pointer["checkpoint_hash"].as_str().unwrap()
    );
    let tenant = eventlog_core::TenantId::new(TENANT).unwrap();
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let bytes = executor
        .block_on(async {
            use eventlog_core::EventStore;
            if file {
                let provider = eventlog_file::FileEventStore::open(path).await.unwrap();
                provider.get_blob(&tenant, &digest).await.unwrap()
            } else {
                let provider = eventlog_sqlite::SqliteEventStore::open(
                    &path.join("state.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap();
                provider.get_blob(&tenant, &digest).await.unwrap()
            }
        })
        .expect("the newest checkpoint is retained");
    (pointer, bytes)
}

fn install(path: &Path, file: bool, covered: u64, binding: ContentHash, bytes: &[u8]) {
    if file {
        ekr_store::FileStore::file_existing(path, TENANT, None)
            .unwrap()
            .write_checkpoint(covered, binding, Some(bytes))
            .unwrap();
    } else {
        ekr_store::SqliteStore::sqlite_existing(&path.join("state.db"), TENANT, None)
            .unwrap()
            .write_checkpoint(covered, binding, Some(bytes))
            .unwrap();
    }
}

type Forge = fn(&mut serde_json::Value);

/// A checkpoint whose supersession records were changed is not taken: the open replays from the
/// seed and answers what a full replay answers. A store that never superseded writes no
/// `superseded` member at all.
#[test]
fn adv_a_forged_supersession_record_in_a_checkpoint_is_ignored() {
    each(|at, path, file| {
        {
            let plain = tempfile::tempdir().unwrap();
            let kernel = seeded(plain.path(), file);
            let (_, bytes) = newest_checkpoint(&kernel, plain.path(), file);
            let checkpoint: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert!(
                checkpoint["graph"]["graph"].get("superseded").is_none(),
                "{at}: {}",
                checkpoint["graph"]["graph"]
            );
        }
        let kernel = seeded(path, file);
        committed(
            &kernel,
            &transaction(vec![supersede(DUPLICATE, KEPT)], &[STATEMENT]),
            100,
        );
        let mut n = 0_i64;
        let checkpoint = loop {
            n += 1;
            committed(
                &kernel,
                &transaction(
                    vec![GraphOperation::CreateNode(node(
                        NodeId::mint(),
                        TAG,
                        &format!("tag {n}"),
                        &[],
                    ))],
                    &[],
                ),
                100 + 10 * n,
            );
            let (pointer, bytes) = newest_checkpoint(&kernel, path, file);
            let checkpoint: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            if checkpoint["graph"]["graph"].get("superseded").is_some() {
                break (pointer, checkpoint);
            }
            assert!(n < 20, "{at}: no checkpoint holds the supersession");
        };
        drop(kernel);
        let (pointer, genuine) = checkpoint;
        let covered = pointer["covered"].as_u64().unwrap();
        let binding: ContentHash = serde_json::from_value(pointer["binding"].clone()).unwrap();
        let truth = {
            let mut full = open(path, file);
            full.set_full_replay(true);
            (full.head().unwrap(), full.snapshot().unwrap())
        };
        let genuine_open = open(path, file);
        assert_eq!(
            genuine_open.seed_replays(),
            0,
            "{at}: the genuine one is taken"
        );
        drop(genuine_open);
        fn record(forged: &mut serde_json::Value) -> &mut serde_json::Value {
            let records = forged["graph"]["graph"]["superseded"]
                .as_object_mut()
                .unwrap();
            let first = records.keys().next().unwrap().clone();
            &mut records[&first]
        }
        let forgeries: Vec<(&str, Forge)> = vec![
            ("another by", |forged| {
                record(forged)["by"] = LATER.into();
            }),
            ("another reason", |forged| {
                record(forged)["reason"] = "something else".into();
            }),
            ("another revision", |forged| {
                record(forged)["revision"] = 3.into();
            }),
            ("a dropped retired edge", |forged| {
                record(forged)["edges"] = serde_json::json!({});
            }),
            ("another node name", |forged| {
                record(forged)["node"]["canonical_name"] = "renamed".into();
            }),
            ("no record", |forged| {
                forged["graph"]["graph"]
                    .as_object_mut()
                    .unwrap()
                    .remove("superseded");
            }),
        ];
        for (name, forge) in forgeries {
            let mut forged = genuine.clone();
            forge(&mut forged);
            assert_ne!(forged, genuine, "{at}: {name}");
            install(
                path,
                file,
                covered,
                binding,
                &serde_json::to_vec(&forged).unwrap(),
            );
            let opened = open(path, file);
            assert_eq!(
                (opened.head().unwrap(), opened.snapshot().unwrap()),
                truth,
                "{at}: a checkpoint with {name} is ignored"
            );
            assert!(
                opened.seed_replays() > 0,
                "{at}: {name}: replayed from the seed"
            );
        }
    });
}

/// Design § 110.3: "a `CreateNode` or `CreateEdge` reusing its id, or a retired edge's, is
/// `identity-already-exists` under every profile".
#[test]
fn adv_a_superseded_nodes_id_and_a_retired_edges_id_are_not_taken_again() {
    each(|at, path, file| {
        let kernel = seeded(path, file);
        committed(
            &kernel,
            &transaction(vec![supersede(DUPLICATE, KEPT)], &[STATEMENT]),
            100,
        );
        let taken = (
            ValidatorName::Structural,
            "identity-already-exists".to_owned(),
        );
        let node_again = submit(
            &kernel,
            &transaction(
                vec![GraphOperation::CreateNode(node(
                    id(DUPLICATE),
                    TAG,
                    "again",
                    &[],
                ))],
                &[],
            ),
            200,
        );
        assert!(
            codes(&node_again).contains(&taken),
            "{at}: {:?}",
            codes(&node_again)
        );
        let edge_again = submit(
            &kernel,
            &transaction(
                vec![GraphOperation::CreateEdge(EdgeDraft {
                    id: id(DUPLICATE_TAGGED),
                    root_id: id(ROOT),
                    type_id: id(TAGGED),
                    source: id(KEPT),
                    target: id(PARSER),
                    properties: BTreeMap::new(),
                })],
                &[],
            ),
            300,
        );
        assert!(
            codes(&edge_again).contains(&taken),
            "{at}: {:?}",
            codes(&edge_again)
        );
    });
}

/// Design § 110.4: "a seed whose `superseded` is not empty is refused as `seed-superseded`".
#[test]
fn adv_a_seed_holding_a_supersession_record_is_refused() {
    each(|at, path, file| {
        let kernel = open(path, file);
        let mut document = seed();
        let other: NodeId = id(OTHER);
        let held = document.graph.nodes.remove(&other).unwrap();
        document.graph.superseded.insert(
            other,
            ekr_graph::SupersededNode {
                node: held,
                edges: BTreeMap::new(),
                by: id(KEPT),
                revision: RevisionNumber::SEED,
                reason: REASON.to_owned(),
                evidence: BTreeSet::from([id::<EvidenceId>(STATEMENT)]),
                merge: None,
            },
        );
        let refused = kernel
            .seed(document, || Timestamp::from_millis(10))
            .expect_err("a seed with a supersession record is refused");
        assert!(
            format!("{refused:?}").contains("seed-superseded"),
            "{at}: {refused:?}"
        );
        assert!(kernel.head().unwrap().is_none(), "{at}");
    });
}
