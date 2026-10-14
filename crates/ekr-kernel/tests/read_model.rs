//! One-shot reads served from the persisted read model, design § 108, on both providers.
//!
//! `story:reads-served-from-a-persisted-read-model`. A one-shot read — a fresh handle opened as
//! one `ekr snapshot` or `ekr ontology` invocation opens it — counts what it decodes and loads:
//! replays begun at the seed, retained records decoded, transaction documents parsed, objects
//! loaded and blobs hashed. No case here reads a clock.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, ValueType};
use serde::Serialize;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TENANT: &str = "read-model";

thread_local! {
    /// Whether the stores this test thread opens admit schema changes (profile v2) rather than
    /// run profile v1. Each test runs on its own thread, so a case that sets it affects only itself.
    static EVOLVING: Cell<bool> = const { Cell::new(false) };
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
        validation_profile: if EVOLVING.get() {
            ValidationProfileV1::schema_evolving(c.validator)
        } else {
            ValidationProfileV1::deterministic(c.validator)
        },
    }
}
/// A writing handle, as `ekr propose`, `validate` and `commit` open one.
fn open(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), anchor())
    }
    .unwrap()
}
/// A reading handle, as `ekr snapshot` and `ekr ontology` open one.
fn reader(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file_reading(path, TENANT, context(), anchor())
    } else {
        Runtime::sqlite_reading(&path.join("state.db"), TENANT, context(), anchor())
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
    seed.evidence_payloads.insert(hash, bytes.into());
    seed
}
/// `nodes` new nodes, each with one evidenced assertion.
fn document(seed: &SeedDocument, n: u64, nodes: u64) -> (TransactionId, Vec<u8>) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let root = seed.graph.root.id;
    let ty = &seed.ontology.node_types[0];
    let label = *ty.properties.keys().next().unwrap();
    let evidence = *seed.graph.evidence.keys().next().unwrap();
    let mut operations = Vec::new();
    for k in 0..nodes {
        let id = NodeId::mint();
        operations.push(GraphOperation::CreateNode(NodeDraft {
            id,
            root_id: root,
            type_id: ty.id,
            canonical_name: format!("subject {n}.{k}"),
            properties: BTreeMap::new(),
            aliases: Vec::new(),
        }));
        operations.push(GraphOperation::AddAssertion(Box::new(Assertion {
            id: AssertionId::mint(),
            root_id: root,
            subject: Subject::Node(id),
            predicate: Predicate::Property(label),
            object: Object::Value(ekr_ontology::Value::String(format!("label {n}.{k}"))),
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
/// Proposes revision `n`'s document through a fresh writing handle.
fn propose(path: &Path, file: bool, seed: &SeedDocument, n: u64, nodes: u64) -> TransactionId {
    let at = i64::try_from(n * 100).unwrap();
    let (tx, bytes) = document(seed, n, nodes);
    open(path, file)
        .propose(&bytes, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    tx
}
/// Proposes, validates and commits revision `n`, each command through a fresh writing handle.
fn commit(path: &Path, file: bool, seed: &SeedDocument, n: u64, nodes: u64) {
    let at = i64::try_from(n * 100).unwrap();
    let tx = propose(path, file, seed, n, nodes);
    let verdict = open(path, file)
        .validate(tx, RevisionNumber::new(n - 1), || {
            Timestamp::from_millis(at + 1)
        })
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let result = open(path, file)
        .commit(tx, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap();
    assert!(
        matches!(result, CommitCommandResult::Committed(_)),
        "{result:?}"
    );
}
fn build(path: &Path, file: bool, commits: u64, nodes: u64) -> SeedDocument {
    let seed = seed();
    open(path, file)
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    for n in 1..=commits {
        commit(path, file, &seed, n, nodes);
    }
    seed
}

/// What one one-shot read did, counted on this thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct Work {
    seed_replays: u64,
    envelope_decodes: u64,
    records_decoded: u64,
    documents_parsed: u64,
    checkpoints_decoded: u64,
    objects_loaded: u64,
    blobs_read: u64,
    blobs_hashed: u64,
}
fn reset() {
    let _ = ekr_kernel::read_cost();
    let _ = read_work();
    let _ = ekr_store::objects_loaded();
}
fn taken(runtime: &Runtime) -> Work {
    let cost: ReadCost = ekr_kernel::read_cost();
    let work = read_work();
    Work {
        seed_replays: runtime.seed_replays(),
        envelope_decodes: runtime.seed_envelope_decodes(),
        records_decoded: cost.records_decoded,
        documents_parsed: cost.documents_parsed,
        checkpoints_decoded: cost.checkpoints_decoded,
        objects_loaded: ekr_store::objects_loaded(),
        blobs_read: work.blobs_read,
        blobs_hashed: work.blobs_hashed,
    }
}
/// A reading handle that replays from the seed, as `--full-replay` opens one: the truth every
/// answer below is held to.
fn in_full(path: &Path, file: bool) -> Runtime {
    let mut runtime = reader(path, file);
    runtime.set_full_replay(true);
    runtime
}
/// What `ekr snapshot` and `ekr ontology` print for `at`, computed from a complete verified read:
/// the snapshot document and the ontology's revision and document.
fn complete(runtime: &Runtime, at: Option<u64>) -> (Vec<u8>, Vec<u8>) {
    let read = runtime.read(at.map(RevisionNumber::new)).unwrap();
    let snapshot = serde_json::to_vec(&read.snapshot(None).unwrap()).unwrap();
    let ontology =
        serde_json::to_vec(&(read.root.revision.get(), read.graph.ontology.to_document())).unwrap();
    (snapshot, ontology)
}
/// What one fresh reading handle answers for the snapshot and the ontology at `at` through the
/// read-model reads, whether the model served each, and the work each read did.
struct Answered {
    snapshot: Vec<u8>,
    ontology: Vec<u8>,
    graph_served: bool,
    ontology_served: bool,
    graph_work: Work,
    ontology_work: Work,
}
fn answered(path: &Path, file: bool, at: Option<u64>) -> Answered {
    let at = at.map(RevisionNumber::new);
    reset();
    let runtime = reader(path, file);
    let graph: GraphRead = runtime.read_graph(at).unwrap();
    let snapshot = serde_json::to_vec(&graph.snapshot(None).unwrap()).unwrap();
    let graph_work = taken(&runtime);
    drop(runtime);
    reset();
    let runtime = reader(path, file);
    let ontology: OntologyRead = runtime.read_ontology(at).unwrap();
    let printed = serde_json::to_vec(&(
        ontology.root.revision.get(),
        ontology.ontology.to_document(),
    ))
    .unwrap();
    let ontology_work = taken(&runtime);
    Answered {
        snapshot,
        ontology: printed,
        graph_served: graph.served_from_model(),
        ontology_served: ontology.served_from_model(),
        graph_work,
        ontology_work,
    }
}
/// The profile's measure of one complete read of `at` through a fresh reading handle.
fn complete_work(path: &Path, file: bool, at: Option<u64>) -> Work {
    reset();
    let runtime = reader(path, file);
    let read = runtime.read(at.map(RevisionNumber::new)).unwrap();
    read.snapshot(None).unwrap();
    taken(&runtime)
}
fn provider(file: bool) -> &'static str {
    if file {
        "file"
    } else {
        "sqlite"
    }
}

/// The profile the story asks for first, and the read model's answer to it. A one-shot complete
/// read of the head (`Runtime::read`, what `ekr snapshot` and `ekr ontology` read through) loads,
/// hashes and decodes every retained record the history holds, so its work grows with the
/// history even where the checkpoint is at the head; a read at an earlier revision replays the
/// whole lineage from the seed. A read of the head served from the read model loads two objects,
/// decodes one record and the checkpoint, parses no document and begins no replay, at five
/// commits and at ten alike.
#[test]
fn a_one_shot_read_of_a_verified_head_costs_what_the_history_length_does_not_change() {
    for file in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let seed = build(path, file, 5, 40);
        let complete_five = complete_work(path, file, None);
        let historical_five = complete_work(path, file, Some(3));
        let five = answered(path, file, None);
        for n in 6..=10 {
            commit(path, file, &seed, n, 40);
        }
        let complete_ten = complete_work(path, file, None);
        let historical_ten = complete_work(path, file, Some(3));
        let ten = answered(path, file, None);
        eprintln!(
            "{}: complete head read at 5 commits {complete_five:?}\n  at 10 commits \
             {complete_ten:?}\n  complete read --at 3 at 5 commits {historical_five:?}\n  at 10 \
             commits {historical_ten:?}\n  model head read at 5 commits graph {:?} ontology \
             {:?}\n  at 10 commits graph {:?} ontology {:?}",
            provider(file),
            five.graph_work,
            five.ontology_work,
            ten.graph_work,
            ten.ontology_work
        );
        assert!(
            complete_ten.records_decoded > complete_five.records_decoded
                && complete_ten.objects_loaded > complete_five.objects_loaded,
            "{}: a complete read decodes and loads every retained record",
            provider(file)
        );
        for (answer, commits) in [(&five, 5), (&ten, 10)] {
            assert!(
                answer.graph_served && answer.ontology_served,
                "{}: the head at {commits} commits is served from the read model",
                provider(file)
            );
            // A blob another handle of this process holds verified is compared, not hashed
            // (`ekr_store::verified`), and the seed envelope of every store here is the same bytes:
            // the hashes are bounded, the rest counted exactly.
            assert!(
                answer.graph_work.blobs_hashed <= 2 && answer.ontology_work.blobs_hashed <= 1,
                "{}: {:?} {:?}",
                provider(file),
                answer.graph_work,
                answer.ontology_work
            );
            assert_eq!(
                Work {
                    blobs_hashed: 0,
                    ..answer.graph_work
                },
                Work {
                    seed_replays: 0,
                    envelope_decodes: 0,
                    records_decoded: 1,
                    documents_parsed: 0,
                    checkpoints_decoded: 1,
                    objects_loaded: 2,
                    blobs_read: 2,
                    blobs_hashed: 0,
                },
                "{}: a graph read at {commits} commits loads the head record and the seed \
                 envelope, decodes the record and the checkpoint, and replays nothing",
                provider(file)
            );
            assert_eq!(
                Work {
                    blobs_hashed: 0,
                    ..answer.ontology_work
                },
                Work {
                    seed_replays: 0,
                    envelope_decodes: 0,
                    records_decoded: 1,
                    documents_parsed: 0,
                    checkpoints_decoded: 1,
                    objects_loaded: 1,
                    blobs_read: 1,
                    blobs_hashed: 0,
                },
                "{}: an ontology read at {commits} commits loads and decodes the head record and \
                 the checkpoint only",
                provider(file)
            );
        }
        let truth = in_full(path, file);
        let (snapshot, ontology) = complete(&truth, None);
        assert_eq!(ten.snapshot, snapshot, "{}", provider(file));
        assert_eq!(ten.ontology, ontology, "{}", provider(file));
        // A long-lived handle serves its first read from the model and every later one from the
        // replay state a complete read reaches and keeps.
        let session = reader(path, file);
        let first = session.read_graph(None).unwrap();
        let second = session.read_graph(None).unwrap();
        let third = session.read_graph(None).unwrap();
        assert!(
            first.served_from_model() && !second.served_from_model() && !third.served_from_model(),
            "{}: only a handle's first read is the model's",
            provider(file)
        );
        for read in [&first, &second, &third] {
            assert_eq!(
                serde_json::to_vec(&read.snapshot(None).unwrap()).unwrap(),
                snapshot,
                "{}",
                provider(file)
            );
        }
    }
}

/// A schema change at revision 3 of a store run under profile v2: the store's schema versions.
fn commit_schema(path: &Path, file: bool, n: u64) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            TypeId::mint(),
            format!("Kind{n}"),
        )))],
        evidence: BTreeSet::new(),
        schema_version: Some(SchemaVersionId::mint()),
    };
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: &tx,
    })
    .unwrap()
    .into_bytes();
    let at = i64::try_from(n * 100).unwrap();
    open(path, file)
        .propose(&bytes, context().operator, || Timestamp::from_millis(at))
        .unwrap();
    let verdict = open(path, file)
        .validate(tx.id, RevisionNumber::new(n - 1), || {
            Timestamp::from_millis(at + 1)
        })
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    open(path, file)
        .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap();
}

/// After every commit of a store whose schema changes at revisions 3 and 7, every revision's
/// snapshot and ontology, and the head's, are byte for byte what a complete read from the seed
/// prints, on both providers, whether or not the read model served them. Where the checkpoint is
/// at the head, the head's graph and every revision's schema come from the model.
#[test]
fn every_revision_answers_byte_for_byte_what_a_complete_read_answers() {
    EVOLVING.set(true);
    for file in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let seed = build(path, file, 0, 0);
        let mut served = (0, 0);
        for head in 1..=8 {
            if head == 3 || head == 7 {
                commit_schema(path, file, head);
            } else {
                commit(path, file, &seed, head, 4);
            }
            let truth = in_full(path, file);
            for at in (0..=head).map(Some).chain([None]) {
                let (snapshot, ontology) = complete(&truth, at);
                let answer = answered(path, file, at);
                assert_eq!(
                    answer.snapshot,
                    snapshot,
                    "{}: snapshot at {at:?} with the head at {head}",
                    provider(file)
                );
                assert_eq!(
                    answer.ontology,
                    ontology,
                    "{}: ontology at {at:?} with the head at {head}",
                    provider(file)
                );
                served.0 += u64::from(answer.graph_served);
                served.1 += u64::from(answer.ontology_served);
                if head == 5 {
                    assert!(
                        answer.ontology_served,
                        "{}: with the checkpoint at the head every revision's schema is the \
                         model's ({at:?})",
                        provider(file)
                    );
                    assert_eq!(
                        answer.graph_served,
                        at.is_none_or(|at| at == 5),
                        "{}: the model holds the graph of its own head only ({at:?})",
                        provider(file)
                    );
                }
            }
        }
        assert!(
            served.0 > 0 && served.1 > served.0,
            "{}: the model served graphs and more schemas: {served:?}",
            provider(file)
        );
    }
}

/// The bytes of the checkpoint the newest pointer names, read from the provider itself, with
/// that pointer's coverage and binding.
fn newest_checkpoint(path: &Path, file: bool) -> (serde_json::Value, u64, ContentHash) {
    let pointer = open(path, file)
        .published_events()
        .unwrap()
        .into_iter()
        .rfind(|event| event.name == "ekr.store.CheckpointWritten")
        .unwrap()
        .data;
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let key = format!(
        "ekr.private.checkpoint.{}",
        pointer["checkpoint_hash"].as_str().unwrap()
    );
    let tenant = eventlog_core::TenantId::new(TENANT).unwrap();
    let bytes = executor
        .block_on(async {
            use eventlog_core::EventStore;
            if file {
                let provider = eventlog_file::FileEventStore::open(path).await.unwrap();
                provider.get_blob(&tenant, &key).await.unwrap()
            } else {
                let provider = eventlog_sqlite::SqliteEventStore::open(
                    &path.join("state.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap();
                provider.get_blob(&tenant, &key).await.unwrap()
            }
        })
        .expect("the newest checkpoint is retained");
    (
        serde_json::from_slice(&bytes).unwrap(),
        pointer["covered"].as_u64().unwrap(),
        serde_json::from_value(pointer["binding"].clone()).unwrap(),
    )
}
/// Retains `bytes` as the newest checkpoint through the store's own writer, as a store handle
/// with no kernel authority: the store neither checks nor could check what a checkpoint says.
fn install(path: &Path, file: bool, covered: u64, binding: ContentHash, bytes: &[u8]) {
    use ekr_store::RevisionLog;
    let written = if file {
        ekr_store::FileStore::file_existing(path, TENANT, None)
            .unwrap()
            .write_checkpoint(covered, binding, Some(bytes))
    } else {
        ekr_store::SqliteStore::sqlite_existing(&path.join("state.db"), TENANT, None)
            .unwrap()
            .write_checkpoint(covered, binding, Some(bytes))
    };
    assert_eq!(written, Ok(true), "the forged model is the newest");
}

type Forge = fn(&mut serde_json::Value);
/// Each forgery of a genuine read model at the head, by the part of it a read serves: whether a
/// graph read and an ontology read may still be served from it (they serve only what they
/// verify, and read nothing else), the field it changes and how.
fn forgeries() -> Vec<(bool, bool, &'static str, Forge)> {
    fn root(forged: &mut serde_json::Value) -> &mut serde_json::Value {
        &mut forged["graph"]["graph"]["root"]
    }
    vec![
        (false, true, "a dropped assertion", |forged| {
            let assertions = forged["graph"]["graph"]["assertions"]
                .as_object_mut()
                .unwrap();
            let first = assertions.keys().next().unwrap().clone();
            assertions.remove(&first);
        }),
        (false, true, "a renamed node", |forged| {
            let nodes = forged["graph"]["graph"]["nodes"].as_object_mut().unwrap();
            let node = nodes.values_mut().next().unwrap();
            node["canonical_name"] = "forged".into();
        }),
        (false, false, "another host", |forged| {
            forged["authority"] = serde_json::to_value(ContentHash::of_bytes(b"x")).unwrap();
        }),
        (false, false, "another history", |forged| {
            forged["prefix"] = serde_json::to_value(ContentHash::of_bytes(b"y")).unwrap();
        }),
        (false, false, "an earlier head", |forged| {
            forged["revision"] = 1.into();
        }),
        (false, false, "another format", |forged| {
            forged["format"] = "ekr.replay-checkpoint/0".into();
        }),
        (false, false, "a shorter coverage", |forged| {
            forged["covered"] = (forged["covered"].as_u64().unwrap() - 1).into();
        }),
        (false, false, "no schema versions", |forged| {
            forged["ontologies"] = serde_json::json!([]);
        }),
        (false, false, "a renamed node type in force", |forged| {
            let at = forged["ontologies"].as_array_mut().unwrap();
            at.last_mut().unwrap()["ontology"]["node_types"][0]["name"] = "Forged".into();
        }),
        (
            false,
            false,
            "a schema version from a later revision",
            |forged| {
                let at = forged["ontologies"].as_array_mut().unwrap();
                at.last_mut().unwrap()["from"] = 99.into();
            },
        ),
        (false, true, "another graph root id", |forged| {
            root(forged)["id"] = serde_json::to_value(GraphRootId::mint()).unwrap();
        }),
        (false, true, "a transient graph root", |forged| {
            root(forged)["space"] = serde_json::to_value(Space::Transient).unwrap();
        }),
        (false, true, "another graph root creation time", |forged| {
            root(forged)["created_at"] =
                serde_json::to_value(Timestamp::from_millis(123_456)).unwrap();
        }),
        (false, true, "no seed payloads", |forged| {
            forged["seed_payloads"] = serde_json::json!([]);
        }),
        (
            false,
            true,
            "held ids in a store that keeps none",
            |forged| {
                forged["held"] = serde_json::json!([
                    { "from": 1, "nodes": [NodeId::mint()], "edges": [] }
                ]);
            },
        ),
    ]
}

/// A read model that does not match the history is never served: a forged checkpoint (each field
/// a read would serve, and each a graph read holds to the seed), and a pointer whose binding is
/// not this host's or that claims more than the stream holds. Every read answers byte for byte
/// what a complete read from the seed answers, and a read served from a forged model serves only
/// a part that was not forged and that it verified.
#[test]
fn a_read_model_that_does_not_match_the_history_is_refused_and_never_served() {
    for file in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        build(path, file, 5, 3);
        let truth = complete(&in_full(path, file), None);
        let historical = complete(&in_full(path, file), Some(4));
        let (genuine, covered, binding) = newest_checkpoint(path, file);
        let answer = answered(path, file, None);
        assert!(answer.graph_served && answer.ontology_served);
        assert_eq!((answer.snapshot, answer.ontology), truth);
        for (graph_may, ontology_may, name, forge) in forgeries() {
            let mut forged = genuine.clone();
            forge(&mut forged);
            assert_ne!(forged, genuine, "{name} changes the model");
            install(
                path,
                file,
                covered,
                binding,
                &serde_json::to_vec(&forged).unwrap(),
            );
            for at in [None, Some(4)] {
                let answer = answered(path, file, at);
                let expected = if at.is_none() { &truth } else { &historical };
                assert_eq!(
                    (&answer.snapshot, &answer.ontology),
                    (&expected.0, &expected.1),
                    "{}: a model with {name} answers as a complete read ({at:?})",
                    provider(file)
                );
                assert!(
                    graph_may || !answer.graph_served,
                    "{}: a model with {name} serves no graph ({at:?})",
                    provider(file)
                );
                assert!(
                    ontology_may || !answer.ontology_served,
                    "{}: a model with {name} serves no schema ({at:?})",
                    provider(file)
                );
            }
        }
        let original = serde_json::to_vec(&genuine).unwrap();
        for (claimed, bound, name) in [
            (
                covered,
                ContentHash::of_bytes(b"another binding"),
                "another binding",
            ),
            (covered + 1, binding, "a coverage past the stream"),
            (covered - 1, binding, "a binding of another coverage"),
        ] {
            install(path, file, claimed, bound, &original);
            for at in [None, Some(4)] {
                let answer = answered(path, file, at);
                assert!(
                    !answer.graph_served && !answer.ontology_served,
                    "{}: a pointer with {name} is not taken ({at:?})",
                    provider(file)
                );
            }
            assert_eq!(
                {
                    let answer = answered(path, file, None);
                    (answer.snapshot, answer.ontology)
                },
                truth,
                "{}",
                provider(file)
            );
        }
        // The genuine model again: served again.
        install(path, file, covered, binding, &original);
        assert!(
            answered(path, file, None).graph_served,
            "{}",
            provider(file)
        );
    }
}

/// The model serves the head only when its pointer covers every occurrence: after a proposal
/// nothing has verified, the head is a complete read, while a revision the pointer covers is
/// still served. A commit past the checkpoint's head is a complete read of the graph, and its
/// schema, unchanged since, is the model's. A handle opened for full replay reads no model.
#[test]
fn the_model_serves_only_what_its_pointer_covers_and_never_under_full_replay() {
    for file in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        let seed = build(path, file, 5, 3);
        commit(path, file, &seed, 6, 3);
        let past = answered(path, file, None);
        assert!(
            !past.graph_served && past.ontology_served,
            "{}: a head past the checkpoint's: graph complete, schema from the model",
            provider(file)
        );
        let at_checkpoint = answered(path, file, Some(5));
        assert!(at_checkpoint.graph_served && at_checkpoint.ontology_served);
        propose(path, file, &seed, 7, 3);
        let head = answered(path, file, None);
        assert!(
            !head.graph_served && !head.ontology_served,
            "{}: a head read after an unverified proposal is complete",
            provider(file)
        );
        let covered = answered(path, file, Some(5));
        assert!(covered.graph_served && covered.ontology_served);
        let truth = in_full(path, file);
        for (at, answer) in [(None, &head), (Some(5), &covered)] {
            assert_eq!(
                (answer.snapshot.clone(), answer.ontology.clone()),
                complete(&truth, at),
                "{}: {at:?}",
                provider(file)
            );
        }
        let graph = in_full(path, file)
            .read_graph(Some(RevisionNumber::new(5)))
            .unwrap();
        let ontology = in_full(path, file).read_ontology(None).unwrap();
        assert!(
            !graph.served_from_model() && !ontology.served_from_model(),
            "{}: a full-replay handle reads no model",
            provider(file)
        );
    }
}
