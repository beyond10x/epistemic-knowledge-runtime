//! Rust-rendered, read-only knowledge inbox. No browser-side write capability.
use ekr_core::contract_data::{EkrKernelAttentionKind, EssPresence};
use ekr_core::{AssertionId, EvidenceId, RevisionNumber};
use ekr_kernel::Runtime;
use std::fmt::Write;

fn escaped(value: impl std::fmt::Display) -> String {
    value
        .to_string()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn document(value: &impl serde::Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map(escaped)
        .map_err(|e| e.to_string())
}
fn excerpt(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => {
            let mut shown: String = text.chars().take(4096).collect();
            if text.chars().count() > 4096 {
                shown.push_str("\n[Excerpt; open the retained evidence for the complete bytes.]");
            }
            escaped(shown)
        }
        Err(_) => format!(
            "Binary evidence ({} bytes). Open the retained evidence to inspect it.",
            bytes.len()
        ),
    }
}
pub(super) fn render(runtime: &Runtime) -> Result<Vec<u8>, String> {
    let items = runtime.attention().map_err(|e| e.to_string())?;
    let mut page = String::from(
        r#"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Knowledge inbox · EKR</title><style>body{max-width:1000px;margin:2rem auto;padding:0 1rem;font:16px/1.5 system-ui;background:#121211;color:#f2f2ee}a{color:#73b3ff}article{border:1px solid #45453e;border-radius:8px;padding:1.3rem;margin:1rem 0}h1,h2,h3{line-height:1.2}pre{overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;background:#232322;padding:1rem}code{overflow-wrap:anywhere}small{color:#c3c2b7}summary{cursor:pointer}section{border-top:1px solid #45453e;margin-top:1rem}</style><nav><a href="/">Knowledge graph</a></nav><main><h1>Knowledge inbox</h1><p>Review competing claims and knowledge waiting for integration. Decisions are submitted through the CLI or SDK.</p>"#,
    );
    if items.is_empty() {
        page.push_str("<p>No unresolved knowledge questions.</p>");
    }
    for item in items {
        let (kind, id) = match (
            &*item.subject.kind,
            &item.subject.dispute_id,
            &item.subject.blocker_id,
            &item.subject.proposal_id,
        ) {
            (EkrKernelAttentionKind::V1, EssPresence::Present(id), _, _) => ("dispute", &id.0),
            (EkrKernelAttentionKind::V0, _, EssPresence::Present(id), _) => {
                ("blocked-integration", &id.0)
            }
            (EkrKernelAttentionKind::V2, _, _, EssPresence::Present(id)) => {
                ("schema-proposal", &id.0)
            }
            _ => return Err("attention subject has no matching identity".into()),
        };
        write!(page, "<article id=\"{}\"><small>{}</small><h2>{}</h2><p><code>ekr attention show {} {}</code></p>", escaped(id), kind, escaped(&item.question), kind, escaped(id)).unwrap();
        if matches!(*item.subject.kind, EkrKernelAttentionKind::V2) {
            write!(page, "<p><a href=\"/schema-proposal/{}\">Review schema additions, mappings and evidence</a></p>", escaped(id)).unwrap();
        }
        if !item.claims.is_empty() {
            let revision = item
                .basis
                .observed_revision
                .0
                .as_u64()
                .ok_or("invalid attention revision")?;
            // Read the exact question revision, even if another writer has advanced the store.
            let read = runtime
                .read(Some(RevisionNumber::new(revision)))
                .map_err(|e| e.to_string())?;
            for id in &item.claims {
                let id: AssertionId =
                    id.0.parse()
                        .map_err(|e: ekr_core::IdParseError| e.to_string())?;
                let claim = read
                    .graph
                    .assertions
                    .get(&id)
                    .ok_or("missing disputed claim")?;
                write!(page, "<section><h3>Claim <code>{id}</code></h3>").unwrap();
                if let ekr_graph::Object::Node(target) = &claim.object {
                    let node = read
                        .graph
                        .nodes
                        .get(&target.id())
                        .ok_or("missing disputed relation target")?;
                    write!(page, "<p>Target: {}</p>", escaped(&node.canonical_name)).unwrap();
                }
                write!(page, "<pre>{}</pre><p>Effective time: <code>{}</code></p><p>Assessment: {}</p><p>Supporting evidence:</p><ul>", document(&claim.object)?, escaped(serde_json::to_string(&claim.valid_time).map_err(|e| e.to_string())?), escaped(claim.assessment.name())).unwrap();
                let mut evidence: std::collections::BTreeSet<_> =
                    claim.evidence.iter().map(|e| e.id()).collect();
                evidence.extend(read.graph.attached(id).map(|a| a.evidence.id()));
                for id in evidence {
                    write!(
                        page,
                        "<li><a href=\"/evidence/{id}\"><code>{id}</code></a></li>"
                    )
                    .unwrap();
                }
                page.push_str("</ul></section>");
            }
            for id in &item.evidence {
                let id: EvidenceId =
                    id.0.parse()
                        .map_err(|e: ekr_core::IdParseError| e.to_string())?;
                let evidence = read
                    .graph
                    .evidence
                    .get(&id)
                    .ok_or("missing question evidence")?;
                let bytes = read
                    .content(&evidence.content_hash)
                    .ok_or("missing retained question bytes")?;
                write!(page, "<details><summary>Evidence {id}</summary><pre>{}</pre><pre>{}</pre><a href=\"/evidence/{id}\">Open retained bytes</a></details>", document(evidence)?, excerpt(bytes)).unwrap();
            }
        }
        if matches!(*item.subject.kind, EkrKernelAttentionKind::V0) {
            for version in runtime.interpretations().map_err(|e| e.to_string())? {
                let held = runtime
                    .interpretation(&version)
                    .map_err(|e| e.to_string())?;
                if held.blockers.iter().any(|b| &b.blocker_id.0 == id) {
                    write!(page, "<details><summary>Parked knowledge and integration gaps</summary><pre>{}</pre></details>", document(&held)?).unwrap();
                    break;
                }
            }
            for id in &item.observations {
                let observation = runtime
                    .observation(
                        id.0.parse()
                            .map_err(|e: ekr_core::IdParseError| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                let bytes =
                    ekr_core::bytes::decode(&observation.payload).map_err(|e| e.to_string())?;
                write!(page, "<details><summary>Source observation {}</summary><pre>{}</pre><pre>{}</pre></details>", escaped(&id.0), document(&observation.observation)?, excerpt(&bytes)).unwrap();
            }
        }
        write!(
            page,
            "<details><summary>Review basis</summary><pre>{}</pre></details></article>",
            document(&item.basis)?
        )
        .unwrap();
    }
    let answers = runtime.answer_history(None).map_err(|e| e.to_string())?;
    if !answers.is_empty() {
        page.push_str("<h2>Answer history</h2><p>Reviewed decisions remain available after their questions are settled.</p>");
    }
    for answer in answers {
        let revision = answer
            .result
            .revision
            .0
            .as_u64()
            .ok_or("invalid answer revision")?;
        let read = runtime
            .read(Some(RevisionNumber::new(revision)))
            .map_err(|e| e.to_string())?;
        let evidence: EvidenceId = answer
            .statement_evidence
            .0
            .parse()
            .map_err(|e: ekr_core::IdParseError| e.to_string())?;
        let entry = read
            .graph
            .evidence
            .get(&evidence)
            .ok_or("missing human statement evidence")?;
        let statement = read
            .content(&entry.content_hash)
            .ok_or("missing human statement bytes")?;
        let outcome = match *answer.receipt.outcome {
            ekr_core::contract_data::EkrKernelAnswerOutcome::V0 => "Already applied",
            ekr_core::contract_data::EkrKernelAnswerOutcome::V1 => "Partially resolved",
            ekr_core::contract_data::EkrKernelAnswerOutcome::V2 => "Resolved",
            ekr_core::contract_data::EkrKernelAnswerOutcome::V3 => "Unresolved",
        };
        write!(page, "<article id=\"answer-{}\"><h3>{outcome}</h3><p>Reviewed by {} · {} · revision {revision}</p><pre>{}</pre><p><a href=\"/evidence/{evidence}\">Human statement evidence</a></p><details><summary>Corrections and effective times</summary><pre>{}</pre></details><details><summary>Signed decision and provenance</summary><pre>{}</pre></details></article>",
            escaped(&answer.answer_id.0), escaped(&answer.review.operator.authentication_subject), escaped(answer.review.recorded_at.0.format(&time::format_description::well_known::Rfc3339).map_err(|error| error.to_string())?), excerpt(statement), document(&answer.corrections)?, document(&answer)?).unwrap();
    }
    if let Some(head) = runtime.head().map_err(|e| e.to_string())? {
        let history = runtime
            .schema_history(head.revision)
            .map_err(|e| e.to_string())?;
        let entries = ekr_views::schema_evidence(&history).map_err(|e| e.to_string())?;
        if !entries.is_empty() {
            page.push_str("<section id=\"schema-history\"><h2>Schema evidence history</h2><p>Sources cited by the transactions that introduced these schema versions.</p>");
            for entry in entries {
                write!(
                    page,
                    "<article><h3>Schema version {}</h3><p>Revision {} · transaction {}</p>",
                    escaped(&entry.schema_version.0),
                    escaped(&entry.revision.0),
                    escaped(&entry.transaction_id.0)
                )
                .unwrap();
                for id in entry.evidence {
                    let evidence_id: EvidenceId =
                        id.0.parse()
                            .map_err(|e: ekr_core::IdParseError| e.to_string())?;
                    let evidence = history
                        .graph
                        .evidence
                        .get(&evidence_id)
                        .ok_or("missing schema evidence")?;
                    let bytes = runtime
                        .content(&evidence.content_hash)
                        .map_err(|e| e.to_string())?
                        .ok_or("missing retained schema evidence bytes")?;
                    write!(page, "<details><summary>Evidence {evidence_id}</summary><pre>{}</pre><pre>{}</pre><a href=\"/evidence/{evidence_id}\">Open retained bytes</a></details>", document(evidence)?, excerpt(&bytes)).unwrap();
                }
                page.push_str("</article>");
            }
            page.push_str("</section>");
        }
    }
    page.push_str("</main></html>");
    Ok(page.into_bytes())
}

pub(super) fn proposal(runtime: &Runtime, id: &str) -> Result<Vec<u8>, String> {
    use ekr_core::generated_identity::{Identity, SchemaProposalId};
    let id = SchemaProposalId::parse_identity(id).map_err(|e| e.to_string())?;
    let shown = runtime.schema_proposal(&id).map_err(|e| e.to_string())?;
    let revision = shown
        .basis
        .observed_revision
        .0
        .as_u64()
        .ok_or("invalid proposal revision")?;
    let read = runtime
        .read(Some(RevisionNumber::new(revision)))
        .map_err(|e| e.to_string())?;
    let mut page = String::from(
        r#"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Schema proposal · EKR</title><style>body{max-width:1000px;margin:2rem auto;padding:0 1rem;font:16px/1.5 system-ui;background:#121211;color:#f2f2ee}a{color:#73b3ff}pre{overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;background:#232322;padding:1rem}code{overflow-wrap:anywhere}</style><nav><a href="/inbox">Knowledge inbox</a></nav><main><h1>Schema proposal</h1>"#,
    );
    write!(page, "<p>{}</p><p>Exact proposal digest: <code>{}</code></p><p>Submit human decisions through the CLI or SDK.</p>", escaped(&shown.proposal.explanation), escaped(&shown.proposal_digest.0)).unwrap();
    application_progress(&mut page, &shown, &read);
    write!(page, "<h2>Schema additions</h2><pre>{}</pre><h2>Mapping preview</h2><pre>{}</pre><h2>Selected claim corrections</h2><pre>{}</pre>", document(&shown.proposal.additions)?, document(&shown.preview)?, document(&shown.proposal.corrections)?).unwrap();
    let mut observations: std::collections::BTreeSet<_> = shown
        .proposal
        .observations
        .iter()
        .map(|id| id.0.clone())
        .collect();
    let mut retained_evidence = std::collections::BTreeMap::new();
    page.push_str("<h2>Source interpretations</h2>");
    for source in &shown.proposal.sources {
        let held = runtime
            .interpretation(&source.version)
            .map_err(|e| e.to_string())?;
        observations.extend(held.document.observations.iter().map(|id| id.0.clone()));
        for evidence in &held.document.evidence {
            retained_evidence.insert(evidence.evidence.id.0.clone(), evidence.clone());
        }
        write!(page, "<details><summary>Retained interpretation {}</summary><pre>{}</pre><pre>{}</pre></details>", escaped(&source.version.interpretation_id.0), document(source)?, document(&held)?).unwrap();
    }
    page.push_str("<h2>Supporting evidence</h2>");
    let evidence_ids: std::collections::BTreeSet<_> = shown
        .proposal
        .evidence
        .iter()
        .map(|id| id.0.clone())
        .chain(retained_evidence.keys().cloned())
        .collect();
    for id in evidence_ids {
        if let Some(evidence) = retained_evidence.get(&id) {
            let bytes = ekr_core::bytes::decode(&evidence.payload).map_err(|e| e.to_string())?;
            write!(page, "<details><summary>Retained evidence {}</summary><pre>{}</pre><pre>{}</pre></details>", escaped(&id), document(&evidence.evidence)?, excerpt(&bytes)).unwrap();
        }
        let parsed: EvidenceId = id
            .parse()
            .map_err(|e: ekr_core::IdParseError| e.to_string())?;
        if let Some(evidence) = read.graph.evidence.get(&parsed) {
            if retained_evidence.contains_key(&id) {
                page.push_str("<p>This identifier appears in both retained and canonical evidence; both records are shown.</p>");
            }
            let bytes = read
                .content(&evidence.content_hash)
                .ok_or("missing supporting bytes")?;
            write!(page, "<details><summary>Canonical evidence {id}</summary><pre>{}</pre><pre>{}</pre><a href=\"/evidence/{id}\">Open retained bytes</a></details>", document(evidence)?, excerpt(bytes)).unwrap();
        } else if !retained_evidence.contains_key(&id) {
            return Err("missing supporting evidence".into());
        }
    }
    page.push_str("<h2>Supporting observations</h2>");
    for id in observations {
        let observation = runtime
            .observation(
                id.parse()
                    .map_err(|e: ekr_core::IdParseError| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        let bytes = ekr_core::bytes::decode(&observation.payload).map_err(|e| e.to_string())?;
        write!(
            page,
            "<details><summary>Source observation {}</summary><pre>{}</pre><pre>{}</pre></details>",
            escaped(id),
            document(&observation.observation)?,
            excerpt(&bytes)
        )
        .unwrap();
    }
    write!(
        page,
        "<h2>Review material</h2><pre>{}</pre><h2 id=\"review-history\">Review history</h2>",
        document(&shown.basis)?
    )
    .unwrap();
    for review in runtime
        .schema_proposal_reviews(&id)
        .map_err(|e| e.to_string())?
    {
        let bytes =
            ekr_core::bytes::decode(&review.statement.payload).map_err(|e| e.to_string())?;
        let decision = match *review.review.decision {
            ekr_core::contract_data::EkrIntegrateReviewDecision::V0 => "Approved",
            ekr_core::contract_data::EkrIntegrateReviewDecision::V1 => "Rejected",
        };
        write!(page, "<article><h3>{decision}</h3><p>Reviewed by {}</p><pre>{}</pre><details><summary>Signed decision and statement provenance</summary><pre>{}</pre></details></article>", escaped(&review.review.operator.authentication_subject), excerpt(&bytes), document(&review)?).unwrap();
    }
    page.push_str("<h2>Application history</h2>");
    if let EssPresence::Present(application) = &shown.application {
        write!(
            page,
            "<details><summary>Retained application records</summary><pre>{}</pre></details>",
            document(application)?
        )
        .unwrap();
    }
    write!(page,"<details><summary>Earlier progress receipts</summary><pre>{}</pre></details></main></html>", document(&shown.receipts)?).unwrap();
    Ok(page.into_bytes())
}

fn application_progress(
    page: &mut String,
    shown: &ekr_core::contract_data::EkrIntegrateSchemaProposalRead,
    read: &ekr_kernel::VerifiedRead,
) {
    use ekr_core::contract_data::{
        EkrIntegrateApplicationStepKind as Step, EkrStorePublicationCommandKind as Command,
    };
    page.push_str("<section id=\"application-progress\"><h2>Application progress</h2>");
    let EssPresence::Present(application) = &shown.application else {
        page.push_str("<p>Not applied.</p></section>");
        return;
    };
    // A Commit command can become Stale. Only its exact retained commit event is progress.
    let commits: Vec<_> = application
        .publications
        .iter()
        .filter_map(|publication| {
            if *publication.command != Command::V2 {
                return None;
            }
            let id = publication
                .transaction_id
                .0
                .parse::<ekr_core::TransactionId>()
                .ok()?;
            let commit = read.transactions.get(&id)?.committed.as_ref()?;
            (commit.event_id.to_string() == publication.event_id.0).then_some((publication, commit))
        })
        .collect();
    let schema = commits.iter().find(|(p, _)| *p.guard.step.kind == Step::V2);
    // Proposal material and application state can be captured on opposite sides of a commit.
    // Derive every displayed outcome from the same verified revision used for item labels.
    let remaining = application.election.selected_items.iter().any(|item| {
        !commits.iter().any(|(publication, _)| {
            *publication.guard.step.kind == Step::V1
                && matches!(&publication.guard.step.item, EssPresence::Present(key) if key == item)
        })
    });
    let corrections_pending = !shown.proposal.corrections.is_empty()
        && !commits.iter().any(|(p, _)| *p.guard.step.kind == Step::V0);
    let status = if schema.is_none() {
        "Waiting for the schema change to commit."
    } else if !remaining && !corrections_pending {
        "Complete."
    } else {
        "Partly applied."
    };
    write!(page, "<p><strong>{status}</strong></p>").unwrap();
    if let Some((_, commit)) = schema {
        write!(
            page,
            "<p>Schema committed at revision {}.</p>",
            commit.result.revision.get()
        )
        .unwrap();
    }
    if !application.election.selected_items.is_empty() {
        page.push_str("<ul aria-label=\"Selected knowledge\">");
        for item in &application.election.selected_items {
            let committed = commits.iter().find(|(publication,_)| {
                *publication.guard.step.kind == Step::V1
                    && matches!(&publication.guard.step.item, EssPresence::Present(key) if key == item)
            });
            write!(
                page,
                "<li><strong>{}</strong> from interpretation <code>{}</code>, version {} — ",
                escaped(&item.item),
                escaped(&item.source.interpretation_id.0),
                escaped(&item.source.version)
            )
            .unwrap();
            if let Some((_, commit)) = committed {
                write!(
                    page,
                    "Integrated at revision {}.",
                    commit.result.revision.get()
                )
                .unwrap();
            } else {
                page.push_str("Pending integration.");
            }
            page.push_str("</li>");
        }
        page.push_str("</ul>");
    }
    if corrections_pending {
        page.push_str("<p>Selected claim corrections are still pending.</p>");
    } else if !shown.proposal.corrections.is_empty() {
        page.push_str("<p>Selected claim corrections have committed.</p>");
    }
    page.push_str("</section>");
}

#[cfg(test)]
mod tests {
    #[test]
    fn proposal_progress_uses_committed_events_and_keeps_read_only_history() {
        use super::super::upgrade_fixture as f;
        use ekr_core::{bytes, contract_data as w, contracts::kernel as m, Timestamp};
        use ekr_kernel::{human_review as review, Runtime};
        use serde_json::json;
        for (sqlite, corrections) in [(false, false), (true, false), (false, true), (true, true)] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("store");
            let open = || {
                if sqlite {
                    Runtime::sqlite(&path, f::TENANT, f::context(), f::anchor()).unwrap()
                } else {
                    Runtime::file(&path, f::TENANT, f::context(), f::anchor()).unwrap()
                }
            };
            let runtime = open();
            let seeded = runtime.seed(f::seed(false), || Timestamp::EPOCH).unwrap();
            let mut human = f::Human::new(seeded.seed_hash);
            human.policy.keys[0]
                .scopes
                .insert(0, m::HumanDecisionScope::ApproveSchemaProposal);
            human.binding.reviewer_policy_digest = m::ContentHash(
                review::digest(&review::policy_bytes(&human.policy).unwrap()).to_string(),
            );
            let runtime = runtime
                .with_review_authority(human.binding.clone())
                .unwrap();
            let preview = runtime.preview_upgrade(&human.policy).unwrap();
            runtime
                .apply_upgrade(
                    &preview,
                    &human.policy,
                    &review::proof_from_document(&human.proof(&preview)).unwrap(),
                    f::STATEMENT,
                    || Timestamp::from_millis(1),
                )
                .unwrap();
            let read = runtime.read(None).unwrap();
            let evidence = read.graph.evidence.keys().next().unwrap();
            let corrections = if corrections {
                vec![
                    json!({"kind":"Retract","assertion_id":read.graph.assertions.keys().next().unwrap(),
                    "reason":"Withdraw the ambiguous claim while preserving its history."}),
                ]
            } else {
                vec![]
            };
            let document = json!({"proposal_id":ekr_core::NodeId::mint(),"base_schema":read.graph.ontology.version().id,
                "observations":[],"sources":[],"evidence":[evidence],
                "additions":[{"kind":"DefineType","value":{"name":"ReviewedVocabulary","parents":[],"abstract_type":false,"properties":[]}}],
                "mappings":[],"corrections":corrections,"explanation":"Keep <script>synthetic()</script> as evidence text."});
            let input = serde_json::from_value(json!({"proposal":document,"payload":bytes::encode(&serde_json::to_vec(&document).unwrap())})).unwrap();
            let shown = runtime
                .submit_schema_proposal(&input, Timestamp::from_millis(2))
                .unwrap();
            let id = &shown.proposal.proposal_id.0;
            let before = String::from_utf8(super::proposal(&runtime, id).unwrap()).unwrap();
            assert!(before.contains("Not applied."));
            let statement = b"Approve the displayed additions.";
            let mut proof: w::EkrKernelSignedHumanDecision = serde_json::from_value(json!({
                "algorithm":"Ed25519","signature":bytes::encode(&[0;64]),
                "intent":{"format":"ekr.human-decision/1","decision_id":ekr_core::EventId::mint(),
                "audience":{"tenant":f::TENANT,"seed_anchor":human.binding.audience.seed_anchor.0},
                "reviewer_policy_digest":human.binding.reviewer_policy_digest.0,"signer_key_digest":human.key_digest(),
                "statement_digest":review::digest(statement).to_string(),
                "target":{"kind":"ApproveSchemaProposal","value":{"proposal_id":shown.proposal.proposal_id,
                    "proposal_digest":shown.proposal_digest,"basis":shown.basis}}}
            })).unwrap();
            human.sign(&mut proof);
            let approval = runtime.approve_schema_proposal(&serde_json::from_value(json!({
                "human_proof":proof,"proposal_id":shown.proposal.proposal_id,"proposal_digest":shown.proposal_digest,
                "basis":shown.basis,"statement":bytes::encode(statement)
            })).unwrap(), Timestamp::from_millis(3)).unwrap();
            runtime
                .apply_schema_proposal(
                    &shown.proposal.proposal_id,
                    &approval.review_id,
                    &shown.proposal_digest,
                    Timestamp::from_millis(4),
                )
                .unwrap();
            drop(runtime);
            let mut reopened = open().with_review_authority(human.binding).unwrap();
            reopened.set_full_replay(true);
            let events = reopened.published_events().unwrap();
            let page = String::from_utf8(super::proposal(&reopened, id).unwrap()).unwrap();
            assert!(page.contains("<strong>Complete.</strong>"), "{page}");
            assert!(page.contains("Schema committed at revision 2."), "{page}");
            assert!(page.contains("Retained application records"));
            assert!(page.contains("Earlier progress receipts"));
            assert!(page.contains("&lt;script&gt;synthetic()&lt;/script&gt;"));
            assert!(!page.contains("<form") && !page.contains("<script>"));
            assert_eq!(reopened.published_events().unwrap(), events);
            let mut shown = reopened
                .schema_proposal(&shown.proposal.proposal_id)
                .unwrap();
            if !corrections.is_empty() {
                // A concurrent commit can advance application state after the basis capture.
                // A later application document cannot claim progress beyond this real read.
                let schema_only = reopened
                    .read(Some(ekr_core::RevisionNumber::new(2)))
                    .unwrap();
                let mut earlier = String::new();
                super::application_progress(&mut earlier, &shown, &schema_only);
                assert!(earlier.contains("Partly applied."), "{earlier}");
                assert!(earlier.contains("Selected claim corrections are still pending."));
                assert!(!earlier.contains("Selected claim corrections have committed."));
            }
            let w::EssPresence::Present(application) = &mut shown.application else {
                panic!("retained application");
            };
            for publication in &mut application.publications {
                *publication.event_id = w::EkrKernelEventId(ekr_core::EventId::mint().to_string());
            }
            let mut pending = String::new();
            super::application_progress(&mut pending, &shown, &reopened.read(None).unwrap());
            assert!(pending.contains("Waiting for the schema change to commit."));
            assert!(!pending.contains("Complete."));
            if let Some(directory) =
                std::env::var_os("EKR_INBOX_CAPTURE_DIR").filter(|_| corrections.is_empty())
            {
                std::fs::create_dir_all(&directory).unwrap();
                std::fs::write(
                    std::path::PathBuf::from(directory).join(if sqlite {
                        "sqlite-applied-proposal.html"
                    } else {
                        "file-applied-proposal.html"
                    }),
                    page,
                )
                .unwrap();
            }
        }
    }

    #[test]
    fn retained_text_never_becomes_active_html() {
        let text = super::excerpt(b"<script>alert('fixture')</script>&\"<img src=x>");
        assert!(!text.contains('<'));
        assert!(text.contains("&lt;script&gt;"));
        assert!(text.contains("&#39;fixture&#39;"));
        assert!(text.contains("&amp;&quot;"));
        assert!(super::excerpt(&[0xff, 0]).starts_with("Binary evidence"));
        assert!(super::excerpt("界".repeat(5000).as_bytes()).contains("[Excerpt;"));
    }
}
