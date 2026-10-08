//! Adversary pass on `story:explain-reads-an-index` (wave extract-06, unit X, commit `4dc51319`).
//!
//! `explain` now finds an assertion's origin among the commits whose receipt instant equals the
//! assertion's `transaction_time.recorded_from`, and its lifecycle change at the commit of the
//! lifecycle's `at_revision`, through an index keyed by revision. The red cases here hold a
//! deliberately altered capture to the refusal the base commit (`40e11f625`) gave for it; the
//! green cases are the ordering and identity shapes the brief named, recorded as attacked.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{Cardinality, NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::BTreeSet;
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
fn at(millis: i64) -> impl FnOnce() -> Timestamp {
    move || Timestamp::from_millis(millis)
}

/// One node, one HumanStatement evidence, one seeded assertion citing it.
struct Seeded {
    document: SeedDocument,
    node: NodeId,
    property: PropertyId,
    evidence: EvidenceId,
    assertion: AssertionId,
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
    let statement = b"an adversary statement";
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
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id: seed.graph.root.id,
        subject: Subject::Node(node.id),
        predicate: Predicate::Property(many),
        object: Object::Value(Value::String("seed".into())),
        evidence: BTreeSet::from([evidence_id]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::from_millis(0)),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    let id = assertion.id;
    seed.graph.nodes.insert(node.id, node.clone());
    seed.graph.assertions.insert(assertion.id, assertion);
    Seeded {
        document: seed,
        node: node.id,
        property: many,
        evidence: evidence_id,
        assertion: id,
    }
}
fn assertion(seed: &Seeded, label: &str, from: i64) -> Assertion<Value> {
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
        valid_time: TemporalRange::since(Timestamp::from_millis(from)),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}
/// A transaction declaring exactly the evidence its added assertions cite.
fn transaction(_seed: &Seeded, operations: Vec<GraphOperation>) -> GraphTransaction {
    let evidence = operations
        .iter()
        .filter_map(|op| match op {
            GraphOperation::AddAssertion(a) => Some(a.evidence.iter().copied()),
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
fn add(a: &Assertion<Value>) -> GraphOperation {
    GraphOperation::AddAssertion(Box::new(a.clone()))
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
/// Propose at `time`, validate at `time + 1`, commit at `time + 2`, through the real handlers.
fn land(kernel: &Runtime, tx: &GraphTransaction, time: i64) {
    land_at(kernel, tx, [time, time + 1, time + 2]);
}
/// Propose, validate and commit at the three given instants.
fn land_at(kernel: &Runtime, tx: &GraphTransaction, [proposed, validated, committed]: [i64; 3]) {
    kernel
        .propose(&encode(tx), context().operator, at(proposed))
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    match kernel.validate(tx.id, head, at(validated)).unwrap() {
        ValidationCommandResult::Validated(_) => {}
        ValidationCommandResult::Rejected(record) => panic!("rejected: {:?}", record.issues),
    }
    let CommitCommandResult::Committed(_) = kernel
        .commit(tx.id, context().operator, at(committed))
        .unwrap()
    else {
        panic!("a fresh commit became stale")
    };
}
fn code(result: Result<ExplanationResult, ProjectionError>) -> String {
    match result {
        Ok(explained) => format!("answered with {} links", explained.links.len()),
        Err(ProjectionError::Unverified { code }) => code,
        Err(other) => format!("{other:?}"),
    }
}
fn kinds(explained: &ExplanationResult) -> Vec<&'static str> {
    explained
        .links
        .iter()
        .map(|link| match link {
            ExplanationLink::Assertion(_) => "Assertion",
            ExplanationLink::Seed(_) => "Seed",
            ExplanationLink::Proposal(_) => "Proposal",
            ExplanationLink::Validation(_) => "Validation",
            ExplanationLink::Commit(_) => "Commit",
            ExplanationLink::Lifecycle(_) => "Lifecycle",
            ExplanationLink::Evidence(_) => "Evidence",
            ExplanationLink::Attachment(_) => "Attachment",
        })
        .collect()
}
/// The transactions the chain's Commit and Lifecycle links name, in chain order.
fn commits(explained: &ExplanationResult) -> Vec<TransactionId> {
    explained
        .links
        .iter()
        .filter_map(|link| match link {
            ExplanationLink::Commit(c) => Some(c.transaction_id),
            ExplanationLink::Lifecycle(l) => Some(l.commit.transaction_id),
            _ => None,
        })
        .collect()
}

// ---- Refusals the base gave for an altered capture ---------------------------------------------

/// An origin commit receipt whose `committed_at` was altered in the capture no longer agrees with
/// the retained receipt at its revision coordinate. Base found the origin by its document and
/// refused it in `verify` as `commit-record-disagrees`. Unit X first looked the origin up by that
/// altered instant and named it `origin-missing`; it now matches the origin by revision, through
/// the verified coordinate's instant, and `verify` refuses the receipt as base did.
#[test]
fn an_origin_receipt_with_an_altered_instant_is_refused_as_base_refused_it() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let x = assertion(&seed, "x", 0);
        let tx = transaction(&seed, vec![add(&x)]);
        land(&kernel, &tx, 20);

        let mut read = kernel.read(None).unwrap();
        assert!(read.explain(x.id).is_ok(), "control file={file}");
        let held = Arc::make_mut(&mut read.transactions)
            .get_mut(&tx.id)
            .unwrap();
        held.committed.as_mut().unwrap().committed_at = Timestamp::from_millis(23);
        let got = code(read.explain(x.id));
        if got != "commit-record-disagrees" {
            wrong.push(format!("file={file}: {got}"));
        }
    }
    assert!(
        wrong.is_empty(),
        "base refused commit-record-disagrees: {wrong:?}"
    );
}

/// A second retained record of the very same commit, under another transaction key. Base saw two
/// commits adding the assertion and refused `origin-ambiguous`. Unit X's revision-keyed index first
/// kept one of them and answered; two records claiming one revision are now `origin-ambiguous`,
/// before any document is read.
#[test]
fn a_duplicated_origin_record_is_refused_as_ambiguous() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let x = assertion(&seed, "x", 0);
        let tx = transaction(&seed, vec![add(&x)]);
        land(&kernel, &tx, 20);

        let mut read = kernel.read(None).unwrap();
        assert!(read.explain(x.id).is_ok(), "control file={file}");
        let transactions = Arc::make_mut(&mut read.transactions);
        let copy = transactions[&tx.id].clone();
        transactions.insert(TransactionId::mint(), copy);
        let got = code(read.explain(x.id));
        if got != "origin-ambiguous" {
            wrong.push(format!("file={file}: {got}"));
        }
    }
    assert!(wrong.is_empty(), "base refused origin-ambiguous: {wrong:?}");
}

/// A forged record, at another revision and instant, whose committed document also adds the
/// assertion. Base refused `origin-ambiguous`. The coordinator decided (adversary x6-x, finding 4)
/// that explain keeps the narrower reading documented in `systems/ekr/domains/kernel.yaml`,
/// `ekr.kernel.ExplanationResult`, "What explain trusts", and in `VerifiedRead::explain`'s
/// "What explain trusts": the record is on no chain, is not read, and the answer is the chain of
/// verified links, exactly as before the forgery.
#[test]
fn a_forged_second_origin_at_another_instant_is_not_read() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let x = assertion(&seed, "x", 0);
        let tx = transaction(&seed, vec![add(&x)]);
        land(&kernel, &tx, 20);

        let mut read = kernel.read(None).unwrap();
        let control = read.explain(x.id).unwrap();
        let transactions = Arc::make_mut(&mut read.transactions);
        let mut forged = transactions[&tx.id].clone();
        let receipt = forged.committed.as_mut().unwrap();
        receipt.committed_at = Timestamp::from_millis(99);
        receipt.result.revision = RevisionNumber::SEED;
        transactions.insert(TransactionId::mint(), forged);
        match read.explain(x.id) {
            Ok(answered) if answered == control => {}
            other => wrong.push(format!("file={file}: {other:?}")),
        }
    }
    assert!(
        wrong.is_empty(),
        "the forged record changed the verified chain: {wrong:?}"
    );
}

/// A forged record whose committed document retracts an assertion the verified graph holds
/// active. Base read every committed document's lifecycle operations, met this one and refused it
/// in `verify` (`proposal-record-disagrees`). The coordinator decided (adversary x6-x, finding 5)
/// that explain keeps the narrower reading documented in `systems/ekr/domains/kernel.yaml`,
/// `ekr.kernel.ExplanationResult`, "What explain trusts", and in `VerifiedRead::explain`'s
/// "What explain trusts": a lifecycle is read only at the graph's own `at_revision`, so the
/// active assertion is answered from its verified links, exactly as before the forgery.
#[test]
fn a_forged_retraction_of_an_active_assertion_is_not_read() {
    let mut wrong = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let x = assertion(&seed, "x", 0);
        let tx = transaction(&seed, vec![add(&x)]);
        land(&kernel, &tx, 20);

        let mut read = kernel.read(None).unwrap();
        let control = read.explain(x.id).unwrap();
        let retraction = transaction(
            &seed,
            vec![GraphOperation::RetractAssertion(Retraction {
                assertion: x.id,
                reason: RetractionReason::new("forged"),
            })],
        );
        let document = encode(&retraction);
        let transactions = Arc::make_mut(&mut read.transactions);
        let mut forged = transactions[&tx.id].clone();
        forged.proposal.document_bytes.clone_from(&document);
        let receipt = forged.committed.as_mut().unwrap();
        receipt.proposal.document_bytes = document;
        receipt.committed_at = Timestamp::from_millis(99);
        receipt.result.revision = RevisionNumber::SEED;
        transactions.insert(retraction.id, forged);
        match read.explain(x.id) {
            Ok(answered) if answered == control => {}
            other => wrong.push(format!("file={file}: {other:?}")),
        }
    }
    assert!(
        wrong.is_empty(),
        "the forged record changed the verified chain: {wrong:?}"
    );
}

// ---- Shapes the brief named, attacked on real history ------------------------------------------

/// Two commits at one instant, the second proposed after the first committed: each assertion is
/// explained from its own commit.
#[test]
fn two_commits_at_one_instant_each_explain_from_their_own_commit() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let x = assertion(&seed, "x", 0);
        let y = assertion(&seed, "y", 0);
        let tx = transaction(&seed, vec![add(&x)]);
        let ty = transaction(&seed, vec![add(&y)]);
        land_at(&kernel, &tx, [20, 21, 22]);
        land_at(&kernel, &ty, [22, 22, 22]);
        let read = kernel.read(None).unwrap();
        assert_eq!(
            read.graph.assertions[&x.id].transaction_time.recorded_from,
            read.graph.assertions[&y.id].transaction_time.recorded_from,
            "file={file}: the two commits are not at one instant"
        );
        assert_eq!(
            commits(&read.explain(x.id).unwrap()),
            vec![tx.id],
            "file={file}"
        );
        assert_eq!(
            commits(&read.explain(y.id).unwrap()),
            vec![ty.id],
            "file={file}"
        );
    }
}

/// A clock that went backwards never reaches the instant index: a validation earlier than the
/// head's commit is refused (`validation-time-order`), so commit instants never decrease along
/// revisions and only equal instants can share an origin lookup.
#[test]
fn a_clock_that_goes_backwards_is_refused_before_it_reaches_the_index() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let x = assertion(&seed, "x", 0);
        let y = assertion(&seed, "y", 0);
        let tx = transaction(&seed, vec![add(&x)]);
        let ty = transaction(&seed, vec![add(&y)]);
        land(&kernel, &tx, 40);
        kernel
            .propose(&encode(&ty), context().operator, at(20))
            .unwrap();
        let head = kernel.head().unwrap().unwrap().revision;
        let refused = kernel.validate(ty.id, head, at(21));
        assert!(
            format!("{refused:?}").contains("validation-time-order"),
            "file={file}: {refused:?}"
        );
    }
}

/// A commit at the seed's own instant: the seeded assertion still explains from the seed, and the
/// committed one from its commit.
#[test]
fn a_commit_at_the_seed_instant_does_not_take_the_seed_origin() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(12)).unwrap();
        let x = assertion(&seed, "x", 0);
        let tx = transaction(&seed, vec![add(&x)]);
        land_at(&kernel, &tx, [12, 12, 12]);
        assert_eq!(
            kernel.read(None).unwrap().graph.assertions[&seed.assertion]
                .transaction_time
                .recorded_from,
            Timestamp::from_millis(12),
            "file={file}: the commit is not at the seed instant"
        );
        let read = kernel.read(None).unwrap();
        let seeded = read.explain(seed.assertion).unwrap();
        assert_eq!(
            kinds(&seeded),
            ["Assertion", "Seed", "Evidence"],
            "file={file}"
        );
        assert_eq!(
            commits(&read.explain(x.id).unwrap()),
            vec![tx.id],
            "file={file}"
        );
    }
}

/// Retracted, then the same claim asserted again under a new id: each is explained from its own
/// commits, the first with its retraction.
#[test]
fn a_claim_restated_after_its_retraction_explains_both_assertions_apart() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let x = assertion(&seed, "x", 0);
        let again = assertion(&seed, "x", 0);
        let t1 = transaction(&seed, vec![add(&x)]);
        let t2 = transaction(
            &seed,
            vec![GraphOperation::RetractAssertion(Retraction {
                assertion: x.id,
                reason: RetractionReason::new("withdrawn"),
            })],
        );
        let t3 = transaction(&seed, vec![add(&again)]);
        land(&kernel, &t1, 20);
        land(&kernel, &t2, 30);
        land(&kernel, &t3, 40);
        drop(kernel);
        let read = open(directory.path(), file).read(None).unwrap();
        let first = read.explain(x.id).unwrap();
        assert_eq!(
            kinds(&first),
            [
                "Assertion",
                "Proposal",
                "Validation",
                "Commit",
                "Lifecycle",
                "Evidence"
            ],
            "file={file}"
        );
        assert_eq!(commits(&first), vec![t1.id, t2.id], "file={file}");
        assert_eq!(
            commits(&read.explain(again.id).unwrap()),
            vec![t3.id],
            "file={file}"
        );
    }
}

/// Seeded A superseded by B (revision 1), B superseded by C (revision 2): explaining A walks the
/// whole chain, every step from its own commit.
#[test]
fn a_supersession_chain_of_three_is_walked_from_each_steps_own_commit() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let b = assertion(&seed, "b", 100);
        let c = assertion(&seed, "c", 200);
        let t1 = transaction(
            &seed,
            vec![
                GraphOperation::SupersedeAssertion(Supersession {
                    assertion: seed.assertion,
                    by: b.id,
                    effective_from: Timestamp::from_millis(100),
                }),
                add(&b),
            ],
        );
        let t2 = transaction(
            &seed,
            vec![
                GraphOperation::SupersedeAssertion(Supersession {
                    assertion: b.id,
                    by: c.id,
                    effective_from: Timestamp::from_millis(200),
                }),
                add(&c),
            ],
        );
        land(&kernel, &t1, 20);
        land(&kernel, &t2, 30);
        drop(kernel);
        let read = open(directory.path(), file).read(None).unwrap();
        let explained = read.explain(seed.assertion).unwrap();
        // A's links, then the replacements in stable id order: b was minted before c.
        assert_eq!(
            kinds(&explained),
            [
                "Assertion",
                "Seed",
                "Lifecycle",
                "Assertion",
                "Proposal",
                "Validation",
                "Commit",
                "Lifecycle",
                "Assertion",
                "Proposal",
                "Validation",
                "Commit",
                "Evidence"
            ],
            "file={file}"
        );
        assert_eq!(
            commits(&explained),
            vec![t1.id, t1.id, t2.id, t2.id],
            "file={file}"
        );
        // At revision 1 the chain stops at b.
        let earlier = open(directory.path(), file)
            .read(Some(RevisionNumber::new(1)))
            .unwrap();
        assert_eq!(
            commits(&earlier.explain(seed.assertion).unwrap()),
            vec![t1.id, t1.id],
            "file={file}"
        );
    }
}
