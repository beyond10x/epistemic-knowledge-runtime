//! Snapshot and Explain projections over one kernel-owned `VerifiedRead`, on both providers,
//! across a reopen: `story:seed-and-explain` and `.engineering/waves/p1-cli-explain-contract-r2.md`.
//!
//! Every chain here is read from real retained history written by the durable handlers; the
//! expected links are the records those handlers returned, not a reconstruction.
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

/// The seed: one node, two distinct HumanStatement evidences, one seeded assertion citing the
/// first. The assertion's valid time starts at 0 so a valid-time selection can come back empty.
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
/// A proposed assertion on the seeded node, citing `evidence`, valid from `from`.
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

/// What the three durable handlers returned for one committed transaction.
struct Landed {
    proposal: ProposalRecordV1,
    validation: ValidationReceiptV1,
    receipt: CommitReceiptV1,
}
/// Propose, validate against the current head and commit, all through the real handlers.
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
fn record_hash(bytes: &[u8]) -> ContentHash {
    ContentHash::of_bytes(bytes)
}
/// The origin links of one ordinary commit, exactly as the handlers returned them.
fn origin(landed: &Landed) -> Vec<ExplanationLink> {
    vec![
        ExplanationLink::Proposal(landed.proposal.clone()),
        ExplanationLink::Validation(ExplainedValidation {
            receipt: landed.validation.clone(),
            validation_profile: anchor().validation_profile,
            record_hash: record_hash(&landed.validation.to_bytes().unwrap()),
        }),
        ExplanationLink::Commit(landed.receipt.clone()),
    ]
}
fn seed_link(result: &SeedResultV1) -> ExplanationLink {
    ExplanationLink::Seed(ExplainedSeed {
        result: result.clone(),
        context: context(),
        validation_profile: anchor().validation_profile,
        record_hash: record_hash(&result.to_bytes().unwrap()),
    })
}
fn lifecycle(id: AssertionId, landed: &Landed, lifecycle: AssertionLifecycle) -> ExplanationLink {
    ExplanationLink::Lifecycle(ExplainedLifecycle {
        assertion_id: id,
        lifecycle,
        receipt: landed.receipt.clone(),
        record_hash: record_hash(&landed.receipt.to_bytes().unwrap()),
    })
}
/// The evidence links a chain must end in, ordered by id, each with verified retained bytes.
fn evidence(read: &VerifiedRead, ids: &[EvidenceId]) -> Vec<ExplanationLink> {
    let ids: BTreeSet<EvidenceId> = ids.iter().copied().collect();
    ids.into_iter()
        .map(|id| {
            let record = read.graph.evidence[&id].clone();
            let bytes = read.content(&record.content_hash).unwrap();
            assert_eq!(ContentHash::of_bytes(bytes), record.content_hash);
            assert!(matches!(
                record.source,
                EvidenceSource::HumanStatement { .. }
            ));
            ExplanationLink::Evidence(record)
        })
        .collect()
}
fn claim(read: &VerifiedRead, id: AssertionId) -> ExplanationLink {
    ExplanationLink::Assertion(read.graph.assertions[&id].clone())
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

#[test]
fn a_seeded_assertion_explains_through_the_retained_seed_to_its_evidence() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        let seeded = kernel.seed(seed.document.clone(), at(10)).unwrap();
        drop(kernel);

        let read = open(directory.path(), file).read(None).unwrap();
        let explained = read.explain(seed.assertion).unwrap();
        assert_eq!(explained.assertion_id, seed.assertion);
        assert_eq!(explained.at, RevisionNumber::SEED);
        let mut expected = vec![claim(&read, seed.assertion), seed_link(&seeded)];
        expected.extend(evidence(&read, &[seed.first]));
        assert_eq!(explained.links, expected, "file={file}");
        assert_eq!(kinds(&explained), ["Assertion", "Seed", "Evidence"]);
        // The unused second payload is retained seed input, not support for this assertion.
        assert!(!explained
            .links
            .iter()
            .any(|link| matches!(link, ExplanationLink::Evidence(e) if e.id == seed.second)));
    }
}

#[test]
fn a_committed_assertion_explains_through_its_proposal_validation_and_commit() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let added = assertion(&seed, seed.second, 0);
        let tx = transaction(vec![GraphOperation::AddAssertion(Box::new(added.clone()))]);
        let landed = land(&kernel, &tx, 20);
        drop(kernel);

        let read = open(directory.path(), file).read(None).unwrap();
        let explained = read.explain(added.id).unwrap();
        assert_eq!(explained.at, RevisionNumber::new(1));
        let mut expected = vec![claim(&read, added.id)];
        expected.extend(origin(&landed));
        expected.extend(evidence(&read, &[seed.second]));
        assert_eq!(explained.links, expected, "file={file}");
        assert_eq!(
            kinds(&explained),
            ["Assertion", "Proposal", "Validation", "Commit", "Evidence"]
        );
        // The commit link's record hash is the revision's actual retained record.
        assert_eq!(
            read.revisions[&RevisionNumber::new(1)].record_hash,
            record_hash(&landed.receipt.to_bytes().unwrap())
        );
    }
}

#[test]
fn a_retraction_keeps_the_original_acceptance_and_adds_the_lifecycle_change() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let added = assertion(&seed, seed.second, 0);
        let first = land(
            &kernel,
            &transaction(vec![GraphOperation::AddAssertion(Box::new(added.clone()))]),
            20,
        );
        let reason = RetractionReason::new("withdrawn by the operator");
        let second = land(
            &kernel,
            &transaction(vec![GraphOperation::RetractAssertion(Retraction {
                assertion: added.id,
                reason: reason.clone(),
            })]),
            30,
        );
        drop(kernel);
        let kernel = open(directory.path(), file);

        let read = kernel.read(None).unwrap();
        let explained = read.explain(added.id).unwrap();
        assert_eq!(explained.at, RevisionNumber::new(2));
        let retracted = AssertionLifecycle::Retracted {
            at_revision: RevisionNumber::new(2),
            reason,
        };
        assert_eq!(read.graph.assertions[&added.id].lifecycle, retracted);
        let mut expected = vec![claim(&read, added.id)];
        expected.extend(origin(&first));
        expected.push(lifecycle(added.id, &second, retracted));
        expected.extend(evidence(&read, &[seed.second]));
        assert_eq!(explained.links, expected, "file={file}");

        // The earlier revision is still explainable, and there the assertion was never withdrawn.
        let earlier = kernel.read(Some(RevisionNumber::new(1))).unwrap();
        let before = earlier.explain(added.id).unwrap();
        assert_eq!(before.at, RevisionNumber::new(1));
        let mut expected = vec![claim(&earlier, added.id)];
        expected.extend(origin(&first));
        expected.extend(evidence(&earlier, &[seed.second]));
        assert_eq!(before.links, expected, "file={file}");
        assert_eq!(
            earlier.graph.assertions[&added.id].lifecycle,
            AssertionLifecycle::Active
        );
    }
}

#[test]
fn a_supersession_follows_the_replacement_and_its_distinct_evidence() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        let seeded = kernel.seed(seed.document.clone(), at(10)).unwrap();
        let replacement = assertion(&seed, seed.second, 100);
        let landed = land(
            &kernel,
            &transaction(vec![
                GraphOperation::SupersedeAssertion(Supersession {
                    assertion: seed.assertion,
                    by: replacement.id,
                    effective_from: Timestamp::from_millis(100),
                }),
                GraphOperation::AddAssertion(Box::new(replacement.clone())),
            ]),
            20,
        );
        drop(kernel);

        let read = open(directory.path(), file).read(None).unwrap();
        let explained = read.explain(seed.assertion).unwrap();
        let superseded = AssertionLifecycle::Superseded {
            by: CanonicalRef::new(replacement.id),
            at_revision: RevisionNumber::new(1),
            effective_from: Timestamp::from_millis(100),
        };
        let mut expected = vec![
            claim(&read, seed.assertion),
            seed_link(&seeded),
            lifecycle(seed.assertion, &landed, superseded),
            claim(&read, replacement.id),
        ];
        expected.extend(origin(&landed));
        expected.extend(evidence(&read, &[seed.first, seed.second]));
        assert_eq!(explained.links, expected, "file={file}");
    }
}

/// One deliberate corruption of a captured read.
type Fault<'a> = Box<dyn Fn(&mut VerifiedRead) + 'a>;

/// Seeded A, then B accepted on its own at revision 1, then A superseded by B at revision 2.
struct Replaced {
    seed: Seeded,
    seeded: SeedResultV1,
    replacement: AssertionId,
    accepted: Landed,
    superseded: Landed,
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
    let superseded = land(
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
        superseded,
    }
}

#[test]
fn a_replacement_accepted_before_the_supersession_is_explained_from_its_own_acceptance() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let case = replaced_later(directory.path(), file);
        let read = open(directory.path(), file).read(None).unwrap();
        let explained = read.explain(case.seed.assertion).unwrap();
        assert_eq!(explained.at, RevisionNumber::new(2));
        let superseded = AssertionLifecycle::Superseded {
            by: CanonicalRef::new(case.replacement),
            at_revision: RevisionNumber::new(2),
            effective_from: Timestamp::from_millis(100),
        };
        let mut expected = vec![
            claim(&read, case.seed.assertion),
            seed_link(&case.seeded),
            lifecycle(case.seed.assertion, &case.superseded, superseded),
            claim(&read, case.replacement),
        ];
        expected.extend(origin(&case.accepted));
        expected.extend(evidence(&read, &[case.seed.first, case.seed.second]));
        assert_eq!(explained.links, expected, "file={file}");
    }
}

#[test]
fn a_snapshot_carries_the_complete_graph_and_root_and_the_shared_valid_time_selection() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let case = replaced_later(directory.path(), file);
        let kernel = open(directory.path(), file);
        let (old, new) = (case.seed.assertion, case.replacement);

        let read = kernel.read(None).unwrap();
        let full = read.snapshot(None).unwrap();
        assert_eq!(full.revision_id, case.superseded.receipt.revision_id);
        assert_eq!(full.root, read.root);
        assert_eq!(Some(full.root), kernel.head().unwrap());
        assert_eq!(
            full.graph,
            ekr_store::GraphDocument::of(&kernel.snapshot().unwrap())
        );
        assert_eq!(full.valid_at, None);
        assert_eq!(full.matching_assertions, None);

        for (millis, expected) in [(-5, vec![]), (99, vec![old]), (100, vec![new])] {
            let t = Timestamp::from_millis(millis);
            let selected = read.snapshot(Some(t)).unwrap();
            // A filtered view keeps the complete graph and root; it never claims a filtered hash.
            assert_eq!(selected.root, full.root);
            assert_eq!(selected.graph, full.graph);
            assert_eq!(selected.revision_id, full.revision_id);
            assert_eq!(selected.valid_at, Some(t));
            let shared: Vec<AssertionId> = GraphSnapshot::of(&read.graph)
                .valid_at(t)
                .iter()
                .map(|a| a.id)
                .collect();
            assert_eq!(selected.matching_assertions, Some(shared));
            assert_eq!(
                selected.matching_assertions,
                Some(expected),
                "file={file} t={millis}"
            );
        }

        // An earlier committed revision: B accepted, A not yet superseded, so both answer at 100.
        let earlier = kernel.read(Some(RevisionNumber::new(1))).unwrap();
        let one = earlier.snapshot(Some(Timestamp::from_millis(100))).unwrap();
        assert_eq!(one.revision_id, case.accepted.receipt.revision_id);
        assert_eq!(one.root, case.accepted.receipt.result);
        assert_eq!(
            one.graph,
            ekr_store::GraphDocument::of(&kernel.replay(RevisionNumber::new(1)).unwrap())
        );
        let mut both = vec![old, new];
        both.sort();
        assert_eq!(one.matching_assertions, Some(both), "file={file}");
    }
}

#[test]
fn an_unknown_assertion_is_refused_with_its_identity() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let kernel = open(directory.path(), file);
        kernel.seed(fixture().document, at(10)).unwrap();
        let requested = AssertionId::mint();
        match kernel.read(None).unwrap().explain(requested) {
            Err(ProjectionError::AssertionNotFound { requested: named }) => {
                assert_eq!(named, requested);
            }
            other => panic!("file={file}: {other:?}"),
        }
    }
}

#[test]
fn missing_or_corrupt_required_support_refuses_the_whole_chain() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let case = replaced_later(directory.path(), file);
        let kernel = open(directory.path(), file);
        let old = case.seed.assertion;
        let accepted_id = case.accepted.proposal.transaction_id;
        let superseded_id = case.superseded.proposal.transaction_id;
        let faults: Vec<(&str, &str, Fault<'_>)> = vec![
            (
                "evidence record missing",
                "evidence-root-disagrees",
                Box::new(|r| {
                    r.graph.evidence.remove(&case.seed.second);
                }),
            ),
            (
                "evidence payload not retained",
                "evidence-root-disagrees",
                Box::new(|r| {
                    r.graph
                        .evidence
                        .get_mut(&case.seed.first)
                        .unwrap()
                        .content_hash = ContentHash::of_bytes(b"never retained");
                }),
            ),
            (
                "replacement assertion missing",
                "knowledge-root-disagrees",
                Box::new(|r| {
                    r.graph.assertions.remove(&case.replacement);
                }),
            ),
            (
                "replacement acceptance missing",
                "origin-missing",
                Box::new(|r| {
                    r.transactions.remove(&accepted_id);
                }),
            ),
            (
                "lifecycle transaction missing",
                "lifecycle-disagrees",
                Box::new(|r| {
                    r.transactions.remove(&superseded_id);
                }),
            ),
            (
                "commit receipt differs from its retained record",
                "commit-record-disagrees",
                Box::new(|r| {
                    let held = r.transactions.get_mut(&accepted_id).unwrap();
                    held.committed.as_mut().unwrap().committer = context().validator;
                }),
            ),
            (
                "validation differs from its retained record",
                "validation-disagrees",
                Box::new(|r| {
                    let held = r.transactions.get_mut(&accepted_id).unwrap();
                    held.validation_record_hash = Some(ContentHash::of_bytes(b"no such record"));
                }),
            ),
            (
                "proposal differs from its retained record",
                "proposal-record-disagrees",
                Box::new(|r| {
                    let held = r.transactions.get_mut(&accepted_id).unwrap();
                    held.proposal_record_hash = ContentHash::of_bytes(b"no such record");
                }),
            ),
            (
                "revision coordinate missing",
                "revision-coordinate-missing",
                Box::new(|r| {
                    r.revisions.remove(&RevisionNumber::new(1));
                }),
            ),
            (
                "seed record differs",
                "seed-record-disagrees",
                Box::new(|r| {
                    r.seed.committed_at = Timestamp::from_millis(11);
                }),
            ),
        ];
        let mut admitted = Vec::new();
        for (name, code, fault) in &faults {
            let mut read = kernel.read(None).unwrap();
            assert!(read.explain(old).is_ok(), "control: {name}");
            fault(&mut read);
            match read.explain(old) {
                Err(ProjectionError::Unverified { code: held }) if held == *code => {}
                other => admitted.push(format!("file={file}/{name}: {other:?}")),
            }
        }
        assert!(
            admitted.is_empty(),
            "partial chains returned: {admitted:#?}"
        );

        // A payload deleted from the provider refuses the read itself: no chain is produced.
        drop(kernel);
        let hash = case.seed.document.graph.evidence[&case.seed.second].content_hash;
        let executor = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        executor.block_on(async {
            let provider: Box<dyn eventlog_core::EventStore> = if file {
                Box::new(
                    eventlog_file::FileEventStore::open(directory.path())
                        .await
                        .unwrap(),
                )
            } else {
                Box::new(
                    eventlog_sqlite::SqliteEventStore::open(
                        &directory.path().join("state.db").to_string_lossy(),
                        "ekr",
                    )
                    .await
                    .unwrap(),
                )
            };
            provider
                .delete_blob(
                    &eventlog_core::TenantId::new("test").unwrap(),
                    &hash.to_hex(),
                )
                .await
                .unwrap();
        });
        assert!(
            open(directory.path(), file).read(None).is_err(),
            "file={file}: a read without retained support"
        );
    }
}

#[test]
fn a_captured_read_explained_after_another_commit_does_not_show_the_later_change() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = fixture();
        let kernel = open(directory.path(), file);
        kernel.seed(seed.document.clone(), at(10)).unwrap();
        let added = assertion(&seed, seed.second, 0);
        let first = land(
            &kernel,
            &transaction(vec![GraphOperation::AddAssertion(Box::new(added.clone()))]),
            20,
        );
        let captured = kernel.read(None).unwrap();

        let second = land(
            &kernel,
            &transaction(vec![GraphOperation::RetractAssertion(Retraction {
                assertion: added.id,
                reason: RetractionReason::new("later withdrawal"),
            })]),
            30,
        );
        assert_eq!(
            kernel.head().unwrap().unwrap().revision,
            RevisionNumber::new(2)
        );

        let held = captured.explain(added.id).unwrap();
        assert_eq!(held.at, RevisionNumber::new(1));
        let mut expected = vec![claim(&captured, added.id)];
        expected.extend(origin(&first));
        expected.extend(evidence(&captured, &[seed.second]));
        assert_eq!(held.links, expected, "file={file}");
        assert!(!held
            .links
            .iter()
            .any(|link| matches!(link, ExplanationLink::Lifecycle(_))));
        assert_eq!(captured.snapshot(None).unwrap().root, first.receipt.result);

        // A new capture does show it, so the absence above is the boundary and not a gap.
        let fresh = kernel.read(None).unwrap().explain(added.id).unwrap();
        assert!(fresh.links.iter().any(|link| matches!(
            link,
            ExplanationLink::Lifecycle(change) if change.receipt == second.receipt
        )));
    }
}

#[test]
fn every_selected_assertion_is_visited_once_in_stable_order() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let case = replaced_later(directory.path(), file);
        let read = open(directory.path(), file).read(None).unwrap();
        // Explaining the replacement directly does not walk back to the assertion it replaced.
        let explained = read.explain(case.replacement).unwrap();
        assert_eq!(
            kinds(&explained),
            ["Assertion", "Proposal", "Validation", "Commit", "Evidence"],
            "file={file}"
        );
        let from_old = read.explain(case.seed.assertion).unwrap();
        let assertions: Vec<AssertionId> = from_old
            .links
            .iter()
            .filter_map(|link| match link {
                ExplanationLink::Assertion(a) => Some(a.id),
                _ => None,
            })
            .collect();
        assert_eq!(assertions, [case.seed.assertion, case.replacement]);
    }
}

/// Every public field of a capture that a projection reads is bound to retained bytes: a
/// forgery of any one of them refuses both Snapshot and Explain with the named check.
#[test]
fn every_capture_field_a_projection_reads_is_bound_to_the_retained_root_and_seed() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let case = replaced_later(directory.path(), file);
        let kernel = open(directory.path(), file);
        let earlier_root = kernel.read(Some(RevisionNumber::new(1))).unwrap().root;
        let old = case.seed.assertion;
        let faults: Vec<(&str, &str, Fault<'_>)> = vec![
            (
                "graph assertion rewritten",
                "knowledge-root-disagrees",
                Box::new(|r| {
                    r.graph.assertions.get_mut(&old).unwrap().object =
                        Object::Value(CanonicalValue::String("forged".into()));
                }),
            ),
            (
                "graph evidence redirected",
                "evidence-root-disagrees",
                Box::new(|r| {
                    let second = r.graph.evidence[&case.seed.second].content_hash;
                    r.graph
                        .evidence
                        .get_mut(&case.seed.first)
                        .unwrap()
                        .content_hash = second;
                }),
            ),
            (
                "graph ontology replaced",
                "ontology-root-disagrees",
                Box::new(|r| {
                    r.graph.ontology = kernel
                        .read(Some(RevisionNumber::SEED))
                        .map(|seed| {
                            let mut document = r.seed_input.ontology.clone();
                            document.node_types.clear();
                            let _ = seed;
                            ekr_ontology::Ontology::load(document).unwrap()
                        })
                        .unwrap();
                }),
            ),
            (
                "graph root rewritten",
                "graph-root-disagrees",
                Box::new(|r| {
                    r.graph.root.created_at = Timestamp::from_millis(987_654_321);
                    r.graph.root.parent = Some(GraphRootId::mint());
                }),
            ),
            (
                "graph revision relabelled",
                "graph-revision-disagrees",
                Box::new(|r| {
                    r.graph.revision = RevisionNumber::new(1);
                }),
            ),
            (
                "root replaced by an earlier revision's root",
                "root-coordinate-disagrees",
                Box::new(move |r| {
                    r.root = earlier_root;
                }),
            ),
            (
                "authority registry rewritten",
                "authority-root-disagrees",
                Box::new(|r| {
                    r.authority.agents.values_mut().next().unwrap().name = "renamed".into();
                }),
            ),
            (
                "seed input rewritten",
                "seed-envelope-disagrees",
                Box::new(|r| {
                    r.seed_input.evidence_payloads.clear();
                }),
            ),
            (
                "bootstrap context swapped",
                "seed-envelope-disagrees",
                Box::new(|r| {
                    r.context = BootstrapContext {
                        operator: r.context.validator,
                        validator: r.context.operator,
                    };
                }),
            ),
            (
                "seed hash redirected",
                "seed-envelope-disagrees",
                Box::new(|r| {
                    r.seed.seed_hash = r.graph.evidence[&case.seed.first].content_hash;
                }),
            ),
        ];
        let mut admitted = Vec::new();
        for (name, code, fault) in &faults {
            let mut read = kernel.read(None).unwrap();
            assert!(read.explain(old).is_ok() && read.snapshot(None).is_ok());
            fault(&mut read);
            for (projection, outcome) in [
                ("explain", read.explain(old).err()),
                ("snapshot", read.snapshot(None).err()),
            ] {
                match outcome {
                    Some(ProjectionError::Unverified { code: held }) if held == *code => {}
                    other => admitted.push(format!("file={file}/{projection}/{name}: {other:?}")),
                }
            }
        }
        assert!(admitted.is_empty(), "unbound capture fields: {admitted:#?}");
    }
}
