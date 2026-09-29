//! A commit hashes the graph once (`story:commit-hashes-the-graph-once`), on both providers.
//!
//! The kernel applies a validated transaction to the head and computes the new root when it
//! decides the commit, and the store then admits the staged candidate by replaying it through the
//! same kernel. That replay reuses the kernel's own result for the candidate instead of applying
//! and hashing the graph a second time; the root is still the kernel's (invariant 1). The count
//! is [`ekr_store::knowledge_roots_hashed`]: every knowledge root computed on this thread, which
//! is every full-graph hash.
//!
//! Reuse changes no root: a fresh runtime replaying the whole lineage from the seed reaches the
//! roots the commits published.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TENANT: &str = "hashes-once";

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
/// A seed with one node type and one evidence payload.
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
/// One node with one evidenced label.
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
/// Proposes and validates revision `n` through `runtime`, then commits it, returning the
/// published root and how many full-graph hashes the commit alone computed.
fn commit(runtime: &Runtime, seed: &SeedDocument, n: u64) -> (Root, u64) {
    let at = i64::try_from(n * 100).unwrap();
    let (tx, bytes) = document(seed, n);
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
    let before = ekr_store::knowledge_roots_hashed();
    let result = runtime
        .commit(tx, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap();
    let hashed = ekr_store::knowledge_roots_hashed() - before;
    let CommitCommandResult::Committed(receipt) = result else {
        panic!("{result:?}");
    };
    (receipt.result, hashed)
}

#[test]
fn a_commit_hashes_the_graph_once() {
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = seed();
        let runtime = open(path, file);
        runtime
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let mut published = Vec::new();
        for n in 1..=4 {
            let (root, hashed) = commit(&runtime, &seed, n);
            assert_eq!(hashed, 1, "commit {n}; file={file}");
            published.push(root);
        }
        // A fresh runtime, as the next `ekr` call is, commits with one full-graph hash too.
        let fresh = open(path, file);
        fresh.read(None).unwrap();
        let (root, hashed) = commit(&fresh, &seed, 5);
        assert_eq!(hashed, 1, "commit through a fresh runtime; file={file}");
        published.push(root);

        // Replay equality: every published root is the one a full replay reaches.
        let mut replayed = open(path, file);
        replayed.set_full_replay(true);
        assert_eq!(replayed.head().unwrap(), published.last().copied());
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
