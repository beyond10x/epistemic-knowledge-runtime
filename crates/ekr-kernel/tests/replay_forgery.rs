//! Forged retained lineages written straight through both native providers, then reopened
//! through the real kernel authority: each is refused with its own named `StoreError`, and the
//! refusal leaves every provider byte where it was.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{Cardinality, NodeType, PropertyDefinition, Value, ValueType};
use ekr_store::{RecordedOccurrence, RetainedHistory, StoreError};
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
            schema_version: None,
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
/// Every provider event and every stored blob, in feed order.
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
fn captured_history(path: &std::path::Path) -> RetainedHistory {
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
/// Writes objects and revision occurrences straight through the native provider, bypassing
/// every kernel and store check: this is how a forged lineage reaches retained state.
fn install_history(path: &std::path::Path, file: bool, history: &RetainedHistory) {
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

/// A genuine retained lineage holding every decision kind, and the identities it decided.
struct Source {
    history: RetainedHistory,
    committed: TransactionId,
    stale: TransactionId,
    rejected: TransactionId,
}
/// Seed; two proposals validated at the seed; the first commits, the second is then stale; a
/// third is rejected.
fn source() -> Source {
    let directory = tempfile::tempdir().unwrap();
    let seed = fixture();
    let kernel = open(directory.path(), false);
    kernel
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    let (one, _) = proposal(&seed);
    let (two, _) = proposal(&seed);
    assert!(matches!(
        submit(&kernel, &one),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        submit(&kernel, &two),
        ValidationCommandResult::Validated(_)
    ));
    assert!(matches!(
        kernel
            .commit(one.id, context().operator, || Timestamp::from_millis(40))
            .unwrap(),
        CommitCommandResult::Committed(_)
    ));
    assert!(matches!(
        kernel
            .commit(two.id, context().operator, || Timestamp::from_millis(40))
            .unwrap(),
        CommitCommandResult::Stale(_)
    ));
    let (mut rejected, _) = proposal(&seed);
    if let GraphOperation::CreateNode(node) = &mut rejected.operations[1] {
        node.type_id = TypeId::mint();
    }
    assert!(matches!(
        submit(&kernel, &rejected),
        ValidationCommandResult::Rejected(_)
    ));
    drop(kernel);
    Source {
        history: captured_history(directory.path()),
        committed: one.id,
        stale: two.id,
        rejected: rejected.id,
    }
}
fn position(history: &RetainedHistory, found: impl Fn(&RevisionPayload) -> bool) -> usize {
    history
        .occurrences
        .iter()
        .position(|o| found(&o.event.payload))
        .expect("the source lineage holds this occurrence")
}
fn commit_of(history: &RetainedHistory, id: TransactionId) -> usize {
    position(
        history,
        |p| matches!(p, RevisionPayload::RevisionCommitted { transaction_id, .. } if *transaction_id == id),
    )
}
/// A copy of `occurrence` under a fresh domain identity, so only the claim it makes is new.
fn reissued(occurrence: &RecordedOccurrence) -> RecordedOccurrence {
    let mut copy = occurrence.clone();
    copy.event.event_id = EventId::mint();
    copy
}

/// Installs `forged` on both providers, reopens each through `Runtime::file`/`Runtime::sqlite`,
/// and requires exactly `expected` from every canonical read, with no provider byte changed.
fn refused_on_both_providers(forged: &RetainedHistory, expected: &StoreError) {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        install_history(directory.path(), file, forged);
        let before = physical(directory.path(), file);
        let reopened = open(directory.path(), file);
        assert_eq!(
            reopened.head().as_ref().map_err(Clone::clone).err(),
            Some(expected.clone()),
            "head file={file}"
        );
        assert_eq!(
            reopened.snapshot().err(),
            Some(expected.clone()),
            "snapshot file={file}"
        );
        assert!(
            matches!(reopened.read(None), Err(CommitError::Store(ref e)) if e == expected),
            "read file={file}"
        );
        assert_eq!(
            physical(directory.path(), file),
            before,
            "refusal changed provider bytes file={file}"
        );
    }
}

#[test]
fn the_unforged_source_lineage_reopens_on_both_providers() {
    let source = source();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        install_history(directory.path(), file, &source.history);
        let reopened = open(directory.path(), file);
        assert_eq!(
            reopened.head().unwrap().unwrap().revision,
            RevisionNumber::new(1)
        );
        let states = reopened.transactions().unwrap();
        assert_eq!(
            states[&source.committed].state(),
            TransactionState::Committed
        );
        assert_eq!(states[&source.stale].state(), TransactionState::Stale);
        assert_eq!(states[&source.rejected].state(), TransactionState::Rejected);
    }
}

#[test]
fn a_commit_whose_revision_does_not_follow_its_parent_is_refused_out_of_order() {
    let source = source();
    let mut forged = source.history.clone();
    let at = commit_of(&forged, source.committed);
    forged.occurrences.truncate(at + 1);
    let RevisionPayload::RevisionCommitted { number, .. } =
        &mut forged.occurrences[at].event.payload
    else {
        unreachable!()
    };
    *number = RevisionNumber::new(2);
    refused_on_both_providers(
        &forged,
        &StoreError::RevisionOutOfOrder {
            expected: RevisionNumber::new(1),
            found: RevisionNumber::new(2),
        },
    );
}

#[test]
fn a_second_seed_occurrence_is_refused_as_seed_is_not_first() {
    let source = source();
    let mut forged = source.history.clone();
    let seed = forged.occurrences[0].clone();
    forged.occurrences.truncate(2);
    forged.occurrences.push(reissued(&seed));
    refused_on_both_providers(&forged, &StoreError::SeedIsNotFirst);
}

#[test]
fn a_first_occurrence_that_is_not_a_seed_is_refused_as_not_seeded() {
    let source = source();
    let mut forged = source.history.clone();
    forged.occurrences.remove(0);
    assert!(!matches!(
        forged.occurrences[0].event.payload,
        RevisionPayload::Seeded { .. }
    ));
    refused_on_both_providers(&forged, &StoreError::NotSeeded);
}

#[test]
fn a_decision_with_no_retained_proposal_is_refused_as_proposal_missing() {
    let source = source();
    let mut forged = source.history.clone();
    let id = source.committed;
    let proposed = position(
        &forged,
        |p| matches!(p, RevisionPayload::TransactionProposed { transaction_id, .. } if *transaction_id == id),
    );
    let validated = position(
        &forged,
        |p| matches!(p, RevisionPayload::TransactionValidated { transaction_id, .. } if *transaction_id == id),
    );
    forged.occurrences.truncate(validated + 1);
    forged.occurrences.remove(proposed);
    refused_on_both_providers(&forged, &StoreError::ProposalMissing { transaction_id: id });
}

/// The genuine lineage, then a commit claim for a transaction whose retained state is terminal.
fn commit_after_terminal(source: &Source, id: TransactionId) -> RetainedHistory {
    let mut forged = source.history.clone();
    let mut claim = reissued(&forged.occurrences[commit_of(&forged, source.committed)]);
    let RevisionPayload::RevisionCommitted { transaction_id, .. } = &mut claim.event.payload else {
        unreachable!()
    };
    *transaction_id = id;
    forged.occurrences.push(claim);
    forged
}

#[test]
fn a_commit_after_a_stale_decision_is_refused_as_validation_missing() {
    let source = source();
    let forged = commit_after_terminal(&source, source.stale);
    refused_on_both_providers(
        &forged,
        &StoreError::ValidationMissing {
            transaction_id: source.stale,
        },
    );
}

#[test]
fn a_commit_after_a_rejected_decision_is_refused_as_validation_missing() {
    let source = source();
    let forged = commit_after_terminal(&source, source.rejected);
    refused_on_both_providers(
        &forged,
        &StoreError::ValidationMissing {
            transaction_id: source.rejected,
        },
    );
}

#[test]
fn a_commit_payload_knowledge_root_that_disagrees_with_its_receipt_is_refused() {
    let source = source();
    let mut forged = source.history.clone();
    let at = commit_of(&forged, source.committed);
    forged.occurrences.truncate(at + 1);
    let receipt = CommitReceiptV1::from_bytes(
        &forged.objects[&forged.occurrences[at].event.record_hash].bytes,
    )
    .unwrap();
    let published = ContentHash::of_bytes(b"forged knowledge root");
    let RevisionPayload::RevisionCommitted { knowledge_root, .. } =
        &mut forged.occurrences[at].event.payload
    else {
        unreachable!()
    };
    *knowledge_root = published;
    refused_on_both_providers(
        &forged,
        &StoreError::KnowledgeRootDisagrees {
            revision: RevisionNumber::new(1),
            published,
            folded: receipt.result.knowledge_root,
        },
    );
}
