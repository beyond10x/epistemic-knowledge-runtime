//! Replay checkpoints and compact records, design § 96, on both providers.
//!
//! A commit leaves one checkpoint of the head it published; a fresh open continues from it and
//! answers exactly as a full replay from the seed does; a checkpoint that does not verify is
//! ignored; a validation against an earlier revision, whose graph a checkpoint does not hold,
//! replays in full; and every record and preparation a command retains spells its byte strings
//! as base64 and holds each staged object once.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use ekr_store::RevisionLog;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TENANT: &str = "checkpoint";

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
fn open_in_full(path: &Path, file: bool) -> Runtime {
    let mut runtime = open(path, file);
    runtime.set_full_replay(true);
    runtime
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
    let bytes = b"synthetic human evidence".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let evidence = Evidence {
        id: "00000000-0000-4000-8000-000000000007".parse().unwrap(),
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
    seed
}
/// `nodes` new nodes, each with one evidenced assertion.
fn document(seed: &SeedDocument, n: u64, nodes: u64) -> (TransactionId, Vec<u8>) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let ty = &seed.ontology.node_types[0];
    let label = *ty.properties.keys().next().unwrap();
    let evidence = *seed.graph.evidence.keys().next().unwrap();
    let mut operations = Vec::new();
    for k in 0..nodes {
        let id = NodeId::mint();
        operations.push(GraphOperation::CreateNode(NodeDraft {
            id,
            root_id: seed.graph.root.id,
            type_id: ty.id,
            canonical_name: format!("subject {n}.{k}"),
            properties: BTreeMap::new(),
        }));
        operations.push(GraphOperation::AddAssertion(Box::new(Assertion {
            id: AssertionId::mint(),
            root_id: seed.graph.root.id,
            subject: Subject::Node(id),
            predicate: Predicate::Property(label),
            object: Object::Value(Value::String(format!("label {n}.{k}"))),
            evidence: BTreeSet::from([evidence]),
            proposed_by: context().operator,
            assessment: Assessment::Proposed,
            lifecycle: AssertionLifecycle::Active,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        })));
    }
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
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
/// Seeds and commits `commits` transactions, each command through a fresh runtime as one `ekr`
/// invocation does. Returns the seed.
fn build(path: &Path, file: bool, commits: u64) -> SeedDocument {
    let seed = seed();
    open(path, file)
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    for revision in 1..=commits {
        commit(path, file, &seed, revision, revision - 1);
    }
    seed
}
fn commit(
    path: &Path,
    file: bool,
    seed: &SeedDocument,
    n: u64,
    against: u64,
) -> CommitCommandResult {
    let at = i64::try_from(n * 100).unwrap();
    let (tx, bytes) = document(seed, n, 3);
    open(path, file)
        .propose(&bytes, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    let verdict = open(path, file)
        .validate(tx, RevisionNumber::new(against), || {
            Timestamp::from_millis(at + 1)
        })
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    open(path, file)
        .commit(tx, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap()
}
type Answers = (
    Option<Root>,
    CanonicalGraph,
    BTreeMap<TransactionId, TransactionRecord>,
);
fn answers(runtime: &Runtime) -> Answers {
    (
        runtime.head().unwrap(),
        runtime.snapshot().unwrap(),
        runtime.transactions().unwrap(),
    )
}
/// Every `ekr.store.CheckpointWritten` pointer the log holds.
fn pointers(runtime: &Runtime) -> Vec<serde_json::Value> {
    runtime
        .published_events()
        .unwrap()
        .into_iter()
        .filter(|event| event.name == "ekr.store.CheckpointWritten")
        .map(|event| {
            assert_eq!(event.stream_type, "ekr.checkpoint");
            event.data
        })
        .collect()
}
fn revision_occurrences(runtime: &Runtime) -> u64 {
    runtime
        .published_events()
        .unwrap()
        .iter()
        .filter(|event| event.stream_type == "ekr.revision")
        .count() as u64
}
/// The bytes of the checkpoint the newest pointer names, read from the provider itself.
fn checkpoint_bytes(path: &Path, file: bool, pointer: &serde_json::Value) -> Vec<u8> {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let digest = format!(
        "ekr.private.checkpoint.{}",
        pointer["checkpoint_hash"].as_str().unwrap()
    );
    let tenant = eventlog_core::TenantId::new(TENANT).unwrap();
    executor
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
        .expect("the newest checkpoint is retained")
}
/// Retains `bytes` as the newest checkpoint through the store's own writer, as a store handle
/// with no kernel authority: the store neither checks nor could check what a checkpoint says.
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

#[test]
fn a_fresh_open_continues_from_the_checkpoint_with_the_answers_of_a_full_replay() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        build(path, file, 4);
        let checkpointed = answers(&open(path, file));
        let replayed = answers(&open_in_full(path, file));
        assert_eq!(checkpointed, replayed, "file={file}");
        assert_eq!(checkpointed.0.unwrap().revision, RevisionNumber::new(4));

        let runtime = open(path, file);
        let written = pointers(&runtime);
        assert_eq!(
            written.len(),
            1 + 4 * 3,
            "file={file}: the seed and each proposal, validation and commit leave one"
        );
        let blobs: BTreeSet<String> = written
            .iter()
            .map(|pointer| pointer["checkpoint_hash"].to_string())
            .collect();
        assert_eq!(
            blobs.len(),
            5,
            "file={file}: only the seed and the commits write one"
        );
        assert_eq!(
            written.last().unwrap()["covered"].as_u64(),
            Some(revision_occurrences(&runtime)),
            "file={file}: the newest covers the whole revision stream"
        );
        if file {
            let retained = std::fs::read_dir(path.join("blobs"))
                .unwrap()
                .filter(|entry| {
                    std::fs::read(entry.as_ref().unwrap().path())
                        .unwrap()
                        .starts_with(br#"{"format":"ekr.replay-checkpoint/1""#)
                })
                .count();
            assert_eq!(retained, 1, "a newer checkpoint replaces the one before it");
        }
    }
}

#[test]
fn a_checkpoint_that_does_not_verify_is_ignored_and_the_history_replays_in_full() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        build(path, file, 2);
        let truth = answers(&open_in_full(path, file));
        let newest = pointers(&open(path, file)).pop().unwrap();
        let genuine: serde_json::Value =
            serde_json::from_slice(&checkpoint_bytes(path, file, &newest)).unwrap();
        // The genuine pointer's coverage and binding, so that only the checkpoint is forged.
        let covered = newest["covered"].as_u64().unwrap();
        let binding: ContentHash = serde_json::from_value(newest["binding"].clone()).unwrap();
        type Forge = fn(&mut serde_json::Value);
        let forgeries: [(&str, Forge); 4] = [
            ("a dropped assertion", |forged| {
                let assertions = forged["graph"]["graph"]["assertions"]
                    .as_object_mut()
                    .unwrap();
                let first = assertions.keys().next().unwrap().clone();
                assertions.remove(&first);
            }),
            ("another host", |forged| {
                forged["authority"] = serde_json::to_value(ContentHash::of_bytes(b"x")).unwrap();
            }),
            ("another history", |forged| {
                forged["prefix"] = serde_json::to_value(ContentHash::of_bytes(b"y")).unwrap();
            }),
            ("an earlier head", |forged| {
                forged["revision"] = 1.into();
            }),
        ];
        for (name, forge) in forgeries {
            let mut forged = genuine.clone();
            forge(&mut forged);
            let bytes = serde_json::to_vec(&forged).unwrap();
            install(path, file, covered, binding, &bytes);
            assert_eq!(
                pointers(&open(path, file)).pop().unwrap()["checkpoint_hash"],
                serde_json::to_value(ContentHash::of_bytes(&bytes)).unwrap(),
                "file={file}: the forgery with {name} is the newest checkpoint"
            );
            assert_eq!(
                answers(&open(path, file)),
                truth,
                "file={file}: a checkpoint with {name} is ignored"
            );
        }
        // A pointer whose binding is not this host's own, and one that claims more than the
        // stream holds, are not taken as a verification of the head.
        let original = serde_json::to_vec(&genuine).unwrap();
        for (claimed, binding) in [
            (covered, ContentHash::of_bytes(b"another binding")),
            (covered + 1, binding),
        ] {
            install(path, file, claimed, binding, &original);
            assert_eq!(answers(&open(path, file)), truth, "file={file}");
        }
    }
}

#[test]
fn validating_against_a_revision_the_checkpoint_holds_no_graph_of_replays_in_full() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = build(path, file, 3);
        // Validated against revision 1 while the head is 3: the commit is stale, and both the
        // validation and the stale decision agree with a full replay.
        let result = commit(path, file, &seed, 4, 1);
        assert!(
            matches!(result, CommitCommandResult::Stale(_)),
            "file={file}: {result:?}"
        );
        assert_eq!(
            answers(&open(path, file)),
            answers(&open_in_full(path, file)),
            "file={file}"
        );
    }
}

/// Every retained proposal and commit receipt is the current format, its document bytes one
/// base64 string; every preparation holds each staged object once, as base64, and no separate
/// blob list; and a preparation is not much larger than the objects it stages.
#[test]
fn retained_records_and_preparations_hold_each_payload_once_as_base64() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    build(path, true, 2);
    let mut formats = BTreeMap::<String, usize>::new();
    for entry in std::fs::read_dir(path.join("blobs")).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        let Some(format) = value["format"].as_str() else {
            continue;
        };
        *formats.entry(format.to_owned()).or_default() += 1;
        match format {
            "ekr.proposal-record/2" => assert!(value["document_bytes"].is_string()),
            "ekr.commit-receipt/2" => {
                assert!(value["proposal"]["document_bytes"].is_string());
            }
            "ekr.publication-preparation/2" => {
                assert!(value["native_request"].get("blobs").is_none());
                let objects = value["decision"]["objects"].as_object().unwrap();
                let mut staged = 0;
                for object in objects.values() {
                    staged += object["bytes"].as_str().unwrap().len();
                }
                assert!(
                    bytes.len() < staged + 8192,
                    "a preparation of {} bytes stages {staged} bytes of base64",
                    bytes.len()
                );
            }
            _ => {}
        }
    }
    for format in [
        "ekr.proposal-record/2",
        "ekr.commit-receipt/2",
        "ekr.publication-preparation/2",
    ] {
        assert!(formats.contains_key(format), "{format} in {formats:?}");
    }
    for format in [
        "ekr.proposal-record/1",
        "ekr.commit-receipt/1",
        "ekr.publication-preparation/1",
    ] {
        assert!(!formats.contains_key(format), "{format} in {formats:?}");
    }
}
