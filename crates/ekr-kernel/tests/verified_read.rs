//! A verified read carries exactly the records the durable command handlers returned, on both
//! providers, at the head and at a historical boundary, and after a reopen.
//!
//! The seed and proposal fixtures are the minimum copied from `durable_commands.rs`.
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
fn proposal(seed: &SeedDocument) -> GraphTransaction {
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
            }),
        ],
        evidence: BTreeSet::from([evidence]),
        schema_version: None,
    }
}
/// The same proposal with a value canonical state refuses, so the type validator rejects it.
fn inadmissible(seed: &SeedDocument) -> GraphTransaction {
    let mut tx = proposal(seed);
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
fn propose(kernel: &Runtime, tx: &GraphTransaction) -> ProposalRecordV1 {
    kernel
        .propose(&encode(tx), context().operator, at(20))
        .unwrap()
}
fn validated(kernel: &Runtime, tx: &GraphTransaction) -> ValidationReceiptV1 {
    match kernel
        .validate(tx.id, RevisionNumber::SEED, at(30))
        .unwrap()
    {
        ValidationCommandResult::Validated(receipt) => receipt,
        ValidationCommandResult::Rejected(record) => panic!("rejected: {:?}", record.issues),
    }
}

/// The bytes a read holds under a payload address are the bytes that address names.
fn retained<'a>(read: &'a VerifiedRead, hash: &ContentHash) -> &'a [u8] {
    let bytes = read
        .content(hash)
        .unwrap_or_else(|| panic!("{hash:?} is not in the read"));
    assert_eq!(ContentHash::of_bytes(bytes), *hash);
    bytes
}

/// One revision's coordinates, checked against the record that published it.
fn assert_revision(
    read: &VerifiedRead,
    revision: &VerifiedRevision,
    revision_id: RevisionId,
    event_id: EventId,
    committed_at: Timestamp,
    root: Root,
    record_bytes: &[u8],
) {
    assert_eq!(revision.revision_id, revision_id);
    assert_eq!(revision.event_id, event_id);
    assert_eq!(revision.committed_at, committed_at);
    assert_eq!(revision.root, root);
    assert_eq!(retained(read, &revision.record_hash), record_bytes);
}

/// What the handlers returned for one transaction, in the shape a read must report it.
struct Returned {
    proposal: ProposalRecordV1,
    validation: Option<ValidationReceiptV1>,
    rejection: Option<RejectionRecordV1>,
    committed: Option<CommitReceiptV1>,
    stale: Option<StaleRecordV1>,
}
fn assert_record(read: &VerifiedRead, record: &TransactionRecord, returned: &Returned) {
    assert_eq!(record.proposal, returned.proposal);
    assert_eq!(
        retained(read, &record.proposal_record_hash),
        returned.proposal.to_bytes().unwrap()
    );
    assert_eq!(record.validation, returned.validation);
    assert_eq!(
        record
            .validation_record_hash
            .map(|hash| retained(read, &hash).to_vec()),
        returned
            .validation
            .as_ref()
            .map(|receipt| receipt.to_bytes().unwrap())
    );
    assert_eq!(record.rejection, returned.rejection);
    assert_eq!(record.committed, returned.committed);
    assert_eq!(record.stale, returned.stale);
}

#[test]
fn a_verified_read_carries_every_committed_record_on_both_providers() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        let seeded = kernel.seed(seed.clone(), at(10)).unwrap();

        let (one, two, accepted, pending, refused) = (
            proposal(&seed),
            proposal(&seed),
            proposal(&seed),
            proposal(&seed),
            inadmissible(&seed),
        );
        let mut returned: BTreeMap<TransactionId, Returned> = BTreeMap::new();
        for tx in [&one, &two, &accepted, &pending, &refused] {
            returned.insert(
                tx.id,
                Returned {
                    proposal: propose(&kernel, tx),
                    validation: None,
                    rejection: None,
                    committed: None,
                    stale: None,
                },
            );
        }
        for tx in [&one, &two, &accepted] {
            returned.get_mut(&tx.id).unwrap().validation = Some(validated(&kernel, tx));
        }
        let ValidationCommandResult::Rejected(rejection) = kernel
            .validate(refused.id, RevisionNumber::SEED, at(30))
            .unwrap()
        else {
            panic!("an inadmissible value was validated")
        };
        returned.get_mut(&refused.id).unwrap().rejection = Some(rejection);
        let CommitCommandResult::Committed(receipt) =
            kernel.commit(one.id, context().operator, at(40)).unwrap()
        else {
            panic!("the first commit became stale")
        };
        let CommitCommandResult::Stale(stale) =
            kernel.commit(two.id, context().operator, at(45)).unwrap()
        else {
            panic!("a commit against a superseded basis was applied")
        };
        returned.get_mut(&one.id).unwrap().committed = Some(*receipt.clone());
        returned.get_mut(&two.id).unwrap().stale = Some(*stale);

        let read: VerifiedRead = kernel.read(None).unwrap();
        assert_eq!(read.graph, kernel.snapshot().unwrap());
        assert_eq!(Some(read.root), kernel.head().unwrap());
        assert_eq!(read.root, receipt.result);
        assert_eq!(read.seed, seeded);
        assert_eq!(read.seed_input, seed);
        assert_eq!(read.context, context());
        assert_eq!(read.authority, anchor());
        for (hash, bytes) in &seed.evidence_payloads {
            assert_eq!(retained(&read, hash), bytes.as_slice());
        }

        assert_eq!(
            read.revisions.keys().copied().collect::<Vec<_>>(),
            [RevisionNumber::SEED, RevisionNumber::new(1)]
        );
        assert_revision(
            &read,
            &read.revisions[&RevisionNumber::SEED],
            seeded.revision_id,
            seeded.event_id,
            Timestamp::from_millis(10),
            seeded.result,
            &seeded.to_bytes().unwrap(),
        );
        assert_revision(
            &read,
            &read.revisions[&RevisionNumber::new(1)],
            receipt.revision_id,
            receipt.event_id,
            Timestamp::from_millis(40),
            receipt.result,
            &receipt.to_bytes().unwrap(),
        );
        assert_ne!(
            read.revisions[&RevisionNumber::SEED].revision_id,
            read.revisions[&RevisionNumber::new(1)].revision_id
        );

        assert_eq!(read.transactions, kernel.transactions().unwrap());
        assert_eq!(
            read.transactions.keys().collect::<BTreeSet<_>>(),
            returned.keys().collect::<BTreeSet<_>>()
        );
        for (id, state) in [
            (one.id, TransactionState::Committed),
            (two.id, TransactionState::Stale),
            (accepted.id, TransactionState::Validated),
            (pending.id, TransactionState::Proposed),
            (refused.id, TransactionState::Rejected),
        ] {
            let record: &TransactionRecord = &read.transactions[&id];
            assert_eq!(record.state(), state, "file={file}");
            assert_record(&read, record, &returned[&id]);
        }

        drop(kernel);
        let reopened = open(directory.path(), file).read(None).unwrap();
        assert_eq!(reopened.graph, read.graph);
        assert_eq!(reopened.root, read.root);
        assert_eq!(reopened.seed, read.seed);
        assert_eq!(reopened.seed_input, read.seed_input);
        assert_eq!(reopened.revisions, read.revisions);
        assert_eq!(reopened.transactions, read.transactions);
    }
}

#[test]
fn a_historical_read_reports_each_record_as_it_stood_at_that_boundary() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        let seeded = kernel.seed(seed.clone(), at(10)).unwrap();
        let (one, two) = (proposal(&seed), proposal(&seed));
        propose(&kernel, &one);
        propose(&kernel, &two);
        validated(&kernel, &one);
        let validation = validated(&kernel, &two);
        kernel.commit(one.id, context().operator, at(40)).unwrap();
        let CommitCommandResult::Stale(stale) =
            kernel.commit(two.id, context().operator, at(45)).unwrap()
        else {
            panic!("a commit against a superseded basis was applied")
        };

        // At revision 1 the second transaction had been validated and not yet found stale: the
        // later decision is not read back into an earlier boundary.
        let at_one: VerifiedRead = kernel.read(Some(RevisionNumber::new(1))).unwrap();
        let before: &TransactionRecord = &at_one.transactions[&two.id];
        assert_eq!(before.state(), TransactionState::Validated);
        assert_eq!(before.validation.as_ref(), Some(&validation));
        assert_eq!(before.stale, None);
        let head = kernel.read(None).unwrap();
        let after: &TransactionRecord = &head.transactions[&two.id];
        assert_eq!(after.state(), TransactionState::Stale);
        assert_eq!(after.stale.as_ref(), Some(stale.as_ref()));

        // The seed boundary holds the seed revision alone and no transaction at all.
        let at_seed: VerifiedRead = kernel.read(Some(RevisionNumber::SEED)).unwrap();
        assert_eq!(
            at_seed.revisions.keys().copied().collect::<Vec<_>>(),
            [RevisionNumber::SEED]
        );
        let only: &VerifiedRevision = &at_seed.revisions[&RevisionNumber::SEED];
        assert_eq!(only.root, seeded.result);
        assert_eq!(at_seed.root, seeded.result);
        assert!(at_seed.transactions.is_empty(), "file={file}");
        assert_eq!(at_seed.seed, seeded);
        assert_eq!(at_seed.graph, kernel.replay(RevisionNumber::SEED).unwrap());
    }
}

#[test]
fn recorded_validation_issues_are_the_pipeline_findings_under_retained_identities() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.clone(), at(10)).unwrap();
        let refused = inadmissible(&seed);
        let bytes = encode(&refused);
        kernel.propose(&bytes, context().operator, at(20)).unwrap();
        let ValidationCommandResult::Rejected(rejection) = kernel
            .validate(refused.id, RevisionNumber::SEED, at(30))
            .unwrap()
        else {
            panic!("an inadmissible value was validated")
        };

        // The pure findings for the exact retained bytes against the seed graph.
        let basis = kernel.read(Some(RevisionNumber::SEED)).unwrap().graph;
        let document = TransactionDocument::parse(&bytes).unwrap();
        let findings = Pipeline::deterministic(context().validator)
            .validate(&GraphSnapshot::of(&basis), document.transaction())
            .expect_err("the pipeline admits what the handler refused");

        let issues: &Vec<RecordedValidationIssue> = &rejection.issues;
        assert!(!issues.is_empty());
        assert_eq!(issues.len(), findings.len());
        assert!(issues
            .iter()
            .any(|issue| issue.validator == ValidatorName::Type));
        for (held, finding) in issues.iter().zip(&findings) {
            assert_eq!(held.transaction_id, refused.id);
            assert_eq!(held.transaction_id, finding.transaction_id);
            assert_eq!(held.validator, finding.validator);
            assert_eq!(held.code, finding.code);
            assert_eq!(held.message, finding.message);
        }
        let ids: BTreeSet<IssueId> = issues.iter().map(|issue| issue.id).collect();
        assert_eq!(ids.len(), issues.len(), "two findings share one identity");

        // The identities are retained, not minted again by a read or by a reopen.
        let read = kernel.read(None).unwrap();
        let held = read.transactions[&refused.id].rejection.as_ref().unwrap();
        assert_eq!(&held.issues, issues);
        drop(kernel);
        let reopened = open(directory.path(), file).transactions().unwrap();
        let again = reopened[&refused.id].rejection.as_ref().unwrap();
        assert_eq!(&again.issues, issues);
        let decoded = RejectionRecordV1::from_bytes(&rejection.to_bytes().unwrap()).unwrap();
        assert_eq!(&decoded.issues, issues);
    }
}

#[test]
fn validation_material_addresses_exactly_the_transaction_basis_and_validators() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.clone(), at(10)).unwrap();
        let (tx, other) = (proposal(&seed), proposal(&seed));
        let proposed = propose(&kernel, &tx);
        let receipt = validated(&kernel, &tx);

        let canonical: GraphTransaction<CanonicalValue> =
            TransactionDocument::parse(&proposed.document_bytes)
                .unwrap()
                .transaction()
                .clone()
                .try_into()
                .unwrap();
        assert_eq!(ContentHash::of(&canonical), receipt.transaction_hash);
        let material = ValidationMaterialV1 {
            transaction: &canonical,
            basis: &receipt.basis,
            validators: &receipt.validators,
        };
        assert_eq!(ContentHash::of(&material), receipt.validation_hash);
        assert_eq!(receipt.validators, BTreeSet::from([context().validator]));

        // Each of the three fields reaches the address.
        let different: GraphTransaction<CanonicalValue> = other.try_into().unwrap();
        let mut basis = receipt.basis.clone();
        basis.previous_event_id = EventId::mint();
        let validators = BTreeSet::from([context().validator, context().operator]);
        for (field, changed) in [
            (
                "transaction",
                ValidationMaterialV1 {
                    transaction: &different,
                    ..material
                },
            ),
            (
                "basis",
                ValidationMaterialV1 {
                    basis: &basis,
                    ..material
                },
            ),
            (
                "validators",
                ValidationMaterialV1 {
                    validators: &validators,
                    ..material
                },
            ),
        ] {
            assert_ne!(
                ContentHash::of(&changed),
                receipt.validation_hash,
                "{field} does not reach the validation address"
            );
        }

        // The commit receipt and a reopened read carry the same address, recomputable from the
        // retained record alone.
        let CommitCommandResult::Committed(committed) =
            kernel.commit(tx.id, context().operator, at(40)).unwrap()
        else {
            panic!("a fresh commit became stale")
        };
        assert_eq!(
            committed.validation.validation_hash,
            receipt.validation_hash
        );
        drop(kernel);
        let read = open(directory.path(), file).read(None).unwrap();
        let held = read.transactions[&tx.id].validation.as_ref().unwrap();
        assert_eq!(
            ContentHash::of(&ValidationMaterialV1 {
                transaction: &canonical,
                basis: &held.basis,
                validators: &held.validators,
            }),
            receipt.validation_hash
        );
    }
}
