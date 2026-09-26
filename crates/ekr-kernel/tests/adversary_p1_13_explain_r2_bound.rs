//! Adversary pass 2 on unit p1-13-explain: the private `VerifiedRead::bound()` correction.
//!
//! Three questions, each on both providers:
//! 1. does a forgery that satisfies every `bound()` check still change what a projection returns;
//! 2. does `bound()` refuse a genuine capture (seed-only lineage, earlier revisions after a
//!    supersession, a fresh-process reopen);
//! 3. is each named `bound()` check executed by some case — a forgery that only that check stops.
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

struct Seeded {
    document: SeedDocument,
    node: NodeId,
    property: PropertyId,
    second: EvidenceId,
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
    let mut evidence_ids = Vec::new();
    for statement in [
        &b"first synthetic statement"[..],
        b"second synthetic statement",
    ] {
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
        evidence_ids.push(evidence.id);
        seed.graph.evidence.insert(evidence.id, evidence);
        seed.evidence_payloads.insert(hash, statement.to_vec());
    }
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id: seed.graph.root.id,
        subject: Subject::Node(node.id),
        predicate: Predicate::Property(many),
        object: Object::Value(Value::String("seed".into())),
        evidence: BTreeSet::from([evidence_ids[0]]),
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
        second: evidence_ids[1],
        assertion: id,
    }
}
fn assertion(seed: &Seeded, evidence: EvidenceId, from: i64) -> Assertion<Value> {
    Assertion {
        id: AssertionId::mint(),
        root_id: seed.document.graph.root.id,
        subject: Subject::Node(seed.node),
        predicate: Predicate::Property(seed.property),
        object: Object::Value(Value::String("replacement".into())),
        evidence: BTreeSet::from([evidence]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::since(Timestamp::from_millis(from)),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}
fn transaction(operations: Vec<GraphOperation>) -> GraphTransaction {
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
fn land(kernel: &Runtime, tx: &GraphTransaction, time: i64) -> CommitReceiptV1 {
    kernel
        .propose(&encode(tx), context().operator, at(time))
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    match kernel.validate(tx.id, head, at(time + 1)).unwrap() {
        ValidationCommandResult::Validated(_) => {}
        ValidationCommandResult::Rejected(record) => panic!("rejected: {:?}", record.issues),
    }
    let CommitCommandResult::Committed(receipt) = kernel
        .commit(tx.id, context().operator, at(time + 2))
        .unwrap()
    else {
        panic!("a fresh commit became stale")
    };
    *receipt
}
fn kinds(result: &ExplanationResult) -> Vec<&'static str> {
    result
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
        })
        .collect()
}

/// Seeded A; B accepted at revision 1; A superseded by B at revision 2.
struct Replaced {
    seed: Seeded,
    seeded: SeedResultV1,
    replacement: AssertionId,
    accepted: CommitReceiptV1,
}
fn replaced_later(path: &std::path::Path, file: bool) -> Replaced {
    let seed = fixture();
    let kernel = open(path, file);
    let seeded = kernel.seed(seed.document.clone(), at(10)).unwrap();
    let replacement = assertion(&seed, seed.second, 100);
    let accepted = land(
        &kernel,
        &transaction(vec![GraphOperation::AddAssertion(Box::new(
            replacement.clone(),
        ))]),
        20,
    );
    land(
        &kernel,
        &transaction(vec![GraphOperation::SupersedeAssertion(Supersession {
            assertion: seed.assertion,
            by: replacement.id,
            effective_from: Timestamp::from_millis(100),
        })]),
        30,
    );
    Replaced {
        seed,
        seeded,
        replacement: replacement.id,
        accepted,
    }
}

fn code(outcome: Result<impl std::fmt::Debug, ProjectionError>) -> String {
    match outcome {
        Err(ProjectionError::Unverified { code }) => code,
        other => format!("not refused: {other:?}"),
    }
}

/// `bound()` binds the graph through its knowledge, evidence and ontology roots, but
/// `CanonicalGraph.root` (the `GraphRoot`: id, space, schema version, parent, creation time) is
/// in none of the three. `Snapshot` emits it verbatim as `graph.root`, so a capture whose graph
/// root is rewritten passes every check and returns a graph document that is not the retained one.
#[test]
fn a_rewritten_graph_root_is_refused_by_snapshot() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let case = replaced_later(directory.path(), file);
        let kernel = open(directory.path(), file);
        let genuine = kernel.read(None).unwrap();
        // The retained value the projection could bind it to.
        assert_eq!(genuine.graph.root, genuine.seed_input.graph.root);

        let mut forged = kernel.read(None).unwrap();
        forged.graph.root.created_at = Timestamp::from_millis(987_654_321);
        forged.graph.root.parent = Some(GraphRootId::mint());
        let outcome = forged.snapshot(None);
        if let Ok(result) = &outcome {
            assert_ne!(result.graph.root, genuine.graph.root);
        }
        assert!(
            outcome.is_err(),
            "file={file}: snapshot returned a forged graph root as the retained graph: {:?}",
            outcome.map(|r| r.graph.root)
        );
        let _ = case;
    }
}

/// `root-record-disagrees` is the only check that stops a root forged together with its head
/// coordinate: the sub-roots still match the graph, the seed is untouched. Without it Snapshot
/// reports a root (and, for the second fault, a revision id) no retained record names.
#[test]
fn a_root_forged_with_its_coordinate_is_refused_by_the_retained_record() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let case = replaced_later(directory.path(), file);
        let kernel = open(directory.path(), file);
        let old = case.seed.assertion;
        let other_revision = case.accepted.revision_id;
        let faults: Vec<(&str, CaptureFault)> = vec![
            (
                "root.transaction and head coordinate root",
                Box::new(|r| {
                    r.root.transaction = ContentHash::of_bytes(b"forged transaction");
                    let head = r.root;
                    r.revisions.values_mut().last().unwrap().root = head;
                }),
            ),
            (
                "head coordinate revision id",
                Box::new(move |r| {
                    r.revisions.values_mut().last().unwrap().revision_id = other_revision;
                }),
            ),
        ];
        let mut admitted = Vec::new();
        for (name, fault) in &faults {
            let mut read = kernel.read(None).unwrap();
            fault(&mut read);
            for (projection, held) in [
                ("explain", code(read.explain(old))),
                ("snapshot", code(read.snapshot(None))),
            ] {
                if held != "root-record-disagrees" {
                    admitted.push(format!("file={file}/{projection}/{name}: {held}"));
                }
            }
        }
        assert!(admitted.is_empty(), "{admitted:#?}");
    }
}

/// `bound()`'s own `seed-record-disagrees`: a forged seed result whose `seed_hash` still names
/// the retained envelope. Explain of a *committed* assertion never reaches `explained_seed`, and
/// Snapshot has no other seed check, so this check alone stops both.
#[test]
fn a_forged_seed_result_is_refused_by_both_projections() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let case = replaced_later(directory.path(), file);
        let kernel = open(directory.path(), file);
        let other_revision = case.accepted.revision_id;
        let faults: Vec<(&str, CaptureFault)> = vec![
            (
                "seed revision id",
                Box::new(move |r| r.seed.revision_id = other_revision),
            ),
            (
                "seed committed_at",
                Box::new(|r| r.seed.committed_at = Timestamp::from_millis(55_555)),
            ),
        ];
        let mut admitted = Vec::new();
        for (name, fault) in &faults {
            let mut read = kernel.read(None).unwrap();
            fault(&mut read);
            for (projection, held) in [
                ("explain", code(read.explain(case.replacement))),
                ("snapshot", code(read.snapshot(None))),
            ] {
                if held != "seed-record-disagrees" {
                    admitted.push(format!("file={file}/{projection}/{name}: {held}"));
                }
            }
        }
        assert!(admitted.is_empty(), "{admitted:#?}");
    }
}

/// Both `revision-coordinate-missing` sites in `bound()`: no coordinates at all, and a head
/// coordinate without the seed coordinate.
#[test]
fn missing_revision_coordinates_are_refused_by_both_projections() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let case = replaced_later(directory.path(), file);
        let kernel = open(directory.path(), file);
        let faults: Vec<(&str, CaptureFault)> = vec![
            ("no coordinates", Box::new(|r| r.revisions.clear())),
            (
                "seed coordinate dropped",
                Box::new(|r| {
                    r.revisions.remove(&RevisionNumber::SEED);
                }),
            ),
        ];
        let mut admitted = Vec::new();
        for (name, fault) in &faults {
            let mut read = kernel.read(None).unwrap();
            fault(&mut read);
            for (projection, held) in [
                ("explain", code(read.explain(case.replacement))),
                ("snapshot", code(read.snapshot(None))),
            ] {
                if held != "revision-coordinate-missing" {
                    admitted.push(format!("file={file}/{projection}/{name}: {held}"));
                }
            }
        }
        assert!(admitted.is_empty(), "{admitted:#?}");
    }
}

/// A seed-only lineage, read in a fresh process: `bound()` must admit revision zero, where the
/// head coordinate and the seed coordinate are the same record.
#[test]
fn a_seed_only_lineage_is_admitted_across_a_reopen() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let seeded = open(directory.path(), file)
            .seed(seed.document.clone(), at(10))
            .unwrap();
        let read = open(directory.path(), file).read(None).unwrap();
        let snapshot = read.snapshot(Some(Timestamp::from_millis(5))).unwrap();
        assert_eq!(snapshot.root, seeded.result, "file={file}");
        assert_eq!(snapshot.revision_id, seeded.revision_id);
        assert_eq!(snapshot.matching_assertions, Some(vec![seed.assertion]));
        let explained = read.explain(seed.assertion).unwrap();
        assert_eq!(explained.at, RevisionNumber::SEED);
        assert_eq!(kinds(&explained), ["Assertion", "Seed", "Evidence"]);
    }
}

/// After a supersession, every earlier revision is still a genuine capture `bound()` admits,
/// and each explains only what it had seen.
#[test]
fn earlier_revisions_after_a_supersession_are_admitted_and_bounded() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let case = replaced_later(directory.path(), file);
        let kernel = open(directory.path(), file);
        let old = case.seed.assertion;

        let zero = kernel.read(Some(RevisionNumber::SEED)).unwrap();
        let snapshot = zero.snapshot(None).unwrap();
        assert_eq!(snapshot.root, case.seeded.result, "file={file}");
        assert_eq!(snapshot.revision_id, case.seeded.revision_id);
        assert_eq!(
            kinds(&zero.explain(old).unwrap()),
            ["Assertion", "Seed", "Evidence"]
        );
        assert_eq!(
            zero.explain(case.replacement).err(),
            Some(ProjectionError::AssertionNotFound {
                requested: case.replacement
            })
        );

        let one = kernel.read(Some(RevisionNumber::new(1))).unwrap();
        assert_eq!(one.snapshot(None).unwrap().root, case.accepted.result);
        assert_eq!(
            kinds(&one.explain(old).unwrap()),
            ["Assertion", "Seed", "Evidence"],
            "file={file}: revision 1 must not show the revision-2 supersession"
        );
        assert_eq!(
            kinds(&one.explain(case.replacement).unwrap()),
            ["Assertion", "Proposal", "Validation", "Commit", "Evidence"]
        );
    }
}

/// One deliberate corruption of a captured read.
type CaptureFault = Box<dyn Fn(&mut VerifiedRead)>;
