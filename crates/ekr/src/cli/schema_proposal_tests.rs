//! Real signed review through CLI dispatch, with independently provisioned embedding trust.
use super::schema_proposal::{run, SchemaProposalCommand};
use ekr_core::{contract_data as w, contracts::kernel as m, Timestamp};
use ekr_kernel::{human_review as h, Runtime};
use serde_json::json;

use super::upgrade_fixture as fixture;

fn review(
    human: &fixture::Human,
    shown: &w::EkrIntegrateSchemaProposalRead,
    approve: bool,
) -> w::EkrIntegrateSchemaProposalReviewApplication {
    let statement = b"Reviewed <proposal> evidence & additions.";
    let mut proof: w::EkrKernelSignedHumanDecision = serde_json::from_value(json!({
        "algorithm":"Ed25519","signature":ekr_core::bytes::encode(&[0;64]),
        "intent":{"format":"ekr.human-decision/1","decision_id":ekr_core::EventId::mint(),
            "audience":{"tenant":fixture::TENANT,"seed_anchor":human.binding.audience.seed_anchor.0},
            "reviewer_policy_digest":human.binding.reviewer_policy_digest.0,"signer_key_digest":human.key_digest(),
            "statement_digest":h::digest(statement).to_string(),
            "target":{"kind":if approve {"ApproveSchemaProposal"} else {"RejectSchemaProposal"},
                "value":{"proposal_id":shown.proposal.proposal_id,"proposal_digest":shown.proposal_digest,"basis":shown.basis}}}
    })).unwrap();
    proof.intent.expected_previous_decision = shown.expected_previous_decision.clone();
    human.sign(&mut proof);
    w::EkrIntegrateSchemaProposalReviewApplication {
        human_proof: Box::new(proof),
        proposal_id: shown.proposal.proposal_id.clone(),
        proposal_digest: shown.proposal_digest.clone(),
        basis: shown.basis.clone(),
        statement: ekr_core::bytes::encode(statement),
    }
}

fn prepared_schema_proposal(
    sqlite: bool,
) -> (
    tempfile::TempDir,
    Runtime,
    fixture::Human,
    w::EkrIntegrateSchemaProposalRead,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("store");
    let runtime = if sqlite {
        Runtime::sqlite(
            &path,
            fixture::TENANT,
            fixture::context(),
            fixture::anchor(),
        )
    } else {
        Runtime::file(
            &path,
            fixture::TENANT,
            fixture::context(),
            fixture::anchor(),
        )
    }
    .unwrap();
    let seeded = runtime
        .seed(fixture::seed(false), || Timestamp::EPOCH)
        .unwrap();
    let mut human = fixture::Human::new(seeded.seed_hash);
    human.policy.keys[0].scopes = vec![
        m::HumanDecisionScope::ApproveSchemaProposal,
        m::HumanDecisionScope::RejectSchemaProposal,
        m::HumanDecisionScope::UpgradeAuthority,
    ];
    human.binding.reviewer_policy_digest =
        m::ContentHash(h::digest(&h::policy_bytes(&human.policy).unwrap()).to_string());
    let runtime = runtime
        .with_review_authority(human.binding.clone())
        .unwrap();
    let preview = runtime.preview_upgrade(&human.policy).unwrap();
    runtime
        .apply_upgrade(
            &preview,
            &human.policy,
            &h::proof_from_document(&human.proof(&preview)).unwrap(),
            fixture::STATEMENT,
            || Timestamp::from_millis(1),
        )
        .unwrap();
    let read = runtime.read(None).unwrap();
    let document = json!({"proposal_id":ekr_core::NodeId::mint(),"base_schema":read.graph.ontology.version().id,
            "observations":[],"sources":[],"evidence":[read.graph.evidence.keys().next().unwrap()],
            "additions":[{"kind":"DefineType","value":{"name":"ReviewableAddition","parents":[],"abstract_type":false,"properties":[]}}],
            "mappings":[],"corrections":[],"explanation":"An evidence-backed addition."});
    let input = serde_json::from_value(json!({"proposal":document,"payload":ekr_core::bytes::encode(&serde_json::to_vec(&document).unwrap())})).unwrap();
    let shown = runtime
        .submit_schema_proposal(&input, Timestamp::from_millis(2))
        .unwrap();
    (dir, runtime, human, shown)
}

#[test]
fn cli_dispatch_records_signed_approval_and_rejection_without_advancing_head() {
    for sqlite in [false, true] {
        let (_dir, runtime, human, shown) = prepared_schema_proposal(sqlite);
        let read = runtime.read(None).unwrap();
        let approval = review(&human, &shown, true);
        let encoded = serde_json::to_vec(&approval).unwrap();
        run(
            SchemaProposalCommand::Approve {
                document: "-".into(),
            },
            &runtime,
            &|| Timestamp::from_millis(3),
            &mut encoded.as_slice(),
        )
        .unwrap();
        let shown = runtime
            .schema_proposal(&shown.proposal.proposal_id)
            .unwrap();
        assert_eq!(shown.reviews.len(), 1);
        assert_eq!(
            serde_json::to_value(&shown.reviews[0].decision).unwrap(),
            "Approved"
        );
        let rejection = serde_json::to_vec(&review(&human, &shown, false)).unwrap();
        run(
            SchemaProposalCommand::Reject {
                document: "-".into(),
            },
            &runtime,
            &|| Timestamp::from_millis(4),
            &mut rejection.as_slice(),
        )
        .unwrap();
        // Replaying the earlier approval is idempotent and cannot reinstate approval.
        run(
            SchemaProposalCommand::Approve {
                document: "-".into(),
            },
            &runtime,
            &|| Timestamp::from_millis(5),
            &mut encoded.as_slice(),
        )
        .unwrap();
        let shown = runtime
            .schema_proposal(&shown.proposal.proposal_id)
            .unwrap();
        assert_eq!(shown.reviews.len(), 2);
        assert_eq!(
            serde_json::to_value(&shown.reviews[1].decision).unwrap(),
            "Rejected"
        );
        assert!(runtime
            .attention()
            .unwrap()
            .iter()
            .all(|item| item.subject.proposal_id
                != w::EssPresence::Present(shown.proposal.proposal_id.clone())));
        let page = String::from_utf8(
            super::inbox::proposal(&runtime, &shown.proposal.proposal_id.0).unwrap(),
        )
        .unwrap();
        assert!(page.contains("Reviewed &lt;proposal&gt; evidence &amp; additions."));
        assert!(page.contains("fixture-human"));
        assert!(!page.contains("Reviewed <proposal>"));
        if let Some(output) = std::env::var_os("EKR_INBOX_INSPECTION_DIR") {
            let output = std::path::Path::new(&output);
            std::fs::create_dir_all(output).unwrap();
            std::fs::write(
                output.join(if sqlite {
                    "sqlite-schema-reviews.html"
                } else {
                    "file-schema-reviews.html"
                }),
                &page,
            )
            .unwrap();
        }
        assert_eq!(runtime.read(None).unwrap().root, read.root);
    }
}

#[test]
fn sdk_schema_application_uses_the_real_session_and_reports_the_committed_result() {
    use ekr_sdk::knowledge::Knowledge;
    for sqlite in [false, true] {
        let (dir, runtime, human, shown) = prepared_schema_proposal(sqlite);
        let before = runtime.read(None).unwrap().root;
        let config = super::Store {
            host: crate::host::CliHostConfigurationV1 {
                format: crate::host::CliHostFormatV1::V1,
                tenant: fixture::TENANT.into(),
                context: fixture::context(),
                authority: fixture::anchor(),
            },
            store: dir.path().join("store"),
            backend: if sqlite {
                super::Backend::Sqlite
            } else {
                super::Backend::File
            },
            full_replay: false,
            access: super::Access::Write,
        };
        let now = std::cell::Cell::new(3);
        let clock = || Timestamp::from_millis(now.get());
        let mut transport = super::session::InProcess::new(config, runtime, &clock);
        let mut knowledge = Knowledge::new(&mut transport);
        let approval = knowledge
            .approve_schema_proposal(&review(&human, &shown, true))
            .unwrap();
        now.set(4);
        let first = knowledge
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approval.review_id,
                &shown.proposal_digest,
            )
            .unwrap();
        assert!(!first.already_complete);
        assert_eq!(*first.progress, w::EkrIntegrateApplicationProgress::V0);
        let w::EssPresence::Present(revision) = &first.schema_revision else {
            panic!("schema commit is absent")
        };
        assert_eq!(revision.0.as_u64(), Some(before.revision.get() + 1));
        now.set(5);
        let second = knowledge
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approval.review_id,
                &shown.proposal_digest,
            )
            .unwrap();
        assert!(second.already_complete);
        assert_eq!(second.application_id, first.application_id);
        assert_eq!(second.schema_transaction, first.schema_transaction);
        let shown = knowledge
            .schema_proposal(&shown.proposal.proposal_id)
            .unwrap();
        let w::EssPresence::Present(application) = shown.application else {
            panic!("application history is absent")
        };
        assert_eq!(application.publications.len(), 3);
        assert_eq!(application.receipts.len(), 1);
        transport.close();
    }
}
