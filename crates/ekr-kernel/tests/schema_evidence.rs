//! Schema supporting evidence is admitted only after an explicit reviewed authority upgrade.
include!("support/authority_review_fixture.rs");

fn schema(evidence: EvidenceId) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            TypeId::mint(),
            "HealthObservation",
        )))],
        evidence: BTreeSet::from([evidence]),
        schema_version: Some(ekr_core::SchemaVersionId::mint()),
    }
}

fn run<S: RevisionLog + ObjectStore + Initialize + ObservationRetention + IncubationRetention>(
    open: impl Fn(Option<m::TrustedReviewHostBinding>, bool) -> Commit<S>,
) {
    assert!(!ValidationProfileV1::knowledge(context().validator).supports_schema_evidence());
    assert!(ValidationProfileV1::knowledge_evidence(context().validator).supports_schema_evidence());
    let doc = seed();
    let old = open(None, false);
    let seeded = old.seed(doc.clone(), || Timestamp::EPOCH).unwrap();
    let original_root = old.head().unwrap();
    let support = *doc.graph.evidence.keys().next().unwrap();
    let historical = schema(support);
    old.propose(&encode(&historical), context().operator, || {
        Timestamp::from_millis(1)
    })
    .unwrap();
    assert!(matches!(
        old.validate(historical.id, RevisionNumber::SEED, || {
            Timestamp::from_millis(2)
        })
        .unwrap(),
        ValidationCommandResult::Rejected(_)
    ));
    assert_eq!(
        old.transaction_states([historical.id]).unwrap()[&historical.id],
        TransactionState::Rejected
    );
    let human = Human::new(seeded.seed_hash);
    drop(old);
    let kernel = open(Some(human.binding.clone()), false);
    let preview = kernel.preview_upgrade(&human.policy).unwrap();
    let proof = human.proof(&preview);
    kernel
        .apply_upgrade(
            &preview,
            &human.policy,
            &proof,
            b"reviewed contradictions and pending validations",
            || Timestamp::from_millis(3),
        )
        .unwrap();
    let mut committed = Vec::new();
    for inline in [false, true] {
        let time = if inline { 20 } else { 10 };
        let mut tx = schema(support);
        if inline {
            let payload = b"Synthetic health vocabulary evidence".to_vec();
            let evidence = Evidence {
                id: EvidenceId::mint(),
                source: EvidenceSource::HumanStatement { identity: None },
                content_hash: ContentHash::of_bytes(&payload),
                extracted_by: context().operator,
                observed_at: Timestamp::from_millis(4),
                confidence: Confidence::CERTAIN,
            };
            tx.evidence.insert(evidence.id);
            tx.operations.insert(
                0,
                GraphOperation::AddEvidence(Box::new(ekr_kernel::EvidenceAddition {
                    evidence,
                    payload,
                })),
            );
        }
        kernel
            .propose(&encode(&tx), context().operator, || {
                Timestamp::from_millis(time)
            })
            .unwrap();
        let revision = kernel.head().unwrap().unwrap().revision;
        let result = kernel
            .validate(tx.id, revision, || Timestamp::from_millis(time + 1))
            .unwrap();
        assert!(
            matches!(result, ValidationCommandResult::Validated(_)),
            "schema supporting evidence must validate (inline={inline}): {result:?}"
        );
        assert!(matches!(
            kernel
                .commit(tx.id, context().operator, || Timestamp::from_millis(
                    time + 2
                ))
                .unwrap(),
            CommitCommandResult::Committed(_)
        ));
        let graph = kernel.snapshot().unwrap();
        assert_eq!(graph.ontology.version().id, tx.schema_version.unwrap());
        for id in &tx.evidence {
            let evidence = &graph.evidence[id];
            assert_eq!(
                ContentHash::of_bytes(&kernel.content(&evidence.content_hash).unwrap().unwrap()),
                evidence.content_hash
            );
        }
        committed.push((graph.revision, tx));
    }
    // An ordinary data operation never becomes admissible merely by sharing a schema transaction.
    let mut mixed = schema(support);
    // Isolate the schema/data boundary: a stray manifest must not mask a missing shape guard.
    mixed.evidence.clear();
    let (_, document) = proposal(&doc);
    let data = ekr_kernel::TransactionDocument::parse(&document).unwrap();
    mixed
        .operations
        .extend(data.transaction().operations.clone());
    kernel
        .propose(&encode(&mixed), context().operator, || {
            Timestamp::from_millis(30)
        })
        .unwrap();
    let ValidationCommandResult::Rejected(mixed_rejection) = kernel
        .validate(mixed.id, kernel.head().unwrap().unwrap().revision, || {
            Timestamp::from_millis(31)
        })
        .unwrap()
    else {
        panic!("mixed schema/data transaction was admitted");
    };
    assert!(
        mixed_rejection
            .issues
            .iter()
            .any(|issue| issue.code == "mixed-schema-transaction"),
        "wrong refusal for mixed schema/data transaction: {:?}",
        mixed_rejection.issues
    );
    let unchanged = kernel.head().unwrap();
    let unknown = schema(EvidenceId::mint());
    let mut uncited = schema(support);
    let mut bad_payload = schema(support);
    for (tx, cited, payload) in [
        (&mut uncited, false, b"valid retained source".to_vec()),
        (
            &mut bad_payload,
            true,
            b"bytes do not match the declared hash".to_vec(),
        ),
    ] {
        let id = EvidenceId::mint();
        if cited {
            tx.evidence.insert(id);
        }
        tx.operations.push(GraphOperation::AddEvidence(Box::new(
            ekr_kernel::EvidenceAddition {
                evidence: Evidence {
                    id,
                    source: EvidenceSource::HumanStatement { identity: None },
                    content_hash: ContentHash::of_bytes(b"valid retained source"),
                    extracted_by: context().operator,
                    observed_at: Timestamp::from_millis(32),
                    confidence: Confidence::CERTAIN,
                },
                payload,
            },
        )));
    }
    for (tx, code) in [
        (&unknown, "unresolved-evidence"),
        (&uncited, "schema-evidence-not-cited"),
        (&bad_payload, "evidence-payload-mismatch"),
    ] {
        kernel
            .propose(&encode(tx), context().operator, || {
                Timestamp::from_millis(33)
            })
            .unwrap();
        let ValidationCommandResult::Rejected(record) = kernel
            .validate(tx.id, unchanged.unwrap().revision, || {
                Timestamp::from_millis(34)
            })
            .unwrap()
        else {
            panic!("invalid schema evidence was admitted: {code}");
        };
        assert!(
            record.issues.iter().any(|issue| issue.code == code),
            "expected {code}: {:?}",
            record.issues
        );
        assert_eq!(kernel.head().unwrap(), unchanged);
    }
    let root = kernel.head().unwrap();
    drop(kernel);
    for full in [false, true] {
        let reopened = open(Some(human.binding.clone()), full);
        assert_eq!(reopened.head().unwrap(), root);
        assert_eq!(
            reopened
                .schema_history(RevisionNumber::SEED)
                .unwrap()
                .revisions[&RevisionNumber::SEED]
                .root,
            original_root.unwrap()
        );
        for (revision, tx) in &committed {
            let history = reopened.schema_history(*revision).unwrap();
            assert_eq!(history.transactions[revision], tx.id);
            assert_eq!(history.supporting_evidence[revision], tx.evidence);
            assert_eq!(
                history.graph.ontology.version().id,
                tx.schema_version.unwrap()
            );
        }
    }
}

#[test]
fn schema_support_survives_file_reopen_and_full_replay() {
    let directory = tempfile::tempdir().unwrap();
    run(|binding, full| {
        let open = |authority| {
            let mut store =
                FileStore::file(directory.path(), "upgrade-fixture", None)?.under(authority);
            store.set_full_replay(full);
            Ok(store)
        };
        match binding {
            Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
            None => Commit::over_with_authority(context(), anchor(), open),
        }
        .unwrap()
    });
}

#[test]
fn schema_support_survives_sqlite_reopen_and_full_replay() {
    let directory = tempfile::tempdir().unwrap();
    run(|binding, full| {
        let open = |authority| {
            let mut store =
                SqliteStore::sqlite(&directory.path().join("store.db"), "upgrade-fixture", None)?
                    .under(authority);
            store.set_full_replay(full);
            Ok(store)
        };
        match binding {
            Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
            None => Commit::over_with_authority(context(), anchor(), open),
        }
        .unwrap()
    });
}

fn copy_fixture(source: &std::path::Path, target: &std::path::Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_fixture(&entry.path(), &target.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
        }
    }
}

#[test]
fn native_knowledge_one_upgrades_without_rewriting_its_rejection_or_roots() {
    use ekr_kernel::Runtime;
    for sqlite in [false, true] {
        let fixture = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
            .join("tests/fixtures/knowledge-one")
            .join(if sqlite { "sqlite" } else { "file" });
        let directory = tempfile::tempdir().unwrap();
        copy_fixture(&fixture, directory.path());
        let metadata: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.path().join("metadata.json")).unwrap())
                .unwrap();
        let binding_bytes: Vec<u8> = serde_json::from_value(metadata["binding"].clone()).unwrap();
        let policy_bytes: Vec<u8> = serde_json::from_value(metadata["policy"].clone()).unwrap();
        let binding = review::read_host_binding(&binding_bytes).unwrap();
        let policy = review::read_policy(&policy_bytes).unwrap();
        let seed_root: ekr_graph::Root =
            serde_json::from_value(metadata["seed_root"].clone()).unwrap();
        let legacy_root: ekr_graph::Root =
            serde_json::from_value(metadata["legacy_root"].clone()).unwrap();
        let pending: TransactionId = serde_json::from_value(metadata["pending"].clone()).unwrap();
        let rejected: TransactionId = serde_json::from_value(metadata["rejected"].clone()).unwrap();
        let open = |full| {
            let runtime = if sqlite {
                Runtime::sqlite(
                    &directory.path().join("store.db"),
                    "upgrade-fixture",
                    context(),
                    anchor(),
                )
            } else {
                Runtime::file(
                    &directory.path().join("store"),
                    "upgrade-fixture",
                    context(),
                    anchor(),
                )
            }
            .unwrap();
            let mut runtime = runtime.with_review_authority(binding.clone()).unwrap();
            runtime.set_full_replay(full);
            runtime
        };
        let runtime = open(true);
        assert_eq!(runtime.head().unwrap(), Some(legacy_root));
        let before = runtime.transactions().unwrap();
        assert_eq!(before[&pending].state(), TransactionState::Validated);
        assert_eq!(before[&rejected].state(), TransactionState::Rejected);
        let mut claim = runtime
            .read(None)
            .unwrap()
            .seed_input
            .graph
            .assertions
            .values()
            .next()
            .unwrap()
            .clone();
        claim.id = AssertionId::mint();
        claim.assessment = Assessment::Proposed;
        claim.transaction_time = TransactionTime::since(Timestamp::from_millis(6));
        let claim_id = claim.id;
        let assertion_tx = GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            evidence: claim.evidence.clone(),
            schema_version: None,
            operations: vec![GraphOperation::AddAssertion(Box::new(claim))],
        };
        runtime
            .propose(&encode(&assertion_tx), context().operator, || {
                Timestamp::from_millis(6)
            })
            .unwrap();
        assert!(matches!(
            runtime
                .validate(assertion_tx.id, legacy_root.revision, || {
                    Timestamp::from_millis(7)
                })
                .unwrap(),
            ValidationCommandResult::Validated(_)
        ));
        runtime
            .commit(assertion_tx.id, context().operator, || {
                Timestamp::from_millis(8)
            })
            .unwrap();
        let assertion_revision = runtime.head().unwrap().unwrap().revision;
        let explain_old = |read: ekr_kernel::VerifiedRead| {
            let explanation = read.explain(claim_id).expect(
                "a knowledge/1 assertion keeps its original explanation after the second upgrade",
            );
            let validation = explanation
                .links
                .iter()
                .find_map(|link| match link {
                    ekr_kernel::ExplanationLink::Validation(validation) => Some(validation),
                    _ => None,
                })
                .expect("ordinary assertion has a validation link");
            assert_eq!(
                validation.validation_profile,
                ValidationProfileV1::knowledge(context().validator)
            );
        };
        explain_old(runtime.read(None).unwrap());
        let human = Human::new(runtime.read(None).unwrap().seed.seed_hash);
        assert_eq!(review::policy_bytes(&human.policy).unwrap(), policy_bytes);
        let preview = runtime.preview_upgrade(&policy).unwrap();
        assert_eq!(preview.from.ruleset.0, "ekr.knowledge-deterministic/1");
        assert_eq!(preview.to.ruleset.0, "ekr.knowledge-deterministic/3");
        assert_eq!(preview.pending_revalidation.len(), 1);
        let proof = human.proof(&preview);
        let receipt = runtime
            .apply_upgrade(
                &preview,
                &policy,
                &proof,
                b"reviewed contradictions and pending validations",
                || Timestamp::from_millis(10),
            )
            .unwrap();
        assert_eq!(
            runtime.transaction_states([pending]).unwrap()[&pending],
            TransactionState::Proposed
        );
        assert!(runtime
            .commit(pending, context().operator, || Timestamp::from_millis(11))
            .is_err());
        let support = *runtime.snapshot().unwrap().evidence.keys().next().unwrap();
        let tx = schema(support);
        runtime
            .propose(&encode(&tx), context().operator, || {
                Timestamp::from_millis(12)
            })
            .unwrap();
        assert!(matches!(
            runtime
                .validate(tx.id, runtime.head().unwrap().unwrap().revision, || {
                    Timestamp::from_millis(13)
                })
                .unwrap(),
            ValidationCommandResult::Validated(_)
        ));
        runtime
            .commit(tx.id, context().operator, || Timestamp::from_millis(14))
            .unwrap();
        let final_root = runtime.head().unwrap().unwrap();
        assert!(runtime.preview_upgrade(&policy).is_err());
        drop(runtime);
        for full in [false, true] {
            let reopened = open(full);
            assert_eq!(reopened.head().unwrap(), Some(final_root));
            explain_old(reopened.read(Some(assertion_revision)).unwrap());
            explain_old(reopened.read(None).unwrap());
            let history = reopened.schema_history(final_root.revision).unwrap();
            assert_eq!(history.revisions[&RevisionNumber::SEED].root, seed_root);
            assert_eq!(history.revisions[&legacy_root.revision].root, legacy_root);
            assert_eq!(
                history.supporting_evidence[&final_root.revision],
                tx.evidence
            );
            let after = reopened.transactions().unwrap();
            assert_eq!(after[&rejected], before[&rejected]);
            let retried = reopened
                .apply_upgrade(
                    &preview,
                    &policy,
                    &proof,
                    b"reviewed contradictions and pending validations",
                    || panic!("retry must retain its time"),
                )
                .unwrap();
            assert_eq!(
                serde_json::to_vec(&retried).unwrap(),
                serde_json::to_vec(&receipt).unwrap()
            );
        }
    }
}
