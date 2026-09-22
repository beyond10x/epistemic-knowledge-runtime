//! Actual retained command results and independent-process reconstruction on both providers.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{Cardinality, NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

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
    let list = "00000000-0000-4000-8000-000000000007".parse().unwrap();
    let mut declared = NodeType::new(type_id, "Subject");
    let mut definition = PropertyDefinition::new(many, "labels", ValueType::String);
    definition.cardinality = Cardinality::Many;
    declared.properties.insert(many, definition);
    declared.properties.insert(
        list,
        PropertyDefinition::new(list, "list", ValueType::List(Box::new(ValueType::String))),
    );
    seed.ontology.node_types.push(declared);
    let node = Node::<Value>::new(NodeId::mint(), seed.graph.root.id, type_id, "seed");
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
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id: seed.graph.root.id,
        subject: Subject::Node(node.id),
        predicate: Predicate::Property(many),
        object: Object::Value(Value::String("seed".into())),
        evidence: BTreeSet::from([evidence.id]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    seed.graph.nodes.insert(node.id, node);
    seed.graph.assertions.insert(assertion.id, assertion);
    seed.graph.evidence.insert(evidence.id, evidence);
    seed.evidence_payloads.insert(hash, bytes);
    seed
}
fn proposal(seed: &SeedDocument) -> (GraphTransaction, NodeId) {
    let id = NodeId::mint();
    let ty = &seed.ontology.node_types[0];
    let many = *ty.properties.keys().next().unwrap();
    let list = *ty.properties.keys().nth(1).unwrap();
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
                    properties: BTreeMap::from([
                        (
                            many,
                            vec![
                                Value::String("same".into()),
                                Value::String("same".into()),
                                Value::String("last".into()),
                            ],
                        ),
                        (list, vec![Value::List(vec![])]),
                    ]),
                }),
            ],
            evidence: BTreeSet::from([evidence]),
        },
        id,
    )
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

#[test]
fn changed_knowledge_reconstructs_in_a_fresh_process_on_both_providers() {
    if std::env::var_os("EKR_DURABLE_CHILD_PATH").is_some() {
        fresh_process_reader();
        return;
    }
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        let initial = kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let (tx, node) = proposal(&seed);
        let bytes = encode(&tx);
        let proposed = kernel
            .propose(&bytes, context().operator, || Timestamp::from_millis(20))
            .unwrap();
        assert_eq!(proposed.document_bytes, bytes);
        assert!(matches!(
            kernel
                .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(30))
                .unwrap(),
            ValidationCommandResult::Validated(_)
        ));
        let CommitCommandResult::Committed(result) = kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(40))
            .unwrap()
        else {
            panic!("fresh commit became stale")
        };
        assert_ne!(initial.result.knowledge_root, result.result.knowledge_root);
        assert_eq!(result.result.revision, RevisionNumber::new(1));
        std::fs::write(
            directory.path().join("expected.json"),
            serde_json::to_vec(&(result, node)).unwrap(),
        )
        .unwrap();
        drop(kernel);
        let child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "changed_knowledge_reconstructs_in_a_fresh_process_on_both_providers",
                "--nocapture",
            ])
            .env("EKR_DURABLE_CHILD_PATH", directory.path())
            .env("EKR_DURABLE_CHILD_FILE", file.to_string())
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&child.stdout),
            String::from_utf8_lossy(&child.stderr)
        );
    }
}
fn fresh_process_reader() {
    let Ok(path) = std::env::var("EKR_DURABLE_CHILD_PATH") else {
        return;
    };
    let path = std::path::Path::new(&path);
    let file = std::env::var("EKR_DURABLE_CHILD_FILE").unwrap() == "true";
    let (receipt, node): (CommitReceiptV1, NodeId) =
        serde_json::from_slice(&std::fs::read(path.join("expected.json")).unwrap()).unwrap();
    let kernel = open(path, file);
    let graph = kernel.snapshot().unwrap();
    assert_eq!(kernel.head().unwrap(), Some(receipt.result));
    let node = graph.nodes.get(&node).unwrap();
    assert_eq!(node.canonical_name, "created");
    assert_eq!(node.properties.values().next().unwrap().len(), 3);
    assert_eq!(
        node.properties.values().nth(1).unwrap(),
        &vec![CanonicalValue::List(vec![])]
    );
    assert_eq!(
        kernel
            .commit(
                receipt.proposal.transaction_id,
                context().operator,
                || panic!("retry sampled clock")
            )
            .unwrap(),
        CommitCommandResult::Committed(Box::new(receipt))
    );
}

fn submit(kernel: &Runtime, tx: &GraphTransaction) -> ValidationCommandResult {
    kernel
        .propose(&encode(tx), context().operator, || {
            Timestamp::from_millis(20)
        })
        .unwrap();
    kernel
        .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(30))
        .unwrap()
}
#[test]
fn invalid_lifecycle_candidates_are_rejected_before_sealing_on_both_providers() {
    let mut admitted = Vec::new();
    for file in [false, true] {
        for fault in [
            "self",
            "cycle",
            "replacement-retracted",
            "before-start",
            "after-end",
            "replacement-start",
            "competing",
        ] {
            let directory = tempfile::tempdir().unwrap();
            let mut seed = fixture();
            let old = *seed.graph.assertions.keys().next().unwrap();
            seed.graph.assertions.get_mut(&old).unwrap().valid_time = TemporalRange::new(
                Some(Timestamp::from_millis(0)),
                Some(Timestamp::from_millis(300)),
            )
            .unwrap();
            let (mut tx, _) = proposal(&seed);
            let GraphOperation::AddAssertion(replacement) = &mut tx.operations[0] else {
                unreachable!()
            };
            replacement.valid_time = TemporalRange::since(Timestamp::from_millis(100));
            let new = replacement.id;
            let mut change = Supersession {
                assertion: old,
                by: new,
                effective_from: Timestamp::from_millis(100),
            };
            match fault {
                "self" => change.by = old,
                "cycle" => tx
                    .operations
                    .push(GraphOperation::SupersedeAssertion(Supersession {
                        assertion: new,
                        by: old,
                        effective_from: Timestamp::from_millis(100),
                    })),
                "replacement-retracted" => {
                    tx.operations
                        .push(GraphOperation::RetractAssertion(Retraction {
                            assertion: new,
                            reason: RetractionReason::new("withdrawn"),
                        }))
                }
                "before-start" => change.effective_from = Timestamp::from_millis(-1),
                "after-end" => change.effective_from = Timestamp::from_millis(301),
                "replacement-start" => change.effective_from = Timestamp::from_millis(99),
                "competing" => tx
                    .operations
                    .push(GraphOperation::RetractAssertion(Retraction {
                        assertion: old,
                        reason: RetractionReason::new("withdrawn"),
                    })),
                _ => unreachable!(),
            }
            tx.operations
                .push(GraphOperation::SupersedeAssertion(change));
            let kernel = open(directory.path(), file);
            kernel.seed(seed, || Timestamp::from_millis(10)).unwrap();
            if !matches!(submit(&kernel, &tx), ValidationCommandResult::Rejected(_)) {
                admitted.push(format!("file={file}/{fault}"));
            }
        }
    }
    assert!(
        admitted.is_empty(),
        "lifecycle candidates accepted: {admitted:?}"
    );
}

fn native(
    path: &std::path::Path,
    file: bool,
) -> (tokio::runtime::Runtime, Box<dyn eventlog_core::EventStore>) {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let provider: Box<dyn eventlog_core::EventStore> = executor.block_on(async {
        if file {
            Box::new(eventlog_file::FileEventStore::open(path).await.unwrap())
                as Box<dyn eventlog_core::EventStore>
        } else {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(
                    &path.join("state.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap(),
            )
        }
    });
    (executor, provider)
}
fn physical(path: &std::path::Path, file: bool) -> Vec<(serde_json::Value, Option<Vec<u8>>)> {
    let (executor, provider) = native(path, file);
    executor.block_on(async {
        let tenant = eventlog_core::TenantId::new("test").unwrap();
        let mut result = Vec::new();
        let mut after = 0;
        loop {
            let page = provider.read_feed(&tenant, after, 100).await.unwrap();
            for event in page.events {
                let blob = if event.name == "ekr.store.ObjectStored" {
                    provider
                        .get_blob(&tenant, event.data["content_hash"].as_str().unwrap())
                        .await
                        .unwrap()
                } else {
                    None
                };
                result.push((serde_json::to_value(event).unwrap(), blob));
            }
            if !page.has_more {
                break;
            }
            after = page.next_position;
        }
        result
    })
}
#[test]
fn retained_terminal_states_exact_retries_and_absent_targets_survive_reopen() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        let root0 = kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let (one, _) = proposal(&seed);
        let (two, _) = proposal(&seed);
        let (pending, _) = proposal(&seed);
        let (accepted, _) = proposal(&seed);
        let (mut rejected, _) = proposal(&seed);
        if let GraphOperation::CreateNode(node) = &mut rejected.operations[1] {
            node.properties.values_mut().next().unwrap()[0] = Value::Float(f64::NAN);
        }
        let bytes = encode(&rejected);
        let input = kernel
            .propose(&bytes, context().operator, || Timestamp::from_millis(20))
            .unwrap();
        assert_eq!(input.canonical_transaction_hash, None);
        assert_eq!(input.canonical_operations_hash, None);
        assert!(
            matches!(kernel.validate(rejected.id,RevisionNumber::SEED,||Timestamp::from_millis(30)).unwrap(),ValidationCommandResult::Rejected(r) if r.issues.iter().any(|i|i.validator==ValidatorName::Type))
        );
        for tx in [&one, &two, &accepted] {
            assert!(matches!(
                submit(&kernel, tx),
                ValidationCommandResult::Validated(_)
            ));
        }
        kernel
            .propose(&encode(&pending), context().operator, || {
                Timestamp::from_millis(20)
            })
            .unwrap();
        let before = physical(directory.path(), file);
        assert!(
            matches!(kernel.validate(pending.id,RevisionNumber::new(99),||panic!("missing basis sampled time")),Err(CommitError::RevisionNotFound {against}) if against==RevisionNumber::new(99))
        );
        for call_commit in [false, true] {
            let missing = TransactionId::mint();
            let result = if call_commit {
                kernel
                    .commit(missing, context().operator, || {
                        panic!("missing target clock")
                    })
                    .map(|_| ())
            } else {
                kernel
                    .validate(missing, RevisionNumber::new(99), || {
                        panic!("missing target clock")
                    })
                    .map(|_| ())
            };
            assert!(
                matches!(result,Err(CommitError::TransactionNotFound {transaction_id}) if transaction_id==missing)
            );
        }
        assert_eq!(before, physical(directory.path(), file));
        let first = kernel
            .commit(one.id, context().operator, || Timestamp::from_millis(40))
            .unwrap();
        let stale = kernel
            .commit(two.id, context().operator, || Timestamp::from_millis(35))
            .unwrap();
        assert!(
            matches!(stale,CommitCommandResult::Stale(ref r) if r.observed_root.revision==RevisionNumber::new(1) && r.stale_at==Timestamp::from_millis(35))
        );
        let (advance, _) = proposal(&seed);
        kernel
            .propose(&encode(&advance), context().operator, || {
                Timestamp::from_millis(40)
            })
            .unwrap();
        assert!(matches!(
            kernel
                .validate(advance.id, RevisionNumber::new(1), || {
                    Timestamp::from_millis(40)
                })
                .unwrap(),
            ValidationCommandResult::Validated(_)
        ));
        kernel
            .commit(advance.id, context().operator, || {
                Timestamp::from_millis(40)
            })
            .unwrap();
        drop(kernel);
        let kernel = open(directory.path(), file);
        let before = physical(directory.path(), file);
        assert_eq!(
            kernel
                .commit(one.id, context().operator, || panic!("retry clock"))
                .unwrap(),
            first
        );
        let differently_spaced = SeedDocument::from_yaml(&format!(
            "# whitespace control\n{}\n",
            serde_yaml_ng::to_string(&seed).unwrap()
        ))
        .unwrap();
        assert_eq!(
            kernel
                .seed(differently_spaced, || panic!("seed retry clock"))
                .unwrap(),
            root0
        );
        for (id, state) in [
            (pending.id, TransactionState::Proposed),
            (rejected.id, TransactionState::Rejected),
            (two.id, TransactionState::Stale),
        ] {
            assert!(
                matches!(kernel.commit(id,context().operator,||panic!("wrong state clock")),Err(CommitError::TransactionStateConflict {transaction_id,state:actual}) if transaction_id==id && actual==state)
            );
        }
        let transactions = kernel.transactions().unwrap();
        assert_eq!(
            transactions[&accepted.id].state(),
            TransactionState::Validated
        );
        assert_eq!(transactions[&one.id].state(), TransactionState::Committed);
        assert_eq!(transactions[&rejected.id].proposal.document_bytes, bytes);
        assert_eq!(transactions[&rejected.id].proposal, input);
        assert_eq!(before, physical(directory.path(), file));
        let mut changed = anchor();
        changed.agents.get_mut(&context().operator).unwrap().name = "changed".into();
        let foreign = if file {
            Runtime::file(directory.path(), "test", context(), changed)
        } else {
            Runtime::sqlite(
                &directory.path().join("state.db"),
                "test",
                context(),
                changed,
            )
        }
        .unwrap();
        assert!(matches!(
            foreign.head(),
            Err(ekr_store::StoreError::AuthorityMismatch)
        ));
        assert!(matches!(
            foreign.seed(seed, || panic!("changed anchor clock")),
            Err(SeedError::Store(ekr_store::StoreError::AlreadySeeded))
        ));
        assert_eq!(before, physical(directory.path(), file));
    }
}
#[test]
fn supersession_and_retraction_preserve_attribution_and_both_time_axes() {
    for file in [false, true] {
        for reverse in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let seed = fixture();
            let old = *seed.graph.assertions.keys().next().unwrap();
            let (mut tx, _) = proposal(&seed);
            let GraphOperation::AddAssertion(a) = &mut tx.operations[0] else {
                unreachable!()
            };
            let new = a.id;
            a.valid_time = TemporalRange::since(Timestamp::from_millis(100));
            tx.operations
                .push(GraphOperation::SupersedeAssertion(Supersession {
                    assertion: old,
                    by: new,
                    effective_from: Timestamp::from_millis(100),
                }));
            if reverse {
                tx.operations.reverse();
            }
            let kernel = open(directory.path(), file);
            kernel.seed(seed, || Timestamp::from_millis(10)).unwrap();
            assert!(matches!(
                submit(&kernel, &tx),
                ValidationCommandResult::Validated(_)
            ));
            kernel
                .commit(tx.id, context().operator, || Timestamp::from_millis(40))
                .unwrap();
            let graph = kernel.snapshot().unwrap();
            assert_eq!(
                GraphSnapshot::of(&graph)
                    .valid_at(Timestamp::from_millis(99))
                    .iter()
                    .map(|a| a.id)
                    .collect::<Vec<_>>(),
                vec![old]
            );
            assert_eq!(
                GraphSnapshot::of(&graph)
                    .valid_at(Timestamp::from_millis(100))
                    .iter()
                    .map(|a| a.id)
                    .collect::<Vec<_>>(),
                vec![new]
            );
            assert_eq!(
                graph.assertions[&old].transaction_time.recorded_from,
                Timestamp::from_millis(10)
            );
            assert_eq!(
                graph.assertions[&old].transaction_time.recorded_to,
                Some(Timestamp::from_millis(40))
            );
            assert_eq!(
                graph.assertions[&new].transaction_time.recorded_from,
                Timestamp::from_millis(40)
            );
            let before = graph.assertions[&new].clone();
            let withdraw = GraphTransaction {
                id: TransactionId::mint(),
                proposer: context().operator,
                operations: vec![GraphOperation::RetractAssertion(Retraction {
                    assertion: new,
                    reason: RetractionReason::new("corrected evidence"),
                })],
                evidence: BTreeSet::new(),
            };
            kernel
                .propose(&encode(&withdraw), context().operator, || {
                    Timestamp::from_millis(40)
                })
                .unwrap();
            assert!(matches!(
                kernel
                    .validate(withdraw.id, RevisionNumber::new(1), || {
                        Timestamp::from_millis(40)
                    })
                    .unwrap(),
                ValidationCommandResult::Validated(_)
            ));
            kernel
                .commit(withdraw.id, context().operator, || {
                    Timestamp::from_millis(40)
                })
                .unwrap();
            drop(kernel);
            let kernel = open(directory.path(), file);
            let graph = kernel.snapshot().unwrap();
            assert!(!graph.assertions[&new].valid_at(Timestamp::from_millis(100)));
            assert_eq!(graph.assertions[&new].assessment, before.assessment);
            assert_eq!(graph.assertions[&new].evidence, before.evidence);
            assert_eq!(graph.assertions[&new].proposed_by, before.proposed_by);
            assert!(
                matches!(&graph.assertions[&new].lifecycle,AssertionLifecycle::Retracted {reason,..} if reason.as_str()=="corrected evidence")
            );
            assert!(
                kernel.replay(RevisionNumber::new(1)).unwrap().assertions[&new]
                    .valid_at(Timestamp::from_millis(100))
            );
            assert!(
                kernel.replay(RevisionNumber::SEED).unwrap().assertions[&old]
                    .valid_at(Timestamp::from_millis(100))
            );
        }
    }
}

// This port fault sits immediately around the real store, not in the kernel authority. It models
// the unresolved response explicitly; it is not evidence of a native disk fault.
type PublicationInterceptor<S> = std::rc::Rc<
    std::cell::RefCell<
        Option<
            Box<
                dyn FnMut(
                    &S,
                    &ekr_store::PublicationPreparationV1,
                ) -> Result<ekr_store::Appended, ekr_store::StoreError>,
            >,
        >,
    >,
>;
struct PublicationProbe<S> {
    inner: S,
    observed: std::rc::Rc<std::cell::RefCell<Vec<ekr_store::Publication>>>,
    intercept: PublicationInterceptor<S>,
}
impl<S: ekr_store::RevisionLog> ekr_store::RevisionLog for PublicationProbe<S> {
    fn preparation(
        &self,
        key: &ekr_store::PublicationCommandKey,
    ) -> Result<Option<ekr_store::PublicationPreparationV1>, ekr_store::StoreError> {
        self.inner.preparation(key)
    }
    fn prepare(
        &self,
        key: &ekr_store::PublicationCommandKey,
        input: ContentHash,
        decision: &ekr_store::Publication,
        previous: Option<&ekr_store::PublicationPreparationV1>,
    ) -> Result<ekr_store::PublicationPreparationV1, ekr_store::StoreError> {
        self.inner.prepare(key, input, decision, previous)
    }
    fn resume(
        &self,
        p: &ekr_store::PublicationPreparationV1,
    ) -> Result<ekr_store::Appended, ekr_store::StoreError> {
        self.observed.borrow_mut().push(p.decision.clone());
        match self.intercept.borrow_mut().as_mut() {
            Some(f) => f(&self.inner, p),
            None => self.inner.resume(p),
        }
    }
    fn history(&self) -> Result<ekr_store::RetainedHistory, ekr_store::StoreError> {
        self.inner.history()
    }
    fn history_at(
        &self,
        r: RevisionNumber,
    ) -> Result<ekr_store::RetainedHistory, ekr_store::StoreError> {
        self.inner.history_at(r)
    }
    fn publish(
        &self,
        p: &ekr_store::Publication,
    ) -> Result<ekr_store::Appended, ekr_store::StoreError> {
        self.inner.publish(p)
    }
    fn seed_bytes(&self) -> Result<Option<Vec<u8>>, ekr_store::StoreError> {
        self.inner.seed_bytes()
    }
    fn fold(&self) -> Result<CanonicalGraph, ekr_store::StoreError> {
        self.inner.fold()
    }
    fn head(&self) -> Result<Option<Root>, ekr_store::StoreError> {
        self.inner.head()
    }
    fn replay(&self, r: RevisionNumber) -> Result<CanonicalGraph, ekr_store::StoreError> {
        self.inner.replay(r)
    }
}
impl<S: ekr_store::Initialize> ekr_store::Initialize for PublicationProbe<S> {
    fn initialize(
        &self,
        p: &ekr_store::Publication,
    ) -> Result<ekr_store::Appended, ekr_store::StoreError> {
        self.inner.initialize(p)
    }
}
impl<S: ekr_store::ObjectStore> ekr_store::ObjectStore for PublicationProbe<S> {
    fn put(
        &self,
        c: ekr_store::StorageClass,
        b: &[u8],
        t: Timestamp,
    ) -> Result<ekr_store::StoredObject, ekr_store::StoreError> {
        self.inner.put(c, b, t)
    }
    fn get(&self, h: &ContentHash) -> Result<Option<Vec<u8>>, ekr_store::StoreError> {
        self.inner.get(h)
    }
}

#[test]
fn unresolved_publication_cannot_be_replaced_by_a_new_occurrence() {
    use ekr_store::{SqliteStore, StoreError};
    let directory = tempfile::tempdir().unwrap();
    let seed = fixture();
    let (tx, _) = proposal(&seed);
    let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let intercept = std::rc::Rc::new(std::cell::RefCell::new(None));
    let kernel = Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(PublicationProbe {
            inner: SqliteStore::sqlite(&directory.path().join("state.db"), "test", None)?
                .under(authority),
            observed: observed.clone(),
            intercept: intercept.clone(),
        })
    })
    .unwrap();
    kernel.seed(seed, || Timestamp::from_millis(10)).unwrap();
    kernel
        .propose(&encode(&tx), context().operator, || {
            Timestamp::from_millis(20)
        })
        .unwrap();
    kernel
        .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(30))
        .unwrap();
    observed.borrow_mut().clear();
    *intercept.borrow_mut() = Some(Box::new(|_, _| Err(StoreError::UnknownCommit)));
    assert!(matches!(
        kernel.commit(tx.id, context().operator, || Timestamp::from_millis(40)),
        Err(CommitError::Store(StoreError::UnknownCommit))
    ));
    assert!(matches!(
        kernel.commit(tx.id, context().operator, || Timestamp::from_millis(41)),
        Err(CommitError::Store(StoreError::UnknownCommit))
    ));
    let observed = observed.borrow();
    assert_eq!(observed.len(), 2);
    assert_eq!(
        observed[0], observed[1],
        "an unresolved publication must keep the exact occurrence, payload and original time"
    );
}

#[test]
fn historical_capture_stops_loading_at_the_selected_revision_and_keeps_its_context() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        let initial = kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let (tx, node) = proposal(&seed);
        submit(&kernel, &tx);
        kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(40))
            .unwrap();
        let captured = kernel.read(Some(RevisionNumber::new(1))).unwrap();
        assert_eq!(captured.context, context());
        assert_eq!(captured.authority, anchor());
        assert_eq!(captured.seed_input, seed);
        assert_eq!(captured.seed, initial);
        assert!(captured.graph.nodes.contains_key(&node));
        assert!(captured.revisions.contains_key(&RevisionNumber::SEED));
        assert_eq!(
            captured.transactions[&tx.id].state(),
            TransactionState::Committed
        );
        assert_eq!(
            captured.revisions[&RevisionNumber::new(1)].root,
            captured.root
        );
        for (hash, bytes) in &seed.evidence_payloads {
            assert_eq!(captured.content(hash), Some(bytes.as_slice()));
        }
        assert!(
            matches!(kernel.read(Some(RevisionNumber::new(99))),Err(CommitError::RevisionNotFound {against}) if against==RevisionNumber::new(99))
        );
        let (later, _) = proposal(&seed);
        let record = kernel
            .propose(&encode(&later), context().operator, || {
                Timestamp::from_millis(50)
            })
            .unwrap();
        let hash = ContentHash::of_bytes(&record.to_bytes().unwrap());
        drop(kernel);
        let (executor, provider) = native(directory.path(), file);
        executor
            .block_on(provider.delete_blob(
                &eventlog_core::TenantId::new("test").unwrap(),
                &hash.to_hex(),
            ))
            .unwrap();
        drop(provider);
        drop(executor);
        let before = physical(directory.path(), file);
        let kernel = open(directory.path(), file);
        assert!(kernel.head().is_err());
        assert!(kernel.snapshot().is_err());
        assert!(kernel.transactions().is_err());
        assert!(kernel.read(None).is_err());
        let old = kernel.read(Some(RevisionNumber::new(1))).unwrap();
        assert_eq!(old.root, captured.root);
        assert_eq!(old.transactions, captured.transactions);
        assert_eq!(
            kernel.replay(RevisionNumber::new(1)).unwrap().nodes,
            captured.graph.nodes
        );
        assert_eq!(
            kernel.read(Some(RevisionNumber::SEED)).unwrap().root,
            initial.result
        );
        assert_eq!(physical(directory.path(), file), before);
    }
}

#[test]
fn bounded_proposal_ingress_and_typed_input_refusals_leave_no_partial_record() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let before = physical(directory.path(), file);
        assert!(matches!(
            kernel.propose_reader(
                std::io::Cursor::new(b"wrong"),
                context().operator,
                || panic!("malformed clock")
            ),
            Err(CommitError::Document(_))
        ));
        let (tx, _) = proposal(&seed);
        assert!(
            matches!(kernel.propose(&encode(&tx),context().validator,||panic!("attribution clock")),Err(CommitError::ProposalAttribution {actor}) if actor==context().validator)
        );
        assert_eq!(physical(directory.path(), file), before);
        let bytes = encode(&tx);
        assert_eq!(
            kernel
                .propose_reader(std::io::Cursor::new(&bytes), context().operator, || {
                    Timestamp::from_millis(20)
                })
                .unwrap()
                .document_bytes,
            bytes
        );
    }
}

#[test]
fn admitted_operation_permutations_apply_equal_graphs_with_clear_edges_and_invocation() {
    use ekr_ontology::{EdgeType, Lifecycle, OperationDefinition, Transition};
    let mut seed = fixture();
    let ty = &mut seed.ontology.node_types[0];
    let type_id = ty.id;
    let property = *ty.properties.keys().next().unwrap();
    let transition = Transition::new("Open", "Done");
    ty.lifecycle = Some(Lifecycle {
        initial: "Open".into(),
        states: BTreeSet::from(["Open".into(), "Done".into()]),
        transitions: BTreeSet::from([transition.clone()]),
    });
    let mut op = OperationDefinition::new("Finish");
    op.transition = Some(transition);
    ty.operations.insert("Finish".into(), op);
    let old = *seed.graph.nodes.keys().next().unwrap();
    let held = seed.graph.nodes.get_mut(&old).unwrap();
    held.type_state = Some("Open".into());
    held.properties
        .insert(property, vec![Value::String("will clear".into())]);
    let edge_type = TypeId::mint();
    let mut relation = EdgeType::new(edge_type, "Related");
    relation.source_types.insert(type_id);
    relation.target_types.insert(type_id);
    relation.cardinality = Cardinality::Many;
    seed.ontology.edge_types.push(relation);
    let (mut tx, node) = proposal(&seed);
    let kept = EdgeId::mint();
    let canceled = EdgeId::mint();
    for id in [kept, canceled] {
        tx.operations.push(GraphOperation::CreateEdge(EdgeDraft {
            id,
            root_id: seed.graph.root.id,
            type_id: edge_type,
            source: old,
            target: node,
            properties: BTreeMap::new(),
        }));
    }
    tx.operations.push(GraphOperation::DeleteEdge(canceled));
    tx.operations
        .push(GraphOperation::UpdateProperty(PropertyMutation {
            node: old,
            property,
            values: vec![],
        }));
    tx.operations.push(GraphOperation::Invoke {
        node,
        operation: "Finish".into(),
        arguments: BTreeMap::new(),
    });
    let mut outputs = Vec::new();
    for file in [false, true] {
        for reverse in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let kernel = open(directory.path(), file);
            kernel
                .seed(seed.clone(), || Timestamp::from_millis(10))
                .unwrap();
            let mut candidate = tx.clone();
            if reverse {
                candidate.operations.reverse();
            }
            assert!(matches!(
                submit(&kernel, &candidate),
                ValidationCommandResult::Validated(_)
            ));
            let CommitCommandResult::Committed(result) = kernel
                .commit(candidate.id, context().operator, || {
                    Timestamp::from_millis(40)
                })
                .unwrap()
            else {
                panic!("commit stale")
            };
            let graph = kernel.snapshot().unwrap();
            assert!(!graph.nodes[&old].properties.contains_key(&property));
            assert_eq!(graph.nodes[&node].type_state.as_deref(), Some("Done"));
            assert!(graph.edges.contains_key(&kept));
            assert!(!graph.edges.contains_key(&canceled));
            outputs.push((
                graph,
                result.result.knowledge_root,
                result.result.transaction,
            ));
        }
    }
    for item in &outputs[1..] {
        assert_eq!(item.0.nodes, outputs[0].0.nodes);
        assert_eq!(item.0.edges, outputs[0].0.edges);
        assert_eq!(item.0.assertions, outputs[0].0.assertions);
        assert_eq!(item.1, outputs[0].1);
    }
    assert_ne!(
        outputs[0].2, outputs[1].2,
        "operation vector order remains part of the actual transaction hash"
    );
}

#[test]
fn backwards_trusted_times_refuse_without_new_objects_or_decisions() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let (tx, _) = proposal(&seed);
        kernel
            .propose(&encode(&tx), context().operator, || {
                Timestamp::from_millis(20)
            })
            .unwrap();
        let before = physical(directory.path(), file);
        assert!(kernel
            .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(19))
            .is_err());
        assert_eq!(physical(directory.path(), file), before);
        kernel
            .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(30))
            .unwrap();
        let before = physical(directory.path(), file);
        assert!(kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(29))
            .is_err());
        assert_eq!(physical(directory.path(), file), before);
        assert_eq!(
            kernel.transactions().unwrap()[&tx.id].state(),
            TransactionState::Validated
        );
        kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(30))
            .unwrap();
    }
}

fn contention<
    S: ekr_store::RevisionLog + ekr_store::ObjectStore + ekr_store::Initialize + 'static,
>(
    directory: &std::path::Path,
    file: bool,
    canonical: bool,
    lost_response: bool,
    open_store: impl FnOnce(KernelAuthority) -> Result<S, ekr_store::StoreError>,
) {
    use ekr_store::StoreError;
    let seed = fixture();
    let (tx, _) = proposal(&seed);
    let (other, _) = proposal(&seed);
    let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let intercept = std::rc::Rc::new(std::cell::RefCell::new(None));
    let kernel = Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(PublicationProbe {
            inner: open_store(authority)?,
            observed: observed.clone(),
            intercept: intercept.clone(),
        })
    })
    .unwrap();
    kernel
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    for candidate in [&tx, &other] {
        kernel
            .propose(&encode(candidate), context().operator, || {
                Timestamp::from_millis(20)
            })
            .unwrap();
        kernel
            .validate(candidate.id, RevisionNumber::SEED, || {
                Timestamp::from_millis(30)
            })
            .unwrap();
    }
    observed.borrow_mut().clear();
    let path = directory.to_path_buf();
    let (unrelated, _) = proposal(&seed);
    let mut first = true;
    *intercept.borrow_mut() = Some(Box::new(
        move |inner: &S, p: &ekr_store::PublicationPreparationV1| {
            if first {
                first = false;
                if lost_response {
                    let _ = inner.resume(p)?;
                    return Err(StoreError::UnknownCommit);
                }
                let competing = open(&path, file);
                if canonical {
                    competing
                        .commit(other.id, context().operator, || Timestamp::from_millis(41))
                        .unwrap();
                } else {
                    competing
                        .propose(&encode(&unrelated), context().operator, || {
                            Timestamp::from_millis(41)
                        })
                        .unwrap();
                }
            }
            inner.resume(p)
        },
    ));
    let result = kernel.commit(tx.id, context().operator, || Timestamp::from_millis(40));
    if lost_response {
        assert!(matches!(
            result,
            Err(CommitError::Store(StoreError::UnknownCommit))
        ));
        let first = CommitReceiptV1::from_bytes(
            observed.borrow()[0]
                .objects
                .values()
                .next()
                .unwrap()
                .bytes
                .as_slice(),
        )
        .unwrap();
        drop(kernel);
        let reopened = open(directory, file);
        let (advance, _) = proposal(&seed);
        reopened
            .propose(&encode(&advance), context().operator, || {
                Timestamp::from_millis(50)
            })
            .unwrap();
        reopened
            .validate(advance.id, RevisionNumber::new(1), || {
                Timestamp::from_millis(50)
            })
            .unwrap();
        reopened
            .commit(advance.id, context().operator, || {
                Timestamp::from_millis(50)
            })
            .unwrap();
        let before = physical(directory, file);
        assert_eq!(
            reopened
                .commit(tx.id, context().operator, || panic!(
                    "lost response retry clock"
                ))
                .unwrap(),
            CommitCommandResult::Committed(Box::new(first))
        );
        assert_eq!(physical(directory, file), before);
    } else {
        let result = result.unwrap();
        let candidates = observed.borrow();
        assert_eq!(candidates.len(), 2);
        assert_ne!(
            candidates[0].expected_version,
            candidates[1].expected_version
        );
        if canonical {
            let CommitCommandResult::Stale(stale) = result else {
                panic!("lost canonical race applied")
            };
            assert_eq!(stale.stale_at, Timestamp::from_millis(40));
            assert_ne!(candidates[0].event.event_id, candidates[1].event.event_id);
            let (executor, provider) = native(directory, file);
            assert!(executor
                .block_on(provider.get_blob(
                    &eventlog_core::TenantId::new("test").unwrap(),
                    &candidates[0].event.record_hash.to_hex()
                ))
                .unwrap()
                .is_none());
            assert_eq!(
                kernel.head().unwrap().unwrap().revision,
                RevisionNumber::new(1)
            );
        } else {
            let CommitCommandResult::Committed(receipt) = result else {
                panic!("noncanonical movement became stale")
            };
            assert_eq!(receipt.committed_at, Timestamp::from_millis(40));
            assert_eq!(candidates[0].event, candidates[1].event);
            assert_eq!(candidates[0].objects, candidates[1].objects);
        }
    }
}
#[test]
fn independent_handles_distinguish_noncanonical_contention_canonical_loss_and_lost_response() {
    for file in [false, true] {
        for (canonical, lost) in [(false, false), (true, false), (false, true)] {
            let directory = tempfile::tempdir().unwrap();
            if file {
                contention(directory.path(), file, canonical, lost, |authority| {
                    Ok(
                        ekr_store::FileStore::file(directory.path(), "test", None)?
                            .under(authority),
                    )
                });
            } else {
                contention(directory.path(), file, canonical, lost, |authority| {
                    Ok(ekr_store::SqliteStore::sqlite(
                        &directory.path().join("state.db"),
                        "test",
                        None,
                    )?
                    .under(authority))
                });
            }
        }
    }
}

fn captured_history(path: &std::path::Path) -> ekr_store::RetainedHistory {
    use ekr_store::RevisionLog;
    let mut history = None;
    let _kernel = Commit::over_with_authority(context(), anchor(), |authority| {
        let store =
            ekr_store::SqliteStore::sqlite(&path.join("state.db"), "test", None)?.under(authority);
        history = Some(store.history()?);
        Ok(store)
    })
    .unwrap();
    history.unwrap()
}
fn install_history(path: &std::path::Path, file: bool, history: &ekr_store::RetainedHistory) {
    use eventlog_core::{CommandMeta, Expected, NewEvent, StreamId, TenantId};
    let (executor, provider) = native(path, file);
    executor.block_on(async {
        let tenant = TenantId::new("test").unwrap();
        let meta = |key: String| CommandMeta {
            idempotency_key: key.clone(),
            request_hash: key.clone(),
            subject: "fixture".into(),
            actor: "fixture".into(),
            request_id: key.clone(),
            trace_id: key,
            causation_id: None,
            causation_depth: 0,
            occurred_at: ::time::OffsetDateTime::UNIX_EPOCH,
            claim: None,
        };
        for (hash, object) in &history.objects {
            provider
                .put_blob(&tenant, &hash.to_hex(), &object.bytes)
                .await
                .unwrap();
            let stream = StreamId::new(tenant.clone(), "ekr.store.object", hash.to_hex()).unwrap();
            let event = NewEvent::new(
                "ekr.store.ObjectStored",
                2,
                serde_json::to_value(&object.metadata).unwrap(),
            )
            .unwrap();
            provider
                .append(
                    &stream,
                    Expected::NoStream,
                    &[event],
                    &meta(format!("object-{hash}")),
                )
                .await
                .unwrap();
        }
        let stream = StreamId::new(tenant, "ekr.revision", "canonical").unwrap();
        let events = history
            .occurrences
            .iter()
            .map(|o| {
                NewEvent::new(o.event.name(), 2, serde_json::to_value(&o.event).unwrap()).unwrap()
            })
            .collect::<Vec<_>>();
        provider
            .append(
                &stream,
                Expected::NoStream,
                &events,
                &meta("history".into()),
            )
            .await
            .unwrap();
    });
}
#[test]
fn all_six_occurrences_refuse_readdressed_forged_and_missing_records_on_both_providers() {
    let source = tempfile::tempdir().unwrap();
    let seed = fixture();
    let kernel = open(source.path(), false);
    kernel
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    let (one, _) = proposal(&seed);
    let (two, _) = proposal(&seed);
    submit(&kernel, &one);
    submit(&kernel, &two);
    kernel
        .commit(one.id, context().operator, || Timestamp::from_millis(40))
        .unwrap();
    kernel
        .commit(two.id, context().operator, || Timestamp::from_millis(40))
        .unwrap();
    let (mut rejected, _) = proposal(&seed);
    if let GraphOperation::CreateNode(node) = &mut rejected.operations[1] {
        node.type_id = TypeId::mint();
    }
    assert!(matches!(
        submit(&kernel, &rejected),
        ValidationCommandResult::Rejected(_)
    ));
    drop(kernel);
    let history = captured_history(source.path());
    for file in [false, true] {
        let control = tempfile::tempdir().unwrap();
        install_history(control.path(), file, &history);
        assert_eq!(
            open(control.path(), file).head().unwrap().unwrap().revision,
            RevisionNumber::new(1)
        );
        let mut seen = BTreeSet::new();
        for (index, occurrence) in history.occurrences.iter().enumerate() {
            if !seen.insert(occurrence.event.name()) {
                continue;
            }
            for mode in ["semantics", "unknown", "duplicate", "missing"] {
                let mut forged = history.clone();
                forged.occurrences.truncate(index + 1);
                let old = occurrence.event.record_hash;
                let mut value: serde_json::Value =
                    serde_json::from_slice(&forged.objects[&old].bytes).unwrap();
                match mode {
                    "missing" => {
                        forged.objects.remove(&old);
                    }
                    _ => {
                        if mode == "unknown" {
                            value["undeclared_authority"] = true.into();
                        }
                        if mode == "semantics" {
                            match occurrence.event.payload {
                                RevisionPayload::Seeded { .. } => {
                                    value["authority_root"] =
                                        serde_json::to_value(ContentHash::of_bytes(b"forged"))
                                            .unwrap()
                                }
                                RevisionPayload::TransactionProposed { .. }
                                | RevisionPayload::TransactionValidated { .. } => {
                                    value["operation_count"] = 999.into()
                                }
                                RevisionPayload::RevisionCommitted { .. } => {
                                    value["result"]["ontology_root"] =
                                        serde_json::to_value(ContentHash::of_bytes(b"forged"))
                                            .unwrap()
                                }
                                RevisionPayload::TransactionRejected { .. } => {
                                    value["issues"][0]["message"] = "forged issue".into()
                                }
                                RevisionPayload::TransactionStale { .. } => {
                                    value["observed_root_hash"] =
                                        serde_json::to_value(ContentHash::of_bytes(b"forged"))
                                            .unwrap()
                                }
                            }
                        }
                        let bytes = if mode == "duplicate" {
                            let encoded = serde_json::to_string(&value).unwrap();
                            format!("{{\"format\":{},{}", value["format"], &encoded[1..])
                                .into_bytes()
                        } else {
                            serde_json::to_vec(&value).unwrap()
                        };
                        let hash = ContentHash::of_bytes(&bytes);
                        let mut object = forged.objects.remove(&old).unwrap();
                        object.metadata.content_hash = hash;
                        object.metadata.byte_len = bytes.len() as u64;
                        object.bytes = bytes;
                        forged.objects.insert(hash, object);
                        forged.occurrences.last_mut().unwrap().event.record_hash = hash;
                    }
                }
                let directory = tempfile::tempdir().unwrap();
                install_history(directory.path(), file, &forged);
                let before = physical(directory.path(), file);
                let reopened = open(directory.path(), file);
                assert!(
                    reopened.head().is_err(),
                    "{} {mode} file={file} was admitted",
                    occurrence.event.name()
                );
                assert!(reopened.read(None).is_err());
                assert_eq!(physical(directory.path(), file), before);
            }
        }
        assert_eq!(seen.len(), 6);
    }
}

fn pending_seed_input_conflict<
    S: ekr_store::RevisionLog + ekr_store::ObjectStore + ekr_store::Initialize,
>(
    directory: &std::path::Path,
    file: bool,
    open_store: impl FnOnce(KernelAuthority) -> Result<S, ekr_store::StoreError>,
) {
    use ekr_store::StoreError;
    let seed = fixture();
    let observed = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let intercept = std::rc::Rc::new(std::cell::RefCell::new(None));
    let kernel = Commit::over_with_authority(context(), anchor(), |authority| {
        Ok(PublicationProbe {
            inner: open_store(authority)?,
            observed: observed.clone(),
            intercept: intercept.clone(),
        })
    })
    .unwrap();
    *intercept.borrow_mut() = Some(Box::new(|_, _| Err(StoreError::UnknownCommit)));
    assert_eq!(
        kernel.seed(seed.clone(), || Timestamp::from_millis(10)),
        Err(SeedError::Store(StoreError::UnknownCommit))
    );
    assert_eq!(observed.borrow().len(), 1);
    drop(kernel);
    let before = physical(directory, file);
    assert_eq!(
        before
            .iter()
            .filter(|(event, _)| event["name"] == "ekr.store.PublicationPrepared")
            .count(),
        1
    );
    assert!(!before
        .iter()
        .any(|(event, _)| event["name"] == "ekr.store.ObjectStored"));
    let reopened = open(directory, file);
    assert_eq!(reopened.head().unwrap(), None);
    let mut changed = seed.clone();
    changed
        .graph
        .nodes
        .values_mut()
        .next()
        .unwrap()
        .canonical_name = "different admitted seed".into();
    assert_eq!(
        reopened.seed(changed, || panic!("pending refusal sampled time")),
        Err(SeedError::Store(StoreError::PublicationInputConflict))
    );
    assert_eq!(physical(directory, file), before);
    let recovered = reopened
        .seed(seed, || panic!("pending recovery sampled time"))
        .unwrap();
    assert_eq!(recovered.committed_at, Timestamp::from_millis(10));
    assert_eq!(reopened.head().unwrap(), Some(recovered.result));
}

#[test]
fn pending_different_seed_refuses_input_conflict_without_publication_or_clock() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        if file {
            pending_seed_input_conflict(directory.path(), file, |authority| {
                Ok(ekr_store::FileStore::file(directory.path(), "test", None)?.under(authority))
            });
        } else {
            pending_seed_input_conflict(directory.path(), file, |authority| {
                Ok(ekr_store::SqliteStore::sqlite(
                    &directory.path().join("state.db"),
                    "test",
                    None,
                )?
                .under(authority))
            });
        }
    }
}

#[test]
fn published_different_seed_refuses_already_seeded_without_publication_or_clock() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        let original = kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        drop(kernel);
        let before = physical(directory.path(), file);
        let mut changed = seed;
        changed
            .graph
            .nodes
            .values_mut()
            .next()
            .unwrap()
            .canonical_name = "different admitted seed".into();
        let reopened = open(directory.path(), file);
        assert_eq!(
            reopened.seed(changed, || panic!("published refusal sampled time")),
            Err(SeedError::Store(ekr_store::StoreError::AlreadySeeded))
        );
        assert_eq!(physical(directory.path(), file), before);
        assert_eq!(reopened.head().unwrap(), Some(original.result));
    }
}
