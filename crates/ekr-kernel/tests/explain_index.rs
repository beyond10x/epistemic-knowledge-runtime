//! `story:explain-reads-an-index`: `explain` looks an assertion's chain up instead of parsing every
//! committed document, and answers by reference, on both providers.
//!
//! * The answer carries the operations about the explained assertion and references the rest of
//!   its origin transaction by hash, so an assertion committed in a large transaction is explained
//!   in about as many bytes as one committed alone.
//! * Only the documents of the transactions on the chain are read: with every other committed
//!   document made unreadable in the capture, the chain is the one it was before, and a chain whose
//!   own document is unreadable is refused rather than shortened.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{Cardinality, NodeType, PropertyDefinition, Value, ValueType};
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

/// A seed with one node whose one property takes many values, and one HumanStatement evidence.
struct Seeded {
    document: SeedDocument,
    node: NodeId,
    property: PropertyId,
    evidence: EvidenceId,
}
fn fixture() -> Seeded {
    let mut seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let type_id = "00000000-0000-4000-8000-000000000005".parse().unwrap();
    let many = "00000000-0000-4000-8000-000000000006".parse().unwrap();
    let mut declared = NodeType::new(type_id, "Subject");
    let mut definition = PropertyDefinition::new(many, "labels", ValueType::String);
    definition.cardinality = Cardinality::Many;
    declared.properties.insert(many, definition);
    seed.ontology.node_types.push(declared);
    let node = Node::<Value>::new(NodeId::mint(), seed.graph.root.id, type_id, "seed");
    let statement = b"a synthetic statement";
    let hash = ContentHash::of_bytes(statement);
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
    let evidence_id = evidence.id;
    seed.graph.evidence.insert(evidence.id, evidence);
    seed.evidence_payloads
        .insert(hash, statement.to_vec().into());
    seed.graph.nodes.insert(node.id, node.clone());
    Seeded {
        document: seed,
        node: node.id,
        property: many,
        evidence: evidence_id,
    }
}
/// A proposed assertion on the seeded node with the value `label`.
fn assertion(seed: &Seeded, label: &str) -> Assertion<Value> {
    Assertion {
        id: AssertionId::mint(),
        root_id: seed.document.graph.root.id,
        subject: Subject::Node(seed.node),
        predicate: Predicate::Property(seed.property),
        object: Object::Value(Value::String(label.into())),
        evidence: BTreeSet::from([seed.evidence]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::from_millis(0)),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}
/// A transaction adding `assertions`, citing the seeded evidence.
fn adding(seed: &Seeded, assertions: &[Assertion<Value>]) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: assertions
            .iter()
            .map(|a| GraphOperation::AddAssertion(Box::new(a.clone())))
            .collect(),
        evidence: BTreeSet::from([seed.evidence]),
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
/// Propose, validate against the current head and commit, through the real handlers.
fn land(kernel: &Runtime, tx: &GraphTransaction, time: i64) {
    kernel
        .propose(&encode(tx), context().operator, at(time))
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    match kernel.validate(tx.id, head, at(time + 1)).unwrap() {
        ValidationCommandResult::Validated(_) => {}
        ValidationCommandResult::Rejected(record) => panic!("rejected: {:?}", record.issues),
    }
    let CommitCommandResult::Committed(_) = kernel
        .commit(tx.id, context().operator, at(time + 2))
        .unwrap()
    else {
        panic!("a fresh commit became stale")
    };
}
fn bytes(explained: &ExplanationResult) -> usize {
    serde_json::to_vec(explained).unwrap().len()
}

/// The size of one assertion's explanation does not follow the size of the transaction that
/// committed it: 400 other assertions beside it add no bytes to its answer beyond its own.
#[test]
fn an_explanation_does_not_grow_with_its_origin_transaction() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let alone = assertion(&seed, "alone");
        land(&kernel, &adding(&seed, std::slice::from_ref(&alone)), 20);
        let mut crowd: Vec<Assertion<Value>> = (0..400)
            .map(|n| assertion(&seed, &format!("beside {n}")))
            .collect();
        let crowded = assertion(&seed, "crowded");
        crowd.push(crowded.clone());
        land(&kernel, &adding(&seed, &crowd), 30);
        drop(kernel);

        let read = open(directory.path(), file).read(None).unwrap();
        let small = bytes(&read.explain(alone.id).unwrap());
        let large = bytes(&read.explain(crowded.id).unwrap());
        assert!(
            large.abs_diff(small) < 2048,
            "file={file}: alone {small} bytes, among 400 others {large} bytes"
        );
        assert!(large < 16 * 1024, "file={file}: {large} bytes");
    }
}

/// The chain is looked up, not found by reading every committed document: a capture whose other
/// transactions' documents are unreadable explains an assertion exactly as before, and an
/// assertion whose own origin document is unreadable is refused, never answered with a shorter
/// chain.
#[test]
fn explain_reads_only_the_documents_of_its_own_chain() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let first = assertion(&seed, "first");
        let second = assertion(&seed, "second");
        let third = assertion(&seed, "third");
        let first_tx = adding(&seed, std::slice::from_ref(&first));
        let others = [
            adding(&seed, std::slice::from_ref(&second)),
            adding(&seed, std::slice::from_ref(&third)),
        ];
        land(&kernel, &first_tx, 20);
        land(&kernel, &others[0], 30);
        land(&kernel, &others[1], 40);

        let mut read = kernel.read(None).unwrap();
        let before = read.explain(first.id).unwrap();
        let transactions = std::sync::Arc::make_mut(&mut read.transactions);
        for tx in &others {
            let record = transactions.get_mut(&tx.id).unwrap();
            let unreadable = b"\x00 not a transaction document".to_vec();
            record.proposal.document_bytes.clone_from(&unreadable);
            record.committed.as_mut().unwrap().proposal.document_bytes = unreadable;
        }
        assert_eq!(read.explain(first.id).unwrap(), before, "file={file}");
        assert!(
            matches!(
                read.explain(second.id),
                Err(ProjectionError::Unverified { .. })
            ),
            "file={file}: an unreadable chain answered"
        );
    }
}
