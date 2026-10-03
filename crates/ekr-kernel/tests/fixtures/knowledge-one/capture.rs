//! Temporary capture of native stores written by the unmodified knowledge/1 implementation.
include!("support/authority_review_fixture.rs");

fn capture<S: RevisionLog + ObjectStore + Initialize + ObservationRetention + IncubationRetention>(
    open: impl Fn(Option<m::TrustedReviewHostBinding>) -> Commit<S>,
    output: &std::path::Path,
) {
    let _: Option<CommitCommandResult> = None;
    let old = open(None);
    let doc = seed();
    let seeded = old.seed(doc.clone(), || Timestamp::EPOCH).unwrap();
    let human = Human::new(seeded.seed_hash);
    let original = old.head().unwrap().unwrap();
    assert_eq!(original.revision, RevisionNumber::SEED);
    drop(old);
    let kernel = open(Some(human.binding.clone()));
    let preview = kernel.preview_upgrade(&human.policy).unwrap();
    assert_eq!(preview.to.ruleset.0, "ekr.knowledge-deterministic/1");
    let transition = kernel.apply_upgrade(&preview, &human.policy, &human.proof(&preview), b"reviewed contradictions and pending validations", || Timestamp::from_millis(1)).unwrap();
    let tx = GraphTransaction {
        id: TransactionId::mint(), proposer: context().operator,
        operations: vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(TypeId::mint(), "ParkedHealth")))],
        evidence: doc.graph.evidence.keys().copied().collect(),
        schema_version: Some(ekr_core::SchemaVersionId::mint()),
    };
    kernel.propose(&encode(&tx), context().operator, || Timestamp::from_millis(2)).unwrap();
    let rejected = kernel.validate(tx.id, kernel.head().unwrap().unwrap().revision, || Timestamp::from_millis(3)).unwrap();
    assert!(matches!(rejected, ValidationCommandResult::Rejected(_)));
    assert_eq!(kernel.transaction_states([tx.id]).unwrap()[&tx.id], TransactionState::Rejected);
    let (pending, document) = proposal(&doc);
    kernel.propose(&document, context().operator, || Timestamp::from_millis(4)).unwrap();
    assert!(matches!(kernel.validate(pending, kernel.head().unwrap().unwrap().revision, || Timestamp::from_millis(5)).unwrap(), ValidationCommandResult::Validated(_)));
    let metadata = serde_json::json!({"binding": review::host_binding_bytes(&human.binding).unwrap(), "policy": review::policy_bytes(&human.policy).unwrap(), "seed_root": original, "legacy_root": kernel.head().unwrap().unwrap(), "rejected": tx.id, "pending": pending, "transition": transition});
    std::fs::write(output.join("metadata.json"), serde_json::to_vec_pretty(&metadata).unwrap()).unwrap();
}

#[test]
fn capture_native_knowledge_one() {
    let root = std::path::PathBuf::from(std::env::var_os("EKR_LEGACY_CAPTURE").unwrap());
    for sqlite in [false, true] {
        let output = root.join(if sqlite { "sqlite" } else { "file" });
        std::fs::create_dir(&output).unwrap();
        if sqlite {
            capture(|binding| {
                let open = |authority| Ok(SqliteStore::sqlite(&output.join("store.db"), "upgrade-fixture", None)?.under(authority));
                match binding {
                    Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
                    None => Commit::over_with_authority(context(), anchor(), open),
                }.unwrap()
            }, &output);
        } else {
            capture(|binding| {
                let open = |authority| Ok(FileStore::file(&output.join("store"), "upgrade-fixture", None)?.under(authority));
                match binding {
                    Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
                    None => Commit::over_with_authority(context(), anchor(), open),
                }.unwrap()
            }, &output);
        }
    }
}
