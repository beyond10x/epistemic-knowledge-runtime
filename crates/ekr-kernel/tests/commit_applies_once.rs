//! A commit applies its transaction to the graph once (`task:commit-applies-once`), on both
//! providers.
//!
//! The kernel applies a validated transaction to the head graph when it decides the commit, and
//! the store then admits the staged candidate by replaying it through the same kernel. That replay
//! takes the graph the decision applied, keyed exactly as the remembered root is, instead of
//! cloning the head and applying the transaction a second time; the graph and the root are still
//! the kernel's (invariant 1). The count is [`ekr_kernel::graphs_applied`]: every clone-and-apply
//! of a head graph on this thread.
//!
//! Taking the graph changes no root. Every id and instant below is fixed, so the head a history
//! reaches is a pure function of its transactions; [`BASE_HEAD`] is the root the kernel reached for
//! this history before the change, and a fresh runtime replaying the lineage from the seed reaches
//! every root the commits published.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TENANT: &str = "applies-once";

/// `ContentHash::of` the head root of [`history`], as the kernel at `d1678844` (the wave's base,
/// before this change) computed it on both providers. The root chains its parent's hash, so this
/// one literal holds every root of the lineage.
const BASE_HEAD: &str = "cbb18047899672102bab47ffa26e36e64c87e08435aaedbe708074f0e6a3d496";

fn id<T: std::str::FromStr>(kind: u16, n: u64) -> T
where
    T::Err: std::fmt::Debug,
{
    format!("00000000-{kind:04x}-4000-8000-{n:012x}")
        .parse()
        .unwrap()
}
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
fn open(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), anchor())
    }
    .unwrap()
}
fn type_id() -> TypeId {
    "00000000-0000-4000-8000-000000000005".parse().unwrap()
}
fn label() -> PropertyId {
    "00000000-0000-4000-8000-000000000006".parse().unwrap()
}
fn evidence() -> EvidenceId {
    "00000000-0000-4000-8000-000000000007".parse().unwrap()
}
/// A seed with one node type and one evidence payload, every id fixed.
fn seed() -> SeedDocument {
    let mut seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let mut declared = NodeType::new(type_id(), "Subject");
    declared.properties.insert(
        label(),
        PropertyDefinition::new(label(), "label", ValueType::String),
    );
    seed.ontology.node_types.push(declared);
    let bytes = vec![7_u8; 4096];
    let hash = ContentHash::of_bytes(&bytes);
    let entry = Evidence {
        id: evidence(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::from_basis_points(10000).unwrap(),
    };
    seed.graph.evidence.insert(entry.id, entry);
    seed.evidence_payloads.insert(hash, bytes.into());
    seed
}
fn assertion(n: u64, subject: NodeId) -> GraphOperation {
    GraphOperation::AddAssertion(Box::new(Assertion {
        id: id(5, n),
        root_id: seed().graph.root.id,
        subject: Subject::Node(subject),
        predicate: Predicate::Property(label()),
        object: Object::Value(Value::String(format!("label {n}"))),
        evidence: BTreeSet::from([evidence()]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }))
}
/// Revision `n`'s transaction: odd revisions create a node with an evidenced label, even ones
/// update the label of the node the revision before created and retract its assertion, so the
/// history exercises creation, property change and retraction at a commit instant.
fn transaction(n: u64) -> GraphTransaction {
    let node: NodeId = id(3, n);
    let operations = if n % 2 == 1 {
        vec![
            GraphOperation::CreateNode(NodeDraft {
                id: node,
                root_id: seed().graph.root.id,
                type_id: type_id(),
                canonical_name: format!("subject {n}"),
                properties: BTreeMap::new(),
                aliases: vec![format!("subject-{n}")],
            }),
            assertion(n, node),
        ]
    } else {
        vec![
            GraphOperation::UpdateProperty(PropertyMutation {
                node: id(3, n - 1),
                property: label(),
                values: vec![Value::String(format!("updated {n}"))],
            }),
            GraphOperation::RetractAssertion(Retraction {
                assertion: id(5, n - 1),
                reason: RetractionReason::new(format!("withdrawn at {n}")),
            }),
        ]
    };
    GraphTransaction {
        id: id(9, n),
        proposer: context().operator,
        evidence: if n % 2 == 1 {
            BTreeSet::from([evidence()])
        } else {
            BTreeSet::new()
        },
        operations,
        schema_version: None,
    }
}
fn document(n: u64) -> (TransactionId, Vec<u8>) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let tx = transaction(n);
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: &tx,
    })
    .unwrap()
    .into_bytes();
    (tx.id, bytes)
}
/// What the commit command alone did on this thread: head graphs cloned and applied, and whole
/// graphs hashed.
#[derive(Debug, PartialEq, Eq)]
struct Work {
    applied: u64,
    hashed: u64,
}
/// Proposes, validates and commits revision `n` through `runtime`, returning the published root
/// and the work the commit command alone did.
fn commit(runtime: &Runtime, n: u64) -> (Root, Work) {
    let at = i64::try_from(n * 100).unwrap();
    let (tx, bytes) = document(n);
    runtime
        .propose(&bytes, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    let verdict = runtime
        .validate(tx, RevisionNumber::new(n - 1), || {
            Timestamp::from_millis(at + 1)
        })
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let (applied, hashed) = (
        ekr_kernel::graphs_applied(),
        ekr_store::knowledge_roots_hashed(),
    );
    let result = runtime
        .commit(tx, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap();
    let work = Work {
        applied: ekr_kernel::graphs_applied() - applied,
        hashed: ekr_store::knowledge_roots_hashed() - hashed,
    };
    let CommitCommandResult::Committed(receipt) = result else {
        panic!("{result:?}");
    };
    (receipt.result, work)
}
/// The fixed history: revisions 1 to 4 through one runtime and revision 5 through a fresh one,
/// as the next `ekr` call is. Returns every published root with the work its commit did.
fn history(path: &Path, file: bool) -> Vec<(Root, Work)> {
    let runtime = open(path, file);
    runtime.seed(seed(), || Timestamp::from_millis(10)).unwrap();
    let mut published: Vec<(Root, Work)> = (1..=4).map(|n| commit(&runtime, n)).collect();
    let fresh = open(path, file);
    fresh.read(None).unwrap();
    published.push(commit(&fresh, 5));
    published
}

#[test]
fn a_commit_clones_and_applies_the_head_graph_once() {
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        for (root, work) in history(directory.path(), file) {
            assert_eq!(
                work,
                Work {
                    applied: 1,
                    hashed: 1
                },
                "revision {}; file={file}",
                root.revision
            );
        }
    }
}

#[test]
fn roots_are_the_ones_the_kernel_reached_before_the_change_and_replay_reaches_them() {
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let published: Vec<Root> = history(path, file)
            .into_iter()
            .map(|(root, _)| root)
            .collect();
        let head = *published.last().unwrap();
        assert_eq!(
            ContentHash::of(&head).to_hex(),
            BASE_HEAD,
            "file={file}; head {head:?}"
        );

        // Replay equality: every published root is the one a full replay from the seed reaches.
        let mut replayed = open(path, file);
        replayed.set_full_replay(true);
        assert_eq!(replayed.head().unwrap(), Some(head));
        for root in &published {
            let graph = replayed.replay(root.revision).unwrap();
            assert_eq!(
                ekr_store::knowledge_root(&graph),
                root.knowledge_root,
                "revision {}; file={file}",
                root.revision
            );
            assert_eq!(ekr_store::evidence_root(&graph), root.evidence_root);
        }
    }
}
