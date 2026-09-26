//! Adversary pass 1 on unit p1-13-explain: `VerifiedRead::snapshot` and `VerifiedRead::explain`.
//!
//! The unit's own corruption table (`tests/explain.rs`) models a damaged capture by mutating the
//! public fields of a `VerifiedRead`. These cases use the same model and ask whether a *consistent*
//! forgery — one that keeps every address it names resolvable — is refused, as the module's own
//! claim says ("Every record an explanation names is compared with the retained bytes at its actual
//! payload address inside the same capture"). The remaining cases drive reachable histories the
//! unit's suite does not: a two-step supersession chain read at the head and at the earlier
//! revision, a lifecycle transaction carrying its own evidence, and the JSON shape of the carriers
//! against `systems/ekr/domains/kernel.yaml`.
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
    first: EvidenceId,
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
        first: evidence_ids[0],
        second: evidence_ids[1],
        assertion: id,
    }
}
fn assertion(seed: &Seeded, evidence: EvidenceId, from: i64, label: &str) -> Assertion<Value> {
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
        valid_time: TemporalRange::since(Timestamp::from_millis(from)),
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    }
}
fn transaction(operations: Vec<GraphOperation>, extra: &[EvidenceId]) -> GraphTransaction {
    let mut evidence: BTreeSet<EvidenceId> = operations
        .iter()
        .filter_map(|op| match op {
            GraphOperation::AddAssertion(a) => Some(a.evidence.iter().copied()),
            _ => None,
        })
        .flatten()
        .collect();
    evidence.extend(extra.iter().copied());
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
struct Landed {
    proposal: ProposalRecordV1,
    validation: ValidationReceiptV1,
    receipt: CommitReceiptV1,
}
fn land(kernel: &Runtime, tx: &GraphTransaction, time: i64) -> Landed {
    let proposal = kernel
        .propose(&encode(tx), context().operator, at(time))
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    let validation = match kernel.validate(tx.id, head, at(time + 1)).unwrap() {
        ValidationCommandResult::Validated(receipt) => receipt,
        ValidationCommandResult::Rejected(record) => panic!("rejected: {:?}", record.issues),
    };
    let CommitCommandResult::Committed(receipt) = kernel
        .commit(tx.id, context().operator, at(time + 2))
        .unwrap()
    else {
        panic!("a fresh commit became stale")
    };
    Landed {
        proposal,
        validation,
        receipt: *receipt,
    }
}
fn origin(landed: &Landed) -> Vec<ExplanationLink> {
    vec![
        ExplanationLink::Proposal(landed.proposal.clone()),
        ExplanationLink::Validation(ExplainedValidation {
            receipt: landed.validation.clone(),
            validation_profile: anchor().validation_profile,
            record_hash: ContentHash::of_bytes(&landed.validation.to_bytes().unwrap()),
        }),
        ExplanationLink::Commit(landed.receipt.clone()),
    ]
}
fn seed_link(result: &SeedResultV1) -> ExplanationLink {
    ExplanationLink::Seed(ExplainedSeed {
        result: result.clone(),
        context: context(),
        validation_profile: anchor().validation_profile,
        record_hash: ContentHash::of_bytes(&result.to_bytes().unwrap()),
    })
}
fn lifecycle(id: AssertionId, landed: &Landed, lifecycle: AssertionLifecycle) -> ExplanationLink {
    ExplanationLink::Lifecycle(ExplainedLifecycle {
        assertion_id: id,
        lifecycle,
        receipt: landed.receipt.clone(),
        record_hash: ContentHash::of_bytes(&landed.receipt.to_bytes().unwrap()),
    })
}
fn evidence(read: &VerifiedRead, ids: &[EvidenceId]) -> Vec<ExplanationLink> {
    let ids: BTreeSet<EvidenceId> = ids.iter().copied().collect();
    ids.into_iter()
        .map(|id| ExplanationLink::Evidence(read.graph.evidence[&id].clone()))
        .collect()
}
fn claim(read: &VerifiedRead, id: AssertionId) -> ExplanationLink {
    ExplanationLink::Assertion(read.graph.assertions[&id].clone())
}

/// Seeded A, then B added and A superseded by B in one transaction (revision 1).
fn superseded_once(path: &std::path::Path, file: bool) -> (Seeded, AssertionId, Landed) {
    let seed = fixture();
    let kernel = open(path, file);
    kernel.seed(seed.document.clone(), at(10)).unwrap();
    let replacement = assertion(&seed, seed.second, 100, "replacement");
    let landed = land(
        &kernel,
        &transaction(
            vec![
                GraphOperation::SupersedeAssertion(Supersession {
                    assertion: seed.assertion,
                    by: replacement.id,
                    effective_from: Timestamp::from_millis(100),
                }),
                GraphOperation::AddAssertion(Box::new(replacement.clone())),
            ],
            &[],
        ),
        20,
    );
    (seed, replacement.id, landed)
}

/// One forgery of a captured read that keeps every address it names resolvable.
type Forgery<'a> = Box<dyn Fn(&mut VerifiedRead) -> AssertionId + 'a>;

#[test]
fn a_consistent_forgery_of_the_captured_graph_is_refused_by_explain() {
    let mut admitted = Vec::new();
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (seed, replacement, _) = superseded_once(directory.path(), file);
        let kernel = open(directory.path(), file);
        let second_hash = seed.document.graph.evidence[&seed.second].content_hash;
        let forgeries: Vec<(&str, Forgery<'_>)> = vec![
            (
                "evidence record redirected to another retained payload (recomputed address)",
                Box::new(|r| {
                    r.graph.evidence.get_mut(&seed.first).unwrap().content_hash = second_hash;
                    seed.assertion
                }),
            ),
            (
                "evidence source identity rewritten",
                Box::new(|r| {
                    r.graph.evidence.get_mut(&seed.second).unwrap().source =
                        EvidenceSource::HumanStatement {
                            identity: Some("someone else".into()),
                        };
                    replacement
                }),
            ),
            (
                "committed assertion's object rewritten",
                Box::new(|r| {
                    r.graph.assertions.get_mut(&replacement).unwrap().object =
                        Object::Value(CanonicalValue::String("forged".into()));
                    replacement
                }),
            ),
            (
                "fabricated assertion claimed as seeded",
                Box::new(|r| {
                    let mut forged = r.graph.assertions[&seed.assertion].clone();
                    forged.id = AssertionId::mint();
                    forged.lifecycle = AssertionLifecycle::Active;
                    forged.object = Object::Value(CanonicalValue::String("fabricated".into()));
                    let mut declared = r.seed_input.graph.assertions[&seed.assertion].clone();
                    let id = forged.id;
                    declared.id = id;
                    r.graph.assertions.insert(id, forged);
                    r.seed_input.graph.assertions.insert(id, declared);
                    id
                }),
            ),
        ];
        for (name, forge) in &forgeries {
            let mut read = kernel.read(None).unwrap();
            let target = forge(&mut read);
            // The forgery is detectable from the capture alone: its own root no longer commits
            // to the graph it is explaining.
            let detectable = ekr_store::knowledge_root(&read.graph) != read.root.knowledge_root
                || ekr_store::evidence_root(&read.graph) != read.root.evidence_root;
            assert!(
                detectable,
                "file={file}/{name}: forgery left both roots intact"
            );
            if let Ok(result) = read.explain(target) {
                admitted.push(format!(
                    "file={file}/{name}: Ok with {} links",
                    result.links.len()
                ));
            }
        }
    }
    assert!(
        admitted.is_empty(),
        "forged chains returned as Ok: {admitted:#?}"
    );
}

#[test]
fn a_snapshot_never_pairs_a_root_with_a_graph_it_does_not_commit_to() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (_, replacement, _) = superseded_once(directory.path(), file);
        let mut read = open(directory.path(), file).read(None).unwrap();
        read.graph.assertions.get_mut(&replacement).unwrap().object =
            Object::Value(CanonicalValue::String("forged".into()));
        match read.snapshot(None) {
            Err(_) => {}
            Ok(result) => {
                let graph_root = ekr_store::knowledge_root(&read.graph);
                assert_eq!(
                    result.root.knowledge_root, graph_root,
                    "file={file}: SnapshotResult claims a root whose knowledge_root does not \
                     commit to the graph it carries"
                );
            }
        }
    }
}

#[test]
fn a_supersession_chain_is_followed_at_the_head_and_stops_at_the_earlier_revision() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        let seeded = kernel.seed(seed.document.clone(), at(10)).unwrap();
        let b = assertion(&seed, seed.second, 100, "b");
        let one = land(
            &kernel,
            &transaction(
                vec![
                    GraphOperation::SupersedeAssertion(Supersession {
                        assertion: seed.assertion,
                        by: b.id,
                        effective_from: Timestamp::from_millis(100),
                    }),
                    GraphOperation::AddAssertion(Box::new(b.clone())),
                ],
                &[],
            ),
            20,
        );
        let c = assertion(&seed, seed.first, 200, "c");
        let two = land(
            &kernel,
            &transaction(
                vec![
                    GraphOperation::SupersedeAssertion(Supersession {
                        assertion: b.id,
                        by: c.id,
                        effective_from: Timestamp::from_millis(200),
                    }),
                    GraphOperation::AddAssertion(Box::new(c.clone())),
                ],
                &[],
            ),
            30,
        );
        drop(kernel);
        let kernel = open(directory.path(), file);

        let head = kernel.read(None).unwrap();
        let explained = head.explain(seed.assertion).unwrap();
        let mut expected = vec![
            claim(&head, seed.assertion),
            seed_link(&seeded),
            lifecycle(
                seed.assertion,
                &one,
                AssertionLifecycle::Superseded {
                    by: CanonicalRef::new(b.id),
                    at_revision: RevisionNumber::new(1),
                    effective_from: Timestamp::from_millis(100),
                },
            ),
            claim(&head, b.id),
        ];
        expected.extend(origin(&one));
        expected.push(lifecycle(
            b.id,
            &two,
            AssertionLifecycle::Superseded {
                by: CanonicalRef::new(c.id),
                at_revision: RevisionNumber::new(2),
                effective_from: Timestamp::from_millis(200),
            },
        ));
        expected.push(claim(&head, c.id));
        expected.extend(origin(&two));
        expected.extend(evidence(&head, &[seed.first, seed.second]));
        assert_eq!(explained.links, expected, "file={file} head");

        // At revision 1 the replacement B is still active and C does not exist yet.
        let earlier = kernel.read(Some(RevisionNumber::new(1))).unwrap();
        let before = earlier.explain(seed.assertion).unwrap();
        assert_eq!(before.at, RevisionNumber::new(1));
        let mut expected = vec![
            claim(&earlier, seed.assertion),
            seed_link(&seeded),
            lifecycle(
                seed.assertion,
                &one,
                AssertionLifecycle::Superseded {
                    by: CanonicalRef::new(b.id),
                    at_revision: RevisionNumber::new(1),
                    effective_from: Timestamp::from_millis(100),
                },
            ),
            claim(&earlier, b.id),
        ];
        expected.extend(origin(&one));
        expected.extend(evidence(&earlier, &[seed.first, seed.second]));
        assert_eq!(before.links, expected, "file={file} revision 1");
        assert_eq!(
            earlier.explain(c.id),
            Err(ProjectionError::AssertionNotFound { requested: c.id })
        );
    }
}

#[test]
fn a_lifecycle_transaction_contributes_its_own_evidence_set() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        let seeded = kernel.seed(seed.document.clone(), at(10)).unwrap();
        let reason = RetractionReason::new("withdrawn on a second statement");
        let retracted = land(
            &kernel,
            // The validator holds a transaction's evidence set to what its assertions cite, so
            // the retraction carries its own support by adding a sibling assertion that cites it.
            &transaction(
                vec![
                    GraphOperation::RetractAssertion(Retraction {
                        assertion: seed.assertion,
                        reason: reason.clone(),
                    }),
                    GraphOperation::AddAssertion(Box::new(assertion(
                        &seed,
                        seed.second,
                        0,
                        "sibling",
                    ))),
                ],
                &[],
            ),
            20,
        );
        let read = open(directory.path(), file).read(None).unwrap();
        let explained = read.explain(seed.assertion).unwrap();
        let mut expected = vec![
            claim(&read, seed.assertion),
            seed_link(&seeded),
            lifecycle(
                seed.assertion,
                &retracted,
                AssertionLifecycle::Retracted {
                    at_revision: RevisionNumber::new(1),
                    reason,
                },
            ),
        ];
        expected.extend(evidence(&read, &[seed.first, seed.second]));
        assert_eq!(explained.links, expected, "file={file}");
    }
}

fn keys(value: &serde_json::Value) -> BTreeSet<String> {
    value
        .as_object()
        .expect("an object")
        .keys()
        .cloned()
        .collect()
}
fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn the_carriers_serialise_with_the_names_and_kind_tags_kernel_yaml_declares() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (seed, _, _) = superseded_once(directory.path(), file);
        let read = open(directory.path(), file).read(None).unwrap();

        let snapshot =
            serde_json::to_value(read.snapshot(Some(Timestamp::from_millis(5))).unwrap()).unwrap();
        assert_eq!(
            keys(&snapshot),
            set(&[
                "revision_id",
                "root",
                "graph",
                "valid_at",
                "matching_assertions"
            ])
        );

        let explained = serde_json::to_value(read.explain(seed.assertion).unwrap()).unwrap();
        assert_eq!(keys(&explained), set(&["assertion_id", "at", "links"]));
        let links = explained["links"].as_array().unwrap();
        let tags: Vec<&str> = links.iter().map(|l| l["kind"].as_str().unwrap()).collect();
        assert_eq!(
            tags,
            [
                "Assertion",
                "Seed",
                "Lifecycle",
                "Assertion",
                "Proposal",
                "Validation",
                "Commit",
                "Evidence",
                "Evidence"
            ],
            "file={file}"
        );
        let shape = |kind: &str| links.iter().find(|l| l["kind"] == kind).map(keys).unwrap();
        assert_eq!(
            shape("Seed"),
            set(&[
                "kind",
                "result",
                "context",
                "validation_profile",
                "record_hash"
            ])
        );
        assert_eq!(
            shape("Validation"),
            set(&["kind", "receipt", "validation_profile", "record_hash"])
        );
        assert_eq!(
            shape("Lifecycle"),
            set(&[
                "kind",
                "assertion_id",
                "lifecycle",
                "receipt",
                "record_hash"
            ])
        );
        // A newtype variant's own fields sit beside the tag; none of them may be called `kind`.
        let assertion_fields =
            serde_json::to_value(&read.graph.assertions[&seed.assertion]).unwrap();
        let mut with_tag = keys(&assertion_fields);
        with_tag.insert("kind".into());
        assert_eq!(shape("Assertion"), with_tag);
    }
}
