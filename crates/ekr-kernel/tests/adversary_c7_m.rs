//! Adversarial pass on `task:migrate-reads-a-current-store` (wave correct-07, unit M): the
//! preserving migration of a store whose history added evidence after its seed, driven through
//! [`Runtime`] on both providers with histories the unit's own test does not build.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use ekr_store::{FileStore, ObjectStore, SqliteStore, StorageClass};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

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
        Runtime::file(path, "test", context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), anchor())
    }
    .unwrap()
}
fn at(millis: i64) -> impl FnOnce() -> Timestamp {
    move || Timestamp::from_millis(millis)
}

const SEEDED_PAYLOAD: &[u8] = b"seeded human statement";

struct Seeded {
    document: SeedDocument,
    node: NodeId,
    property: PropertyId,
}

fn seeded() -> Seeded {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let type_id: TypeId = "00000000-0000-4000-8000-000000000005".parse().unwrap();
    let property: PropertyId = "00000000-0000-4000-8000-000000000006".parse().unwrap();
    let mut declared = NodeType::new(type_id, "Subject");
    declared.properties.insert(
        property,
        PropertyDefinition::new(property, "label", ValueType::String),
    );
    document.ontology.node_types.push(declared);
    let node = Node::<Value>::new(NodeId::mint(), document.graph.root.id, type_id, "subject");
    let seeded_evidence = human_evidence(EvidenceId::mint(), SEEDED_PAYLOAD);
    document.graph.nodes.insert(node.id, node.clone());
    document
        .graph
        .evidence
        .insert(seeded_evidence.id, seeded_evidence.clone());
    document
        .evidence_payloads
        .insert(seeded_evidence.content_hash, SEEDED_PAYLOAD.to_vec().into());
    Seeded {
        document,
        node: node.id,
        property,
    }
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

fn add_evidence(evidence: Evidence, payload: &[u8]) -> GraphOperation {
    GraphOperation::AddEvidence(Box::new(EvidenceAddition {
        evidence,
        payload: payload.to_vec(),
    }))
}

fn assertion(seed: &Seeded, evidence: EvidenceId, label: &str) -> GraphOperation {
    GraphOperation::AddAssertion(Box::new(Assertion {
        id: AssertionId::mint(),
        root_id: seed.document.graph.root.id,
        subject: Subject::Node(seed.node),
        predicate: Predicate::Property(seed.property),
        object: Object::Value(Value::String(label.into())),
        evidence: BTreeSet::from([evidence]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }))
}

fn transaction(operations: Vec<GraphOperation>) -> GraphTransaction {
    let evidence = operations
        .iter()
        .filter_map(|operation| match operation {
            GraphOperation::AddAssertion(assertion) => Some(assertion.evidence.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence,
        schema_version: None,
    }
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

fn head(runtime: &Runtime) -> RevisionNumber {
    runtime.head().unwrap().unwrap().revision
}

fn propose_and_validate(
    runtime: &Runtime,
    tx: &GraphTransaction,
    against: RevisionNumber,
    now: i64,
) -> ValidationCommandResult {
    runtime
        .propose(&encode(tx), context().operator, at(now))
        .unwrap();
    runtime.validate(tx.id, against, at(now + 1)).unwrap()
}

fn commit(runtime: &Runtime, tx: &GraphTransaction, now: i64) -> CommitCommandResult {
    runtime.commit(tx.id, context().operator, at(now)).unwrap()
}

fn committed(runtime: &Runtime, tx: &GraphTransaction, now: i64) {
    let against = head(runtime);
    assert!(
        matches!(
            propose_and_validate(runtime, tx, against, now),
            ValidationCommandResult::Validated(_)
        ),
        "{tx:?}"
    );
    assert!(matches!(
        commit(runtime, tx, now + 2),
        CommitCommandResult::Committed(_)
    ));
}

/// Every `ekr.store.object` event the store logged for `hash`, by name and body: the object's
/// first class and `stored_at`, and each retention raise, in order.
fn object_history(runtime: &Runtime, hash: ContentHash) -> Vec<(String, serde_json::Value)> {
    runtime
        .published_events()
        .unwrap()
        .into_iter()
        .filter(|event| event.stream_type == "ekr.store.object" && event.stream_id == hash.to_hex())
        .map(|event| (event.name, event.data))
        .collect()
}

/// The knowledge, evidence, ontology and agent roots of every revision.
fn roots(runtime: &Runtime) -> BTreeMap<RevisionNumber, Root> {
    runtime
        .read(None)
        .unwrap()
        .revisions
        .into_iter()
        .map(|(number, revision)| (number, revision.root))
        .collect()
}

/// Brief items 1, 2, 4 and 5 in one history, on both providers: a payload equal to a seeded one;
/// an `!AddEvidence` with no citing assertion; the same payload added again by a second
/// transaction under another id; a rejected and a stale transaction each holding an
/// `!AddEvidence`; a validation against an earlier revision; and a retained checkpoint. The
/// migration must succeed; every payload's object history (first class, `stored_at`, raises) must
/// be the source's, logged once; no payload may be in `carried_objects`; and every revision's
/// roots must be the source's.
#[test]
fn a_history_of_every_added_evidence_shape_migrates_with_payloads_unchanged() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source");
        std::fs::create_dir_all(&source_path).unwrap();
        let seed = seeded();
        let source = open(&source_path, file);
        source.seed(seed.document.clone(), at(10)).unwrap();

        // Revision 1: the seeded payload added again under a new id, cited.
        let again = human_evidence(EvidenceId::mint(), SEEDED_PAYLOAD);
        committed(
            &source,
            &transaction(vec![
                add_evidence(again.clone(), SEEDED_PAYLOAD),
                assertion(&seed, again.id, "from the seeded bytes"),
            ]),
            20,
        );
        // Revision 2: evidence with no citing assertion.
        let shared: &[u8] = b"a message added twice";
        let lone = human_evidence(EvidenceId::mint(), shared);
        committed(
            &source,
            &transaction(vec![add_evidence(lone.clone(), shared)]),
            30,
        );
        // A rejected transaction holding an `!AddEvidence` whose payload is never stored.
        let rejected_payload: &[u8] = b"rejected bytes";
        let rejected = human_evidence(EvidenceId::mint(), b"what the rejected entry names");
        let refused = propose_and_validate(
            &source,
            &transaction(vec![
                add_evidence(rejected.clone(), rejected_payload),
                assertion(&seed, rejected.id, "rejected"),
            ]),
            head(&source),
            40,
        );
        assert!(matches!(refused, ValidationCommandResult::Rejected(_)));
        // A transaction validated against revision 2 and then made stale.
        let stale_payload: &[u8] = b"stale bytes";
        let stale_evidence = human_evidence(EvidenceId::mint(), stale_payload);
        let stale = transaction(vec![
            add_evidence(stale_evidence.clone(), stale_payload),
            assertion(&seed, stale_evidence.id, "stale"),
        ]);
        assert!(matches!(
            propose_and_validate(&source, &stale, head(&source), 50),
            ValidationCommandResult::Validated(_)
        ));
        // Revision 3: the shared payload added a second time, under another id, cited.
        let twice = human_evidence(EvidenceId::mint(), shared);
        committed(
            &source,
            &transaction(vec![
                add_evidence(twice.clone(), shared),
                assertion(&seed, twice.id, "from the shared bytes"),
            ]),
            60,
        );
        assert!(matches!(
            commit(&source, &stale, 70),
            CommitCommandResult::Stale(_)
        ));
        // A validation against revision 1 while the head is 3.
        let late_payload: &[u8] = b"validated against an earlier revision";
        let late_evidence = human_evidence(EvidenceId::mint(), late_payload);
        let _ = propose_and_validate(
            &source,
            &transaction(vec![add_evidence(late_evidence.clone(), late_payload)]),
            RevisionNumber::new(1),
            80,
        );
        // Revision 4, after a retained checkpoint.
        source.retain_checkpoint_at_rest();
        let last: &[u8] = b"the last message";
        let last_evidence = human_evidence(EvidenceId::mint(), last);
        committed(
            &source,
            &transaction(vec![
                add_evidence(last_evidence.clone(), last),
                assertion(&seed, last_evidence.id, "last"),
            ]),
            90,
        );
        source.retain_checkpoint_at_rest();
        assert_eq!(head(&source), RevisionNumber::new(4));

        let destination_path = directory.path().join("destination");
        std::fs::create_dir_all(&destination_path).unwrap();
        let destination = open(&destination_path, file);
        let report = source
            .migrate_into(&destination)
            .unwrap_or_else(|error| panic!("file={file}: {error}"));

        let payloads: Vec<ContentHash> = [SEEDED_PAYLOAD, shared, last]
            .iter()
            .map(|bytes| ContentHash::of_bytes(bytes))
            .collect();
        for hash in &payloads {
            assert_eq!(
                destination.content(hash).unwrap(),
                source.content(hash).unwrap(),
                "file={file}: {hash}"
            );
            assert_eq!(
                object_history(&destination, *hash),
                object_history(&source, *hash),
                "file={file}: the class history of {hash}"
            );
            assert!(
                !report.carried_objects.contains(hash),
                "file={file}: {hash} is published with its commit, not carried"
            );
        }
        for absent in [rejected_payload, stale_payload, late_payload] {
            let hash = ContentHash::of_bytes(absent);
            assert_eq!(source.content(&hash).unwrap(), None, "file={file}");
            assert_eq!(destination.content(&hash).unwrap(), None, "file={file}");
        }
        assert_eq!(roots(&destination), roots(&source), "file={file}");
        let before = source.read(None).unwrap();
        let after = destination.read(None).unwrap();
        assert_eq!(after.graph.evidence, before.graph.evidence, "file={file}");
        assert_eq!(
            after.graph.assertions, before.graph.assertions,
            "file={file}"
        );
        drop(destination);
        let mut reopened = open(&destination_path, file);
        reopened.set_full_replay(true);
        assert_eq!(
            reopened.head().unwrap(),
            source.head().unwrap(),
            "file={file}"
        );
    }
}

/// Puts `bytes` straight into the store at `path` in `class`, through the public `ObjectStore`.
fn stage(path: &Path, file: bool, class: StorageClass, bytes: &[u8], stored_at: i64) {
    if file {
        let store = FileStore::file(path, "test", None).unwrap();
        store
            .put(class, bytes, Timestamp::from_millis(stored_at))
            .unwrap();
    } else {
        let store = SqliteStore::sqlite(&path.join("state.db"), "test", None).unwrap();
        store
            .put(class, bytes, Timestamp::from_millis(stored_at))
            .unwrap();
    }
}

/// A payload a host stored below Provenance (`Incubating`, the class design § 37 gives material
/// with integration potential) before a commit made it evidence. The source's commit raises it to
/// Provenance and replays. The migration publishes it with its commit in the class it was
/// *first* stored with (`migrate.rs:274`, `held.stored_as`), below what replay of that commit
/// reads it at (`replay.rs:1134`, `StorageClass::Provenance`), so the next occurrence's
/// reconstruction of the destination refuses.
#[test]
fn a_payload_stored_below_provenance_before_its_commit_migrates() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source");
        std::fs::create_dir_all(&source_path).unwrap();
        let seed = seeded();
        let staged: &[u8] = b"a document a host kept for integration";
        let evidence = human_evidence(EvidenceId::mint(), staged);
        {
            let source = open(&source_path, file);
            source.seed(seed.document.clone(), at(10)).unwrap();
        }
        stage(&source_path, file, StorageClass::Incubating, staged, 15);
        let source = open(&source_path, file);
        committed(
            &source,
            &transaction(vec![
                add_evidence(evidence.clone(), staged),
                assertion(&seed, evidence.id, "from the staged document"),
            ]),
            20,
        );
        // One later occurrence, so the commit is replayed inside the migration's loop.
        let later = human_evidence(EvidenceId::mint(), b"later");
        source
            .propose(
                &encode(&transaction(vec![add_evidence(later, b"later")])),
                context().operator,
                at(30),
            )
            .unwrap();
        // The source itself is sound: it replays in full from its seed.
        let source_head = source.head().unwrap();
        drop(source);
        let mut reopened = open(&source_path, file);
        reopened.set_full_replay(true);
        assert_eq!(reopened.head().unwrap(), source_head, "file={file}");
        let hash = ContentHash::of_bytes(staged);
        let history = object_history(&reopened, hash);
        assert_eq!(
            history.len(),
            2,
            "file={file}: stored, then raised: {history:?}"
        );

        let destination_path = directory.path().join("destination");
        std::fs::create_dir_all(&destination_path).unwrap();
        let destination = open(&destination_path, file);
        let migrated = reopened.migrate_into(&destination);
        assert!(migrated.is_ok(), "file={file}: {}", migrated.unwrap_err());
        assert_eq!(
            object_history(&destination, hash),
            history,
            "file={file}: the payload's class history"
        );
    }
}

/// An `!AddEvidence` whose payload is the byte string of the migration's started marker
/// (`migrate.rs:92-95`). It validates; its commit is refused as `migrate-incomplete` before
/// anything is published, because the history the commit checks would hold the started marker
/// without the finished one (`migrate::finished`). The store stays readable, but the transaction
/// can never commit and its refusal names a migration nobody ran. Not introduced by this unit: the
/// markers are fixed bytes at fixed addresses, and a payload is any bytes.
#[test]
#[ignore = "task:migration-marker-cannot-be-evidence: a payload equal to the migration marker's bytes is refused at commit"]
fn evidence_whose_payload_is_the_migration_marker_commits() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let marker: &[u8] = br#"{"format":"ekr.migration-started/1"}"#;
        let evidence = human_evidence(EvidenceId::mint(), marker);
        let tx = transaction(vec![add_evidence(evidence, marker)]);
        let against = head(&runtime);
        assert!(matches!(
            propose_and_validate(&runtime, &tx, against, 20),
            ValidationCommandResult::Validated(_)
        ));
        let result = runtime.commit(tx.id, context().operator, at(22));
        drop(runtime);
        let mut reopened = open(directory.path(), file);
        reopened.set_full_replay(true);
        let committed = reopened
            .published_events()
            .unwrap()
            .iter()
            .filter(|event| event.name == "ekr.kernel.RevisionCommitted")
            .count();
        let head = reopened.head().map(|root| root.map(|root| root.revision));
        assert!(
            matches!(result, Ok(CommitCommandResult::Committed(_))) && head.is_ok(),
            "file={file}: commit {result:?}; commits logged {committed}; reopened head {head:?}"
        );
    }
}

/// A payload whose retention a host raised to Canonical after the commit that added it. The
/// migration publishes the commit with the payload at its first class, then the after-loop carry
/// raises it: its class history in the destination must be the source's, stored once and raised
/// once.
#[test]
fn a_payload_raised_after_its_commit_keeps_its_class_history() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source");
        std::fs::create_dir_all(&source_path).unwrap();
        let seed = seeded();
        let payload: &[u8] = b"a message later kept for good";
        let evidence = human_evidence(EvidenceId::mint(), payload);
        {
            let source = open(&source_path, file);
            source.seed(seed.document.clone(), at(10)).unwrap();
            committed(
                &source,
                &transaction(vec![
                    add_evidence(evidence.clone(), payload),
                    assertion(&seed, evidence.id, "kept"),
                ]),
                20,
            );
        }
        stage(&source_path, file, StorageClass::Canonical, payload, 99);
        let source = open(&source_path, file);
        source
            .propose(
                &encode(&transaction(vec![assertion(&seed, evidence.id, "later")])),
                context().operator,
                at(30),
            )
            .unwrap();
        let hash = ContentHash::of_bytes(payload);
        let history = object_history(&source, hash);
        assert_eq!(history.len(), 2, "file={file}: {history:?}");

        let destination_path = directory.path().join("destination");
        std::fs::create_dir_all(&destination_path).unwrap();
        let destination = open(&destination_path, file);
        let report = source
            .migrate_into(&destination)
            .unwrap_or_else(|error| panic!("file={file}: {error}"));
        assert!(!report.carried_objects.contains(&hash), "file={file}");
        assert_eq!(object_history(&destination, hash), history, "file={file}");
    }
}

#[allow(dead_code)]
#[path = "recovery/support.rs"]
mod support;

/// A store whose `!AddEvidence` commit was elected, its outcome lost, and resumed by the same
/// command: its preparation is resolved, so the store migrates, and the payload's class history
/// in the destination is the source's.
#[test]
fn a_resumed_commit_preparation_that_added_evidence_migrates() {
    for fault in [support::Fault::BeforeWrite, support::Fault::AfterWrite] {
        for file in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let source_path = directory.path().join("source");
            std::fs::create_dir_all(&source_path).unwrap();
            let seed = support::seed_fixture();
            let payload: &[u8] = b"added under a lost response";
            let evidence = human_evidence(EvidenceId::mint(), payload);
            let tx = transaction(vec![add_evidence(evidence, payload)]);
            let plan = support::Plan {
                kind: support::Kind::Committed,
                file,
                seed_yaml: serde_yaml_ng::to_string(&seed).unwrap(),
                tx: tx.id,
                document: encode(&tx),
            };
            {
                let source = support::open(&source_path, file);
                source.seed(seed, at(10)).unwrap();
                source
                    .propose(&plan.document, context().operator, at(20))
                    .unwrap();
                source
                    .validate(tx.id, RevisionNumber::SEED, at(30))
                    .unwrap();
            }
            let outcome = support::probed(&source_path, file, &support::Hooks::new(fault), |k| {
                support::run(k, &plan, support::at(40))
            });
            assert_eq!(outcome, support::Outcome::Unknown, "file={file}");
            let source = support::open(&source_path, file);
            assert!(
                matches!(
                    support::run(&source, &plan, support::no_clock("resumed")),
                    support::Outcome::Record(_)
                ),
                "file={file}"
            );
            let hash = ContentHash::of_bytes(payload);
            let destination_path = directory.path().join("destination");
            std::fs::create_dir_all(&destination_path).unwrap();
            let destination = open(&destination_path, file);
            source
                .migrate_into(&destination)
                .unwrap_or_else(|error| panic!("file={file}: {error}"));
            assert_eq!(
                destination.content(&hash).unwrap().as_deref(),
                Some(payload),
                "file={file}"
            );
            assert_eq!(
                object_history(&destination, hash),
                object_history(&source, hash),
                "file={file}"
            );
            assert_eq!(roots(&destination), roots(&source), "file={file}");
        }
    }
}
