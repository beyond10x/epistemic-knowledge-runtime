//! Adversary pass on `story:reads-share-verified-state` (wave perf-01, unit R).
//!
//! A `VerifiedRead` now shares the kernel's graph, transaction records and seed input, and the
//! head's alias index is built once and kept beside the head graph. These cases attack that
//! sharing: a capture taken before later decisions, a capture its owner changes, a past revision
//! read before and after later commits, a head restored from a checkpoint by a fresh handle, and
//! a reader handle racing a committing one.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{Cardinality, NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

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
fn open(path: &std::path::Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, "test", context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), anchor())
    }
    .unwrap()
}
fn fixture() -> SeedDocument {
    let mut seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let type_id = "00000000-0000-4000-8000-000000000005".parse().unwrap();
    let many = "00000000-0000-4000-8000-000000000006".parse().unwrap();
    let mut declared = NodeType::new(type_id, "Subject");
    let mut definition = PropertyDefinition::new(many, "labels", ValueType::String);
    definition.cardinality = Cardinality::Many;
    declared.properties.insert(many, definition);
    seed.ontology.node_types.push(declared);
    let mut node = Node::<Value>::new(NodeId::mint(), seed.graph.root.id, type_id, "seed");
    node.aliases = vec!["seeded".into()];
    let bytes = b"synthetic human evidence".to_vec();
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
    seed.graph.nodes.insert(node.id, node);
    seed.graph.evidence.insert(evidence.id, evidence);
    seed.evidence_payloads.insert(hash, bytes.into());
    seed
}
fn subject(seed: &SeedDocument) -> TypeId {
    seed.ontology.node_types[0].id
}
/// Creates one node of the fixture's type carrying `aliases`.
fn creating(seed: &SeedDocument, aliases: &[&str]) -> (GraphTransaction, NodeId) {
    let id = NodeId::mint();
    let ty = &seed.ontology.node_types[0];
    let many = *ty.properties.keys().next().unwrap();
    let evidence = *seed.graph.evidence.keys().next().unwrap();
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id: seed.graph.root.id,
        subject: Subject::Node(id),
        predicate: Predicate::Property(many),
        object: Object::Value(Value::String("changed".into())),
        evidence: BTreeSet::from([evidence]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    (
        GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            operations: vec![
                GraphOperation::AddAssertion(Box::new(assertion)),
                GraphOperation::CreateNode(NodeDraft {
                    id,
                    root_id: seed.graph.root.id,
                    type_id: ty.id,
                    canonical_name: "created".into(),
                    properties: BTreeMap::from([(many, vec![Value::String("label".into())])]),
                    aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
                }),
            ],
            evidence: BTreeSet::from([evidence]),
            schema_version: None,
        },
        id,
    )
}
fn inadmissible(seed: &SeedDocument) -> GraphTransaction {
    let (mut tx, _) = creating(seed, &[]);
    if let GraphOperation::CreateNode(node) = &mut tx.operations[1] {
        node.properties.values_mut().next().unwrap()[0] = Value::Float(f64::NAN);
    }
    tx
}
fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}
fn at(millis: i64) -> impl FnOnce() -> Timestamp {
    move || Timestamp::from_millis(millis)
}
/// A clock that only moves forward, shared by every command of the binary.
fn tick() -> impl FnOnce() -> Timestamp {
    static NOW: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(100);
    at(NOW.fetch_add(10, std::sync::atomic::Ordering::SeqCst))
}
fn propose(kernel: &Runtime, tx: &GraphTransaction) {
    kernel
        .propose(&encode(tx), context().operator, tick())
        .unwrap();
}
fn validate(kernel: &Runtime, tx: &GraphTransaction, against: RevisionNumber) {
    match kernel.validate(tx.id, against, tick()).unwrap() {
        ValidationCommandResult::Validated(_) => {}
        ValidationCommandResult::Rejected(record) => panic!("rejected: {:?}", record.issues),
    }
}
fn commit(kernel: &Runtime, tx: &GraphTransaction) -> CommitCommandResult {
    kernel.commit(tx.id, context().operator, tick()).unwrap()
}
/// Everything a capture answers, deep-copied out of it.
#[derive(Debug, PartialEq)]
struct Held {
    graph: CanonicalGraph,
    root: Root,
    transactions: BTreeMap<TransactionId, TransactionRecord>,
    seed_input: SeedDocument,
    revisions: Vec<RevisionNumber>,
    aliases: AliasIndex,
}
fn held(read: &VerifiedRead) -> Held {
    Held {
        graph: CanonicalGraph::clone(&read.graph),
        root: read.root,
        transactions: BTreeMap::clone(&read.transactions),
        seed_input: SeedDocument::clone(&read.seed_input),
        revisions: read.revisions.keys().copied().collect(),
        aliases: read.aliases().into_owned(),
    }
}

/// A capture and a `transactions` listing taken before a validation, a rejection, a commit and a
/// stale decision still answer exactly what they answered when taken; the alias index a capture
/// was handed stays its graph's index; a new read sees every later decision and the new alias.
#[test]
fn a_capture_answers_what_it_answered_when_taken_through_later_decisions() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.clone(), at(10)).unwrap();
        let (one, created) = creating(&seed, &["alpha"]);
        let (two, _) = creating(&seed, &["beta"]);
        let refused = inadmissible(&seed);
        for tx in [&one, &two, &refused] {
            propose(&kernel, tx);
        }
        let before = kernel.read(None).unwrap();
        let listed = kernel.transactions().unwrap();
        let taken = held(&before);
        let listed_taken = BTreeMap::clone(&listed);

        validate(&kernel, &one, RevisionNumber::SEED);
        validate(&kernel, &two, RevisionNumber::SEED);
        assert!(matches!(
            kernel
                .validate(refused.id, RevisionNumber::SEED, tick())
                .unwrap(),
            ValidationCommandResult::Rejected(_)
        ));
        assert!(matches!(
            commit(&kernel, &one),
            CommitCommandResult::Committed(_)
        ));
        assert!(matches!(
            commit(&kernel, &two),
            CommitCommandResult::Stale(_)
        ));

        assert_eq!(held(&before), taken, "file={file}: the capture changed");
        assert_eq!(*listed, listed_taken, "file={file}: the listing changed");
        assert!(before.aliases().nodes(subject(&seed), "alpha").is_empty());

        let after = kernel.read(None).unwrap();
        assert_eq!(after.aliases().nodes(subject(&seed), "alpha"), [created]);
        assert_eq!(*after.aliases(), AliasIndex::of(&after.graph));
        let states: Vec<TransactionState> = [one.id, two.id, refused.id]
            .map(|id| after.transactions[&id].state())
            .into();
        assert_eq!(
            states,
            [
                TransactionState::Committed,
                TransactionState::Stale,
                TransactionState::Rejected
            ],
            "file={file}"
        );
    }
}

/// Changing a capture through `Arc::make_mut` changes that capture alone: the next read of the
/// same handle, and its alias index, answer what they would have answered untouched.
#[test]
fn a_capture_changed_by_its_owner_leaves_the_kernel_state_untouched() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.clone(), at(10)).unwrap();
        let (one, created) = creating(&seed, &["alpha"]);
        propose(&kernel, &one);
        validate(&kernel, &one, RevisionNumber::SEED);
        commit(&kernel, &one);
        let untouched = held(&kernel.read(None).unwrap());

        let mut changed = kernel.read(None).unwrap();
        let _ = changed.aliases();
        let graph = Arc::make_mut(&mut changed.graph);
        graph.nodes.get_mut(&created).unwrap().aliases = vec!["omega".into()];
        Arc::make_mut(&mut changed.transactions).clear();
        Arc::make_mut(&mut changed.seed_input)
            .evidence_payloads
            .clear();
        assert_eq!(changed.aliases().nodes(subject(&seed), "omega"), [created]);
        assert!(changed.aliases().nodes(subject(&seed), "alpha").is_empty());

        let again = kernel.read(None).unwrap();
        assert_eq!(held(&again), untouched, "file={file}");
        assert_eq!(*kernel.transactions().unwrap(), untouched.transactions);
        assert_eq!(
            *kernel.schema_history(again.root.revision).unwrap().graph,
            untouched.graph
        );
    }
}

/// Views rule 5 at the kernel: a read of a past revision answers the same graph, records, seed
/// input and alias index before and after later commits, and a node created later is not in its
/// alias index; a read at the seed on a handle that has read the head answers the seed's aliases.
#[test]
fn a_past_revision_reads_the_same_before_and_after_later_commits() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.clone(), at(10)).unwrap();
        let (one, first) = creating(&seed, &["alpha"]);
        propose(&kernel, &one);
        validate(&kernel, &one, RevisionNumber::SEED);
        commit(&kernel, &one);
        let _ = kernel.read(None).unwrap().aliases();
        let at_one = held(&kernel.read(Some(RevisionNumber::new(1))).unwrap());
        let at_seed = held(&kernel.read(Some(RevisionNumber::SEED)).unwrap());

        let (two, second) = creating(&seed, &["beta"]);
        propose(&kernel, &two);
        validate(&kernel, &two, RevisionNumber::new(1));
        commit(&kernel, &two);
        let _ = kernel.read(None).unwrap().aliases();

        for handle in [&kernel, &open(directory.path(), file)] {
            let later_one = held(&handle.read(Some(RevisionNumber::new(1))).unwrap());
            let later_seed = held(&handle.read(Some(RevisionNumber::SEED)).unwrap());
            assert_eq!(later_one, at_one, "file={file}: revision 1 changed");
            assert_eq!(later_seed, at_seed, "file={file}: the seed changed");
        }
        assert_eq!(at_one.aliases.nodes(subject(&seed), "alpha"), [first]);
        assert!(at_seed.aliases.nodes(subject(&seed), "alpha").is_empty());
        let head = kernel.read(None).unwrap();
        assert_eq!(head.aliases().nodes(subject(&seed), "alpha"), [first]);
        assert_eq!(head.aliases().nodes(subject(&seed), "beta"), [second]);
    }
}

/// A fresh handle continues from the store's replay checkpoint, which holds only the head's
/// graph. Its alias index is its head graph's, and a later commit through that handle gets an
/// index of its own head.
#[test]
fn a_head_restored_from_a_checkpoint_indexes_its_own_graph() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let writer = open(directory.path(), file);
        writer.seed(seed.clone(), at(10)).unwrap();
        let (one, first) = creating(&seed, &["alpha"]);
        propose(&writer, &one);
        validate(&writer, &one, RevisionNumber::SEED);
        commit(&writer, &one);

        let fresh = open(directory.path(), file);
        let read = fresh.read(None).unwrap();
        assert_eq!(
            fresh.seed_replays(),
            0,
            "file={file}: the head was not restored from the checkpoint"
        );
        assert_eq!(*read.aliases(), AliasIndex::of(&read.graph), "file={file}");
        assert_eq!(read.aliases().nodes(subject(&seed), "alpha"), [first]);

        let (two, second) = creating(&seed, &["gamma"]);
        propose(&fresh, &two);
        validate(&fresh, &two, RevisionNumber::new(1));
        commit(&fresh, &two);
        let later = fresh.read(None).unwrap();
        assert_eq!(later.aliases().nodes(subject(&seed), "gamma"), [second]);
        assert_eq!(*later.aliases(), AliasIndex::of(&later.graph));
        assert!(read.aliases().nodes(subject(&seed), "gamma").is_empty());
    }
}

/// Aliases are matched byte for byte: case, Unicode normalisation and whitespace variants of a
/// held alias are other aliases.
#[test]
fn the_alias_index_matches_aliases_byte_for_byte() {
    let seed = fixture();
    let directory = tempfile::tempdir().unwrap();
    let kernel = open(directory.path(), false);
    kernel.seed(seed.clone(), at(10)).unwrap();
    let (one, created) = creating(&seed, &["Stra\u{df}e", "e\u{301}", "K"]);
    propose(&kernel, &one);
    validate(&kernel, &one, RevisionNumber::SEED);
    commit(&kernel, &one);
    let read = kernel.read(None).unwrap();
    let index = read.aliases();
    for held in ["Stra\u{df}e", "e\u{301}", "K"] {
        assert_eq!(index.nodes(subject(&seed), held), [created], "{held:?}");
    }
    for other in ["STRASSE", "Strasse", "\u{e9}", "\u{212a}", "k", " K", "K "] {
        assert!(index.nodes(subject(&seed), other).is_empty(), "{other:?}");
    }
}

/// A reader handle reading while a second handle on the same store commits never sees a mixed
/// state: every capture's graph is its root's revision, its alias index is its graph's, and every
/// committed record it holds names a revision it holds.
#[test]
fn a_reader_racing_a_committing_handle_sees_one_state_per_capture() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let writer = open(directory.path(), file);
        writer.seed(seed.clone(), at(10)).unwrap();
        let path = directory.path().to_path_buf();
        let done = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let reader = {
            let done = Arc::clone(&done);
            std::thread::spawn(move || {
                let reader = open(&path, file);
                let mut seen = 0u64;
                while !done.load(std::sync::atomic::Ordering::SeqCst) || seen == 0 {
                    let read = reader.read(None).unwrap();
                    assert_eq!(read.graph.revision, read.root.revision);
                    assert_eq!(*read.aliases(), AliasIndex::of(&read.graph));
                    for record in read.transactions.values() {
                        if let Some(receipt) = &record.committed {
                            assert!(read.revisions.contains_key(&receipt.result.revision));
                            assert!(receipt.result.revision <= read.root.revision);
                        }
                    }
                    seen += 1;
                }
                seen
            })
        };
        for n in 0..6u64 {
            let (tx, _) = creating(&seed, &[&format!("n{n}")]);
            propose(&writer, &tx);
            validate(&writer, &tx, RevisionNumber::new(n));
            assert!(matches!(
                commit(&writer, &tx),
                CommitCommandResult::Committed(_)
            ));
        }
        done.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(reader.join().unwrap() > 0, "file={file}");
    }
}

/// Withdraws the retained object at `hash` as the store domain requires any withdrawal to
/// (`systems/ekr/domains/store.yaml`, held bytes): its blob is deleted and an event is appended to
/// its object stream. Nothing in this runtime withdraws bytes yet, so no event name is declared
/// for it; this one stands in for it.
fn withdraw_object(path: &std::path::Path, file: bool, hash: ContentHash) {
    async fn withdraw<P: eventlog_core::EventStore>(provider: P, hash: ContentHash) {
        let tenant = eventlog_core::TenantId::new("test").unwrap();
        provider.delete_blob(&tenant, &hash.to_hex()).await.unwrap();
        let stream =
            eventlog_core::StreamId::new(tenant, "ekr.store.object", hash.to_hex()).unwrap();
        let event = eventlog_core::NewEvent::new(
            "adversary.perf01r.WithdrawalRecorded",
            1,
            serde_json::json!({"reason": "adversary deletion request"}),
        )
        .unwrap();
        let meta = eventlog_core::CommandMeta {
            idempotency_key: format!("adversary-perf01r-withdrawal-{hash}"),
            request_hash: "adversary-perf01r-withdrawal".into(),
            subject: "adversary".into(),
            actor: "adversary".into(),
            request_id: "adversary-perf01r-withdrawal".into(),
            trace_id: "adversary-perf01r-withdrawal".into(),
            causation_id: None,
            causation_depth: 0,
            occurred_at: ::time::OffsetDateTime::UNIX_EPOCH,
            claim: None,
        };
        provider
            .append(&stream, eventlog_core::Expected::Any, &[event], &meta)
            .await
            .unwrap();
    }
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    executor.block_on(async {
        if file {
            withdraw(
                eventlog_file::FileEventStore::open(path).await.unwrap(),
                hash,
            )
            .await;
        } else {
            let provider = eventlog_sqlite::SqliteEventStore::open(
                &path.join("state.db").to_string_lossy(),
                "ekr",
            )
            .await
            .unwrap();
            withdraw(provider, hash).await;
        }
    });
}

/// The seed input a handle rebuilt once is not handed out after the seed's named evidence
/// payload was withdrawn from the store: the handle that read before answers the withdrawal as a
/// fresh handle does. A blob deleted through the provider alone is not seen by the handle that
/// holds its bytes (`task:held-bytes-notice-deleted-blobs`); the store domain decides that every
/// withdrawal also appends an event to the object's stream, and so this one does.
#[test]
fn a_seed_payload_withdrawn_after_a_read_is_answered_as_a_fresh_handle_answers_it() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.clone(), at(10)).unwrap();
        let _ = kernel.read(None).unwrap();
        let payload = *seed.evidence_payloads.keys().next().unwrap();
        withdraw_object(directory.path(), file, payload);
        let fresh = open(directory.path(), file)
            .read(None)
            .map(|read| read.root)
            .map_err(|error| error.to_string());
        let again = kernel
            .read(None)
            .map(|read| read.root)
            .map_err(|error| error.to_string());
        assert!(fresh.is_err(), "file={file}: control: {fresh:?}");
        assert_eq!(again, fresh, "file={file}");
    }
}
