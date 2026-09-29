//! Adversary, wave perf-02, `story:commit-hashes-the-graph-once`: the per-thread root the kernel
//! remembers does not cross from one store to another.
//!
//! Two stores seeded with the same seed document commit the same transaction documents, from one
//! thread, alternately, at different commit times. Every input the remembered root is keyed by
//! except the prior graph's allocation and the commit time is then equal between a commit in one
//! store and the next commit in the other; a key that dropped either would hand the second store
//! the first store's root. Runtimes are opened and dropped between rounds so graph allocations are
//! freed and reused. Each published root is held against a full replay in a fresh runtime.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TENANT: &str = "memo-across-stores";

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
fn seed() -> SeedDocument {
    let mut seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let type_id = "00000000-0000-4000-8000-000000000005".parse().unwrap();
    let label = "00000000-0000-4000-8000-000000000006".parse().unwrap();
    let mut declared = NodeType::new(type_id, "Subject");
    declared.properties.insert(
        label,
        PropertyDefinition::new(label, "label", ValueType::String),
    );
    seed.ontology.node_types.push(declared);
    let bytes = vec![7_u8; 4096];
    let hash = ContentHash::of_bytes(&bytes);
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::from_basis_points(10000).unwrap(),
    };
    seed.graph.evidence.insert(evidence.id, evidence);
    seed.evidence_payloads.insert(hash, bytes.into());
    seed
}
fn document(seed: &SeedDocument, n: u64) -> (TransactionId, Vec<u8>) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let ty = &seed.ontology.node_types[0];
    let label = *ty.properties.keys().next().unwrap();
    let evidence = *seed.graph.evidence.keys().next().unwrap();
    let id = NodeId::mint();
    let root = seed.graph.root.id;
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![
            GraphOperation::CreateNode(NodeDraft {
                id,
                root_id: root,
                type_id: ty.id,
                canonical_name: format!("subject {n}"),
                properties: BTreeMap::new(),
                aliases: Vec::new(),
            }),
            GraphOperation::AddAssertion(Box::new(Assertion {
                id: AssertionId::mint(),
                root_id: root,
                subject: Subject::Node(id),
                predicate: Predicate::Property(label),
                object: Object::Value(Value::String(format!("label {n}"))),
                evidence: BTreeSet::from([evidence]),
                proposed_by: context().operator,
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time: TemporalRange::UNBOUNDED,
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            })),
        ],
        evidence: BTreeSet::from([evidence]),
        schema_version: None,
    };
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: &tx,
    })
    .unwrap()
    .into_bytes();
    (tx.id, bytes)
}
fn prepare(runtime: &Runtime, tx: TransactionId, bytes: &[u8], n: u64, at: i64) {
    runtime
        .propose(bytes, context().operator, || Timestamp::from_millis(at))
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
}
fn commit(runtime: &Runtime, tx: TransactionId, at: i64) -> (Root, u64) {
    let before = ekr_store::knowledge_roots_hashed();
    let result = runtime
        .commit(tx, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    let hashed = ekr_store::knowledge_roots_hashed() - before;
    let CommitCommandResult::Committed(receipt) = result else {
        panic!("{result:?}");
    };
    (receipt.result, hashed)
}
fn replayed(path: &Path, file: bool, published: &[Root]) {
    let mut fresh = open(path, file);
    fresh.set_full_replay(true);
    assert_eq!(fresh.head().unwrap(), published.last().copied());
    for root in published {
        let graph = fresh.replay(root.revision).unwrap();
        assert_eq!(
            ekr_store::knowledge_root(&graph),
            root.knowledge_root,
            "revision {}; file={file}",
            root.revision
        );
    }
}

#[test]
fn a_remembered_root_never_crosses_to_another_store_on_the_same_thread() {
    let seed = seed();
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    let a = open(left.path(), true);
    let b = open(right.path(), false);
    a.seed(seed.clone(), || Timestamp::from_millis(10)).unwrap();
    b.seed(seed.clone(), || Timestamp::from_millis(10)).unwrap();
    assert_eq!(a.head().unwrap(), b.head().unwrap(), "the two seeds agree");

    let (mut in_a, mut in_b) = (Vec::new(), Vec::new());
    for n in 1..=4_u64 {
        let at = i64::try_from(n * 100).unwrap();
        let (tx, bytes) = document(&seed, n);
        prepare(&a, tx, &bytes, n, at);
        prepare(&b, tx, &bytes, n, at);
        // `b` commits first on odd rounds, so each store follows the other at least once.
        let order: [(&Runtime, i64, bool); 2] = if n % 2 == 1 {
            [(&b, at + 9, false), (&a, at + 2, true)]
        } else {
            [(&a, at + 2, true), (&b, at + 9, false)]
        };
        for (runtime, commit_at, is_a) in order {
            let (root, hashed) = commit(runtime, tx, commit_at);
            assert_eq!(hashed, 1, "round {n}; store a={is_a}");
            if is_a {
                in_a.push(root);
            } else {
                in_b.push(root);
            }
        }
        let (ra, rb) = (in_a.last().unwrap(), in_b.last().unwrap());
        assert_eq!(ra.revision, rb.revision);
        assert_eq!(ra.transaction, rb.transaction, "the same transaction");
        assert_ne!(
            ra.knowledge_root, rb.knowledge_root,
            "round {n}: two commit times, so two knowledge roots"
        );
        // Open and drop runtimes over both stores, freeing graph allocations for reuse.
        drop(open(left.path(), true).read(None).unwrap());
        drop(open(right.path(), false).read(None).unwrap());
    }
    replayed(left.path(), true, &in_a);
    replayed(right.path(), false, &in_b);
}
