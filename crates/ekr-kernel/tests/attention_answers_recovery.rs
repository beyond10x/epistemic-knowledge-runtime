//! Port-level response loss and real process death inside native atomic answer publication.
//! Native cases replay the kernel-elected request, then resume through the kernel after reopen.
//! Process death preserves the OS page cache; these are not power-loss/fsync tests.
#![allow(dead_code, unused_imports)]
include!("support/attention_answer_fixture.rs");
#[path = "recovery/answer_native.rs"]
mod native;
#[path = "recovery/support.rs"]
mod recovery;

#[derive(serde::Serialize, serde::Deserialize)]
struct RetryPlan {
    file: bool,
    proof: Vec<u8>,
    binding: Vec<u8>,
    assertion: String,
    temporal: bool,
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
                kind: if self.temporal {
                    m::ClaimCorrectionKind::CorrectTime
                } else {
                    m::ClaimCorrectionKind::Choose
                },
                assertion_id: ekr_core::contracts::graph::AssertionId(Uuid(self.assertion.clone())),
                valid_from: self.temporal.then(|| {
                    ekr_core::contracts::primitives::Timestamp("2000-01-01T00:00:00Z".into())
                }),
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
    assert_eq!(
        kernel
            .read(None)
            .unwrap()
            .dispute_attention()
            .unwrap()
            .is_empty(),
        !plan.temporal
    );
    assert_eq!(result.replacements.len(), usize::from(plan.temporal));
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
    crash: Option<&str>,
    temporal: bool,
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
        if temporal {
            m::ClaimCorrectionKind::CorrectTime
        } else {
            m::ClaimCorrectionKind::Choose
        },
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
        temporal,
    };
    std::fs::write(path.join("retry.json"), serde_json::to_vec(&plan).unwrap()).unwrap();
    if let Some(point) = crash {
        native::crash(path, file, point, &elected);
        let reopened = open(
            Some(human.binding.clone()),
            recovery::Hooks::new(recovery::Fault::Pass),
        );
        let history = reopened.answer_history(None).unwrap();
        assert_eq!(history.len(), usize::from(point == "after"));
        assert_eq!(
            reopened
                .read(None)
                .unwrap()
                .dispute_attention()
                .unwrap()
                .is_empty(),
            point == "after" && !temporal
        );
    }
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
    let native_before_retry = crash.map(|_| native::capture(path, file, &elected));
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
    if let Some(before) = native_before_retry {
        assert_eq!(
            native::capture(path, file, &elected),
            before,
            "kernel retry must preserve all native event coordinates and blob bytes"
        );
    }
}
#[test]
fn answer_port_fault_resumes_exactly_in_a_fresh_process() {
    if let Some(path) = std::env::var_os("EKR_ANSWER_RETRY_PATH") {
        child(std::path::Path::new(&path));
        return;
    }
    for fault in [recovery::Fault::BeforeWrite, recovery::Fault::AfterWrite] {
        let directory = tempfile::tempdir().unwrap();
        recover(
            directory.path(),
            true,
            fault,
            None,
            false,
            |binding, hooks| {
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
            },
        );
        let directory = tempfile::tempdir().unwrap();
        recover(
            directory.path(),
            false,
            fault,
            None,
            false,
            |binding, hooks| {
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
            },
        );
    }
}

#[test]
fn answer_native_process_death_resumes_without_partial_corrections() {
    if let Some(path) = std::env::var_os("EKR_ANSWER_NATIVE_PATH") {
        native::child(std::path::Path::new(&path));
        return;
    }
    for point in ["before", "first", "last", "after"] {
        for file in [true, false] {
            for temporal in [false, true] {
                let directory = tempfile::tempdir().unwrap();
                macro_rules! exercise {
                    ($store:expr) => {
                        recover(
                            directory.path(),
                            file,
                            recovery::Fault::BeforeWrite,
                            Some(point),
                            temporal,
                            |binding, hooks| {
                                let open = |a| {
                                    let mut store = $store?.under(a);
                                    store.set_full_replay(true);
                                    Ok(recovery::Probe {
                                        inner: store,
                                        hooks,
                                    })
                                };
                                match binding {
                                    Some(b) => Commit::over_with_review_authority(
                                        context(),
                                        anchor(),
                                        b,
                                        open,
                                    ),
                                    None => Commit::over_with_authority(context(), anchor(), open),
                                }
                                .unwrap()
                            },
                        );
                    };
                }
                if file {
                    exercise!(FileStore::file(directory.path(), "upgrade-fixture", None));
                } else {
                    exercise!(SqliteStore::sqlite(
                        &directory.path().join("store.db"),
                        "upgrade-fixture",
                        None
                    ));
                }
                eprintln!(
                    "native answer recovery: provider={} point={point} temporal={temporal} passed",
                    if file { "file" } else { "sqlite" }
                );
            }
        }
    }
}
