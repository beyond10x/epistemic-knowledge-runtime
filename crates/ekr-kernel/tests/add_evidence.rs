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
