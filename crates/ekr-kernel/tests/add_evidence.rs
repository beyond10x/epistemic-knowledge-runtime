//! `story:add-evidence-operation`: evidence enters canonical state after the seed through an
//! `AddEvidence` operation inside a validated transaction, on both providers.
//!
//! Acceptance, as the story states it:
//!
//! * a transaction with `AddEvidence` and an `AddAssertion` citing it validates and commits on
//!   both providers, and full replay reproduces the root;
//! * a payload whose hash does not match, a reused evidence id, and an assertion citing evidence
//!   that no revision holds are refused by name.
//!
//! Every case drives the durable handlers through [`Runtime`], and reads what they retained back
//! from the provider: the payload's own stored object, the verified read, and a reopen that
//! replays the whole history from the seed.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::BTreeSet;

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
fn at(millis: i64) -> impl FnOnce() -> Timestamp {
    move || Timestamp::from_millis(millis)
}

/// A seed with one node type, one node and one seeded evidence entry.
struct Seeded {
    document: SeedDocument,
    node: NodeId,
    property: PropertyId,
    seeded_evidence: EvidenceId,
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
    let bytes = b"seeded human statement".to_vec();
    let seeded_evidence = human_evidence(EvidenceId::mint(), &bytes);
    document.graph.nodes.insert(node.id, node.clone());
    document
        .graph
        .evidence
        .insert(seeded_evidence.id, seeded_evidence.clone());
    document
        .evidence_payloads
        .insert(seeded_evidence.content_hash, bytes.into());
    Seeded {
        document,
        node: node.id,
        property,
        seeded_evidence: seeded_evidence.id,
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

fn assertion(seed: &Seeded, evidence: EvidenceId, label: &str) -> Assertion<Value> {
    Assertion {
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
    }
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

/// Proposes `tx` and validates it against the head, returning the verdict.
fn verdict(runtime: &Runtime, tx: &GraphTransaction, now: i64) -> ValidationCommandResult {
    runtime
        .propose(&encode(tx), context().operator, at(now))
        .unwrap();
    let head = runtime.head().unwrap().unwrap().revision;
    runtime.validate(tx.id, head, at(now + 1)).unwrap()
}

/// Every issue code of a rejection, and the validator that raised it.
fn refused(result: &ValidationCommandResult) -> Vec<(String, ValidatorName)> {
    match result {
        ValidationCommandResult::Rejected(record) => record
            .issues
            .iter()
            .map(|issue| (issue.code.clone(), issue.validator))
            .collect(),
        ValidationCommandResult::Validated(_) => panic!("validated: {result:?}"),
    }
}

fn committed(runtime: &Runtime, tx: &GraphTransaction, now: i64) -> CommitReceiptV1 {
    assert!(
        matches!(
            verdict(runtime, tx, now),
            ValidationCommandResult::Validated(_)
        ),
        "{tx:?}"
    );
    match runtime
        .commit(tx.id, context().operator, at(now + 2))
        .unwrap()
    {
        CommitCommandResult::Committed(receipt) => *receipt,
        CommitCommandResult::Stale(stale) => panic!("stale: {stale:?}"),
    }
}

#[test]
fn added_evidence_and_the_assertion_citing_it_commit_and_replay_on_both_providers() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();

        let payload = b"one message, said once".to_vec();
        let evidence = human_evidence(EvidenceId::mint(), &payload);
        let claim = assertion(&seed, evidence.id, "from the message");
        let tx = transaction(vec![
            add_evidence(evidence.clone(), &payload),
            GraphOperation::AddAssertion(Box::new(claim.clone())),
        ]);
        let before = runtime.published_events().unwrap().len();
        let receipt = committed(&runtime, &tx, 20);
        assert_eq!(receipt.result.revision, RevisionNumber::new(1));

        // The payload is its own stored object, Provenance class, published with the commit;
        // no event body carries its bytes.
        let published = runtime.published_events().unwrap();
        let stored: Vec<_> = published[before..]
            .iter()
            .filter(|event| event.name == "ekr.store.ObjectStored")
            .filter(|event| event.data["content_hash"] == evidence.content_hash.to_string())
            .collect();
        assert_eq!(stored.len(), 1, "{published:#?}");
        assert_eq!(stored[0].data["storage_class"], "Provenance");
        assert_eq!(stored[0].data["byte_len"], payload.len());
        for event in &published {
            let body = event.data.to_string();
            assert!(
                !body.contains("one message, said once"),
                "{} carries the payload",
                event.name
            );
        }

        let snapshot = runtime.snapshot().unwrap();
        assert_eq!(snapshot.evidence.get(&evidence.id), Some(&evidence));
        assert!(snapshot.evidence.contains_key(&seed.seeded_evidence));
        assert_eq!(
            runtime.content(&evidence.content_hash).unwrap(),
            Some(payload.clone())
        );

        // A later transaction may cite it.
        let later = assertion(&seed, evidence.id, "cited again");
        let receipt_2 = committed(
            &runtime,
            &transaction(vec![GraphOperation::AddAssertion(Box::new(later.clone()))]),
            30,
        );
        assert_eq!(receipt_2.result.revision, RevisionNumber::new(2));

        // Explain reaches the added evidence and its retained bytes.
        let read = runtime.read(None).unwrap();
        for id in [claim.id, later.id] {
            let explained = read.explain(id).unwrap();
            assert!(
                explained
                    .links
                    .iter()
                    .any(|link| matches!(link, ExplanationLink::Evidence(e) if *e == evidence)),
                "{explained:?}"
            );
        }
        assert_eq!(
            read.content(&evidence.content_hash),
            Some(payload.as_slice())
        );
        let head = runtime.head().unwrap().unwrap();
        assert_eq!(head, receipt_2.result);
        drop(read);
        drop(runtime);

        // A fresh open that replays every revision from the seed reproduces the root.
        let mut reopened = open(directory.path(), file);
        reopened.set_full_replay(true);
        assert_eq!(reopened.head().unwrap().unwrap(), head);
        assert_eq!(reopened.snapshot().unwrap().evidence, snapshot.evidence);
        assert_eq!(
            reopened.replay(RevisionNumber::new(1)).unwrap().evidence,
            snapshot.evidence
        );
        let read = reopened.read(None).unwrap();
        assert_eq!(read.root, head);
        assert_eq!(
            read.content(&evidence.content_hash),
            Some(payload.as_slice())
        );
        drop(read);
        drop(reopened);

        // And a fresh open that continues from the replay checkpoint agrees.
        let reopened = open(directory.path(), file);
        assert_eq!(reopened.head().unwrap().unwrap(), head);
        assert!(reopened.read(None).unwrap().explain(later.id).is_ok());
    }
}

#[test]
fn a_payload_that_does_not_hash_to_its_entry_is_refused_as_evidence_payload_mismatch() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let evidence = human_evidence(EvidenceId::mint(), b"what the entry names");
        let tx = transaction(vec![
            add_evidence(evidence.clone(), b"other bytes"),
            GraphOperation::AddAssertion(Box::new(assertion(&seed, evidence.id, "x"))),
        ]);
        let result = verdict(&runtime, &tx, 20);
        assert_eq!(
            refused(&result),
            vec![(
                "evidence-payload-mismatch".to_owned(),
                ValidatorName::Provenance
            )]
        );
        let ValidationCommandResult::Rejected(record) = result else {
            unreachable!()
        };
        let message = &record.issues[0].message;
        assert!(
            message.contains(&evidence.content_hash.to_string())
                && message.contains(&ContentHash::of_bytes(b"other bytes").to_string()),
            "{message}"
        );
        assert_eq!(
            runtime.head().unwrap().unwrap().revision,
            RevisionNumber::SEED
        );
    }
}

#[test]
fn a_reused_evidence_id_is_refused_by_name() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();

        // An id the seed already holds.
        let payload = b"a second statement".to_vec();
        let reused = human_evidence(seed.seeded_evidence, &payload);
        let result = verdict(
            &runtime,
            &transaction(vec![add_evidence(reused, &payload)]),
            20,
        );
        assert_eq!(
            refused(&result),
            vec![(
                "identity-already-exists".to_owned(),
                ValidatorName::Structural
            )]
        );

        // One id added twice by one transaction.
        let id = EvidenceId::mint();
        let result = verdict(
            &runtime,
            &transaction(vec![
                add_evidence(human_evidence(id, b"first"), b"first"),
                add_evidence(human_evidence(id, b"second"), b"second"),
            ]),
            30,
        );
        assert_eq!(
            refused(&result),
            vec![("duplicate-identity".to_owned(), ValidatorName::Structural)]
        );

        // An id a committed AddEvidence introduced.
        let first = human_evidence(EvidenceId::mint(), b"committed once");
        committed(
            &runtime,
            &transaction(vec![add_evidence(first.clone(), b"committed once")]),
            40,
        );
        let result = verdict(
            &runtime,
            &transaction(vec![add_evidence(
                human_evidence(first.id, b"committed twice"),
                b"committed twice",
            )]),
            50,
        );
        assert_eq!(
            refused(&result),
            vec![(
                "identity-already-exists".to_owned(),
                ValidatorName::Structural
            )]
        );
    }
}

#[test]
fn an_assertion_citing_evidence_no_revision_holds_is_refused_as_unresolved_evidence() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let absent = EvidenceId::mint();
        let payload = b"a statement nobody cites".to_vec();
        let tx = transaction(vec![
            // Evidence this transaction does add, so the refusal is about the other id only.
            add_evidence(human_evidence(EvidenceId::mint(), &payload), &payload),
            GraphOperation::AddAssertion(Box::new(assertion(&seed, absent, "x"))),
        ]);
        let result = verdict(&runtime, &tx, 20);
        assert_eq!(
            refused(&result),
            vec![("unresolved-evidence".to_owned(), ValidatorName::Reference)]
        );
        let ValidationCommandResult::Rejected(record) = result else {
            unreachable!()
        };
        assert!(
            record.issues[0].message.contains(&absent.to_string()),
            "{}",
            record.issues[0].message
        );
    }
}

#[test]
fn evidence_from_a_source_other_than_a_human_statement_is_refused_by_name() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let payload = b"https://example.invalid/page".to_vec();
        let mut evidence = human_evidence(EvidenceId::mint(), &payload);
        evidence.source = EvidenceSource::Url("https://example.invalid/page".into());
        let result = verdict(
            &runtime,
            &transaction(vec![add_evidence(evidence, &payload)]),
            20,
        );
        assert_eq!(
            refused(&result),
            vec![(
                "evidence-unsupported-source".to_owned(),
                ValidatorName::Provenance
            )]
        );
    }
}

#[test]
fn evidence_extracted_by_another_agent_than_the_proposer_is_misattributed() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let payload = b"said by someone else".to_vec();
        let mut evidence = human_evidence(EvidenceId::mint(), &payload);
        evidence.extracted_by = context().validator;
        let tx = transaction(vec![add_evidence(evidence, &payload)]);
        let refused = runtime.propose(&encode(&tx), context().operator, at(20));
        assert!(
            matches!(refused, Err(CommitError::ProposalAttribution { actor }) if actor == context().operator),
            "{refused:?}"
        );
        assert!(runtime.transactions().unwrap().is_empty());
    }
}

/// `task:validate-cost-flat-with-store-size`: the retained objects one evidence-carrying
/// transaction's head read, proposal, validation and commit place into the histories they replay
/// do not grow with the evidence the store already holds, on both providers.
///
/// Every payload an earlier commit added is retained and was checked when the handle first
/// replayed past it; a command replays only the occurrences after the state the handle already
/// reached, which reads only the payloads those occurrences add. Placing every earlier payload
/// into each command's history again made each verb cost the evidence count. Two stores with the
/// same transactions, one holding eight times the evidence of the other, load the same objects
/// for the same next transaction. A verified read still holds every payload
/// ([`a_verified_read_still_holds_every_added_payload`]).
#[test]
fn a_command_loads_the_same_objects_however_much_evidence_the_store_holds() {
    /// The objects the seventh transaction loads, after six that add `per` payloads each.
    fn seventh(file: bool, per: usize) -> u64 {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let mut loaded = 0;
        for n in 0..7_i64 {
            let added = if n == 6 { 1 } else { per };
            let operations = (0..added)
                .flat_map(|k| {
                    let payload = format!("message {n}.{k}").into_bytes();
                    let evidence = human_evidence(EvidenceId::mint(), &payload);
                    let claim = assertion(&seed, evidence.id, &format!("claim {n}.{k}"));
                    [
                        add_evidence(evidence, &payload),
                        GraphOperation::AddAssertion(Box::new(claim)),
                    ]
                })
                .collect();
            let _ = ekr_store::objects_loaded();
            committed(&runtime, &transaction(operations), 20 + 10 * n);
            loaded = ekr_store::objects_loaded();
        }
        loaded
    }
    for file in [false, true] {
        let (fewer, more) = (seventh(file, 1), seventh(file, 8));
        eprintln!("file {file}: retained objects loaded into command histories: {fewer} / {more}");
        assert_eq!(
            more, fewer,
            "file {file}: objects the seventh transaction placed into histories over 7 and over \
             49 payloads"
        );
    }
}

/// [`a_command_loads_the_same_objects_however_much_evidence_the_store_holds`] leaves a verified
/// read as it was: it holds the payload of every evidence entry the lineage added.
#[test]
fn a_verified_read_still_holds_every_added_payload() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let runtime = open(directory.path(), file);
        runtime.seed(seed.document.clone(), at(10)).unwrap();
        let mut payloads = Vec::new();
        for n in 0..4_i64 {
            let payload = format!("message {n}").into_bytes();
            let evidence = human_evidence(EvidenceId::mint(), &payload);
            let claim = assertion(&seed, evidence.id, &format!("claim {n}"));
            committed(
                &runtime,
                &transaction(vec![
                    add_evidence(evidence.clone(), &payload),
                    GraphOperation::AddAssertion(Box::new(claim)),
                ]),
                20 + 10 * n,
            );
            payloads.push((evidence.content_hash, payload));
        }
        let read = runtime.read(None).unwrap();
        for (hash, payload) in &payloads {
            assert_eq!(read.content(hash), Some(payload.as_slice()), "file {file}");
            assert_eq!(
                runtime.content(hash).unwrap().as_deref(),
                Some(payload.as_slice()),
                "file {file}"
            );
        }
    }
}

/// A cached command may omit the bytes it already verified, but not the stream check that
/// withdraws those bytes. Compare its named refusal with a complete history read, repeatedly:
/// a failed fast replay must not clear the condition and make its next attempt succeed.
#[test]
fn cached_commands_refuse_withdrawn_evidence_as_complete_history_does() {
    use eventlog_core::{CommandMeta, EventStore, Expected, NewEvent, StreamId, TenantId};
    for file in [false, true] {
        for (reopened, cold) in [(false, false), (true, false), (false, true)] {
            let directory = tempfile::tempdir().unwrap();
            let seed = seeded();
            let mut runtime = open(directory.path(), file);
            runtime.seed(seed.document.clone(), at(10)).unwrap();
            let bytes = b"later withdrawn evidence";
            let evidence = human_evidence(EvidenceId::mint(), bytes);
            committed(
                &runtime,
                &transaction(vec![
                    add_evidence(evidence.clone(), bytes),
                    GraphOperation::AddAssertion(Box::new(assertion(&seed, evidence.id, "held"))),
                ]),
                20,
            );
            runtime.retain_checkpoint_at_rest();
            if reopened {
                drop(runtime);
                runtime = open(directory.path(), file);
            }
            let tx = transaction(vec![GraphOperation::AddAssertion(Box::new(assertion(
                &seed,
                evidence.id,
                "next",
            )))]);
            assert!(matches!(
                verdict(&runtime, &tx, 30),
                ValidationCommandResult::Validated(_)
            ));
            runtime.transactions().unwrap();
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            let stream = StreamId::new(
                TenantId::new("test").unwrap(),
                "ekr.store.object",
                evidence.content_hash.to_hex(),
            )
            .unwrap();
            let event = NewEvent::new("test.WithdrawalRecorded", 1, serde_json::json!({})).unwrap();
            let unrelated = StreamId::new(
                TenantId::new("test").unwrap(),
                "ekr.store.object",
                ContentHash::of_bytes(b"not a dependency of the history").to_hex(),
            )
            .unwrap();
            let meta = CommandMeta {
                idempotency_key: EventId::mint().to_string(),
                request_hash: "withdrawal".into(),
                subject: "test".into(),
                actor: "test".into(),
                request_id: "withdrawal".into(),
                trace_id: "withdrawal".into(),
                causation_id: None,
                causation_depth: 0,
                occurred_at: ::time::OffsetDateTime::UNIX_EPOCH,
                claim: None,
            };
            if file {
                let provider = rt
                    .block_on(eventlog_file::FileEventStore::open(directory.path()))
                    .unwrap();
                rt.block_on(provider.append(
                    &unrelated,
                    Expected::Any,
                    std::slice::from_ref(&event),
                    &meta,
                ))
                .unwrap();
                assert!(
                    runtime.transactions().is_ok(),
                    "unrelated invalid stream must not refuse history"
                );
                rt.block_on(provider.append(&stream, Expected::Any, &[event], &meta))
                    .unwrap();
            } else {
                let db = directory.path().join("state.db");
                let provider = rt
                    .block_on(eventlog_sqlite::SqliteEventStore::open(
                        db.to_str().unwrap(),
                        "ekr",
                    ))
                    .unwrap();
                rt.block_on(provider.append(
                    &unrelated,
                    Expected::Any,
                    std::slice::from_ref(&event),
                    &meta,
                ))
                .unwrap();
                assert!(
                    runtime.transactions().is_ok(),
                    "unrelated invalid stream must not refuse history"
                );
                rt.block_on(provider.append(&stream, Expected::Any, &[event], &meta))
                    .unwrap();
            }
            if cold {
                drop(runtime);
                runtime = open(directory.path(), file);
            }
            let fresh = open(directory.path(), file)
                .read(None)
                .err()
                .unwrap()
                .to_string();
            assert!(fresh.contains("unsupported-retention-envelope"), "{fresh}");
            for _ in 0..2 {
                let answers = [
                    runtime
                        .transactions()
                        .map(|_| ())
                        .map_err(|e| e.to_string()),
                    runtime
                        .propose(&encode(&transaction(vec![])), context().operator, at(40))
                        .map(|_| ())
                        .map_err(|e| e.to_string()),
                    runtime
                        .validate(tx.id, RevisionNumber::new(0), at(41))
                        .map(|_| ())
                        .map_err(|e| e.to_string()),
                    runtime
                        .commit(tx.id, context().operator, at(42))
                        .map(|_| ())
                        .map_err(|e| e.to_string()),
                ];
                for (verb, answer) in ["transactions", "propose", "validate historical", "commit"]
                    .into_iter()
                    .zip(answers)
                {
                    assert_eq!(
                        answer,
                        Err(fresh.clone()),
                        "file {file}, reopened {reopened}, {verb}"
                    );
                }
            }
        }
    }
}

/// A checkpoint-backed command handle must recover an old validation basis introduced by a
/// different handle, including evidence the checkpoint replay ordinarily leaves unloaded.
#[test]
fn adversary_peer_validation_at_an_old_basis_matches_full_replay() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let writer = open(directory.path(), file);
        writer.seed(seed.document.clone(), at(10)).unwrap();
        let mut first_evidence = None;
        for n in 0..6 {
            let bytes = format!("old basis evidence {n}").into_bytes();
            let evidence = human_evidence(EvidenceId::mint(), &bytes);
            first_evidence.get_or_insert(evidence.id);
            committed(
                &writer,
                &transaction(vec![
                    add_evidence(evidence.clone(), &bytes),
                    GraphOperation::AddAssertion(Box::new(assertion(&seed, evidence.id, "held"))),
                ]),
                20 + 10 * n,
            );
        }
        writer.retain_checkpoint_at_rest();
        drop(writer);
        let held = open(directory.path(), file);
        let before = held.transactions().unwrap();
        assert_eq!(before.len(), 6);
        let peer = open(directory.path(), file);
        let tx = transaction(vec![GraphOperation::AddAssertion(Box::new(assertion(
            &seed,
            first_evidence.unwrap(),
            "validated at old revision",
        )))]);
        peer.propose(&encode(&tx), context().operator, at(100))
            .unwrap();
        assert!(matches!(
            peer.validate(tx.id, RevisionNumber::new(1), at(101))
                .unwrap(),
            ValidationCommandResult::Validated(_)
        ));
        let mut full = open(directory.path(), file);
        full.set_full_replay(true);
        let expected = full.transactions().unwrap();
        assert_eq!(expected.len(), 7);
        assert_eq!(held.transactions().unwrap(), expected);
        let result = held.commit(tx.id, context().operator, at(102)).unwrap();
        let CommitCommandResult::Stale(stale) = result else {
            panic!("old-basis validation committed")
        };
        assert_eq!(
            stale.expected_basis.previous_root.revision,
            RevisionNumber::new(1)
        );
        assert_eq!(stale.observed_root.revision, RevisionNumber::new(6));
        assert_eq!(held.transactions().unwrap(), full.transactions().unwrap());
    }
}
