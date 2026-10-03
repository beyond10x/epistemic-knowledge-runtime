//! Fresh-process recovery after port-level unsent/lost response injection, not a native crash.
#![allow(dead_code, unused_imports)]
include!("support/attention_answer_fixture.rs");
#[path = "recovery/support.rs"]
mod recovery;

#[derive(serde::Serialize, serde::Deserialize)]
struct RetryPlan {
    file: bool,
    proof: Vec<u8>,
    binding: Vec<u8>,
    assertion: String,
}
impl RetryPlan {
    fn input(&self) -> m::AttentionAnswerApplication {
        let proof = review::read_proof(&self.proof).unwrap();
        let m::HumanDecisionTarget::AnswerAttention(target) = &proof.intent.target else {
            panic!("answer target")
        };
        m::AttentionAnswerApplication {
            dispute_id: target.dispute_id.clone(),
            basis: target.basis.clone(),
            human_proof: proof,
            corrections: vec![m::ClaimCorrection {
                kind: m::ClaimCorrectionKind::Choose,
                assertion_id: ekr_core::contracts::graph::AssertionId(Uuid(self.assertion.clone())),
                valid_from: None,
                valid_to: None,
                reason: "human reviewed the retained source evidence".into(),
            }],
            statement: b"reviewed answer".to_vec(),
        }
    }
}
fn resume<S: RevisionLog + ObjectStore>(kernel: Commit<S>, plan: &RetryPlan) -> Vec<u8> {
    let result = kernel
        .answer_attention(&plan.input(), || panic!("retry sampled time"))
        .unwrap();
    assert_eq!(kernel.answer_history(None).unwrap(), vec![result.clone()]);
    assert!(kernel
        .read(None)
        .unwrap()
        .dispute_attention()
        .unwrap()
        .is_empty());
    serde_json::to_vec(&result).unwrap()
}
fn child(path: &std::path::Path) {
    let plan: RetryPlan =
        serde_json::from_slice(&std::fs::read(path.join("retry.json")).unwrap()).unwrap();
    let binding = review::read_host_binding(&plan.binding).unwrap();
    let result = if plan.file {
        resume(
            Commit::over_with_review_authority(context(), anchor(), binding, |a| {
                let mut s = FileStore::file(path, "upgrade-fixture", None)?.under(a);
                s.set_full_replay(true);
                Ok(s)
            })
            .unwrap(),
            &plan,
        )
    } else {
        resume(
            Commit::over_with_review_authority(context(), anchor(), binding, |a| {
                let mut s =
                    SqliteStore::sqlite(&path.join("store.db"), "upgrade-fixture", None)?.under(a);
                s.set_full_replay(true);
                Ok(s)
            })
            .unwrap(),
            &plan,
        )
    };
    std::fs::write(path.join("result.json"), result).unwrap();
}
fn recover<S: RevisionLog + ObjectStore + Initialize>(
    path: &std::path::Path,
    file: bool,
    fault: recovery::Fault,
    open: impl Fn(Option<m::TrustedReviewHostBinding>, recovery::Hooks) -> Commit<recovery::Probe<S>>,
) {
    let seeded = open(None, recovery::Hooks::new(recovery::Fault::Pass))
        .seed(seed(), || Timestamp::EPOCH)
        .unwrap();
    let mut human = Human::new(seeded.seed_hash);
    human.policy.keys[0]
        .scopes
        .insert(0, m::HumanDecisionScope::AnswerAttention);
    human.binding.reviewer_policy_digest = hash(&review::policy_bytes(&human.policy).unwrap());
    let kernel = open(
        Some(human.binding.clone()),
        recovery::Hooks::new(recovery::Fault::Pass),
    );
    let preview = kernel.preview_upgrade(&human.policy).unwrap();
    kernel
        .apply_upgrade(
            &preview,
            &human.policy,
            &human.proof(&preview),
            b"reviewed contradictions and pending validations",
            || Timestamp::from_millis(1),
        )
        .unwrap();
    let input = answer(
        &human,
        &kernel.read(None).unwrap(),
        m::ClaimCorrectionKind::Choose,
    );
    drop(kernel);
    let hooks = recovery::Hooks::new(fault);
    let kernel = open(Some(human.binding.clone()), hooks.clone());
    assert!(matches!(
        kernel.answer_attention(&input, || Timestamp::from_millis(2)),
        Err(ekr_kernel::CommitError::Store(
            ekr_store::StoreError::UnknownCommit
        ))
    ));
    let elected = hooks.resumed.borrow()[0].clone();
    assert_eq!(
        elected.format,
        ekr_store::PublicationPreparationV1::FORMAT_V5
    );
    assert_eq!(elected.decision.event.schema_version(), 4);
    let mut wrong = input.clone();
    wrong.statement.push(b'!');
    assert!(kernel
        .answer_attention(&wrong, || panic!("changed retry allocated"))
        .is_err());
    drop(kernel);
    let plan = RetryPlan {
        file,
        proof: review::proof_bytes(&input.human_proof).unwrap(),
        binding: review::host_binding_bytes(&human.binding).unwrap(),
        assertion: input.corrections[0].assertion_id.0 .0.clone(),
    };
    std::fs::write(path.join("retry.json"), serde_json::to_vec(&plan).unwrap()).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "answer_port_fault_resumes_exactly_in_a_fresh_process",
            "--exact",
            "--test-threads=1",
        ])
        .env("EKR_ANSWER_RETRY_PATH", path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child failed: {} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let result = std::fs::read(path.join("result.json")).unwrap();
    assert_eq!(
        result,
        elected.decision.objects[&elected.decision.event.record_hash].bytes
    );
    assert_eq!(
        resume(
            open(
                Some(human.binding),
                recovery::Hooks::new(recovery::Fault::Pass)
            ),
            &plan
        ),
        result
    );
}
#[test]
fn answer_port_fault_resumes_exactly_in_a_fresh_process() {
    if let Some(path) = std::env::var_os("EKR_ANSWER_RETRY_PATH") {
        child(std::path::Path::new(&path));
        return;
    }
    for fault in [recovery::Fault::BeforeWrite, recovery::Fault::AfterWrite] {
        let directory = tempfile::tempdir().unwrap();
        recover(directory.path(), true, fault, |binding, hooks| {
            let open = |a| {
                Ok(recovery::Probe {
                    inner: FileStore::file(directory.path(), "upgrade-fixture", None)?.under(a),
                    hooks,
                })
            };
            match binding {
                Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
                None => Commit::over_with_authority(context(), anchor(), open),
            }
            .unwrap()
        });
        let directory = tempfile::tempdir().unwrap();
        recover(directory.path(), false, fault, |binding, hooks| {
            let open = |a| {
                Ok(recovery::Probe {
                    inner: SqliteStore::sqlite(
                        &directory.path().join("store.db"),
                        "upgrade-fixture",
                        None,
                    )?
                    .under(a),
                    hooks,
                })
            };
            match binding {
                Some(b) => Commit::over_with_review_authority(context(), anchor(), b, open),
                None => Commit::over_with_authority(context(), anchor(), open),
            }
            .unwrap()
        });
    }
}
