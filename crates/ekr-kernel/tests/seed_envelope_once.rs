//! The retained seed envelope is decoded in full at most once per kernel authority
//! (`story:seed-envelope-decoded-once`), on both providers.
//!
//! One `ekr` invocation opens one runtime. Across a checkpoint restore, verified reads at the head
//! and at an earlier revision, and a complete propose, validate and commit, that runtime decodes
//! the envelope's full bytes once: every later path takes the envelope it already holds for the
//! same `seed_hash`, and the checkpoint restore keeps only the payload keys and the seed graph.
//! A full replay decodes it once too, and a runtime that writes the seed decodes the staged bytes
//! once, in the replay that admits the publication. The count is the kernel's own,
//! [`Runtime::seed_envelope_decodes`].
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TENANT: &str = "envelope-once";

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
/// A seed with one node type and three evidence payloads.
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
    for n in 0..3_u8 {
        let bytes = vec![n; 4096];
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
        seed.evidence_payloads.insert(hash, bytes);
    }
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
/// Proposes, validates against `n - 1` and commits revision `n` through `runtime`.
fn commit(runtime: &Runtime, seed: &SeedDocument, n: u64) {
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
    let result = runtime
        .commit(tx, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap();
    assert!(
        matches!(result, CommitCommandResult::Committed(_)),
        "{result:?}"
    );
}
/// Seeds and commits revision 1, each command through a fresh runtime as one `ekr` call does.
fn build(path: &Path, file: bool) -> SeedDocument {
    let seed = seed();
    open(path, file)
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    let at = 100;
    let (tx, bytes) = document(&seed, 1);
    open(path, file)
        .propose(&bytes, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    open(path, file)
        .validate(tx, RevisionNumber::SEED, || Timestamp::from_millis(at + 1))
        .unwrap();
    open(path, file)
        .commit(tx, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap();
    seed
}
/// Everything the runtime does in the count: restore and a verified read of the head, a read of
/// the seed revision, a complete commit, and a read of the new head with its seed payloads.
fn exercise(runtime: &Runtime, seed: &SeedDocument) {
    let read = runtime.read(None).unwrap();
    assert_eq!(read.root.revision, RevisionNumber::new(1));
    assert_eq!(read.seed_input, *seed);
    assert_eq!(
        runtime.read(Some(RevisionNumber::SEED)).unwrap().seed_input,
        *seed
    );
    commit(runtime, seed, 2);
    let read = runtime.read(None).unwrap();
    assert_eq!(read.root.revision, RevisionNumber::new(2));
    assert_eq!(read.seed_input, *seed);
    for (hash, bytes) in &seed.evidence_payloads {
        assert_eq!(read.content(hash), Some(bytes.as_slice()));
    }
}

#[test]
fn one_runtime_decodes_the_seed_envelope_once_across_restore_read_and_commit() {
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = build(path, file);
        let runtime = open(path, file);
        runtime.read(None).unwrap();
        assert_eq!(
            runtime.seed_replays(),
            0,
            "the checkpoint was restored; file={file}"
        );
        exercise(&runtime, &seed);
        assert_eq!(runtime.seed_envelope_decodes(), 1, "file={file}");
    }
}

#[test]
fn a_full_replay_also_decodes_the_seed_envelope_once() {
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = build(path, file);
        let mut runtime = open(path, file);
        runtime.set_full_replay(true);
        exercise(&runtime, &seed);
        assert!(runtime.seed_replays() > 0, "file={file}");
        assert_eq!(runtime.seed_envelope_decodes(), 1, "file={file}");
    }
}

#[test]
fn a_runtime_that_writes_the_seed_decodes_the_staged_envelope_once() {
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = seed();
        let runtime = open(path, file);
        runtime
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        commit(&runtime, &seed, 1);
        assert_eq!(runtime.read(None).unwrap().seed_input, seed);
        // The replay that admits the publication decodes the staged bytes, not the envelope
        // the handle built in memory (invariant 1); every later path takes that decode.
        assert_eq!(runtime.seed_envelope_decodes(), 1, "file={file}");
    }
}
