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
    let mut page = String::from(
        r#"<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Schema proposal · EKR</title><style>body{max-width:1000px;margin:2rem auto;padding:0 1rem;font:16px/1.5 system-ui;background:#121211;color:#f2f2ee}a{color:#73b3ff}pre{overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;background:#232322;padding:1rem}code{overflow-wrap:anywhere}</style><nav><a href="/inbox">Knowledge inbox</a></nav><main><h1>Schema proposal</h1>"#,
    );
    write!(page, "<p>{}</p><p>Exact proposal digest: <code>{}</code></p><p>Submit human decisions through the CLI or SDK.</p><h2>Schema additions</h2><pre>{}</pre><h2>Mapping preview</h2><pre>{}</pre><h2>Selected claim corrections</h2><pre>{}</pre>", escaped(&shown.proposal.explanation), escaped(&shown.proposal_digest.0), document(&shown.proposal.additions)?, document(&shown.preview)?, document(&shown.proposal.corrections)?).unwrap();
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
    let revision = shown
        .basis
        .observed_revision
        .0
        .as_u64()
        .ok_or("invalid proposal revision")?;
    let read = runtime
        .read(Some(RevisionNumber::new(revision)))
        .map_err(|e| e.to_string())?;
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
    write!(page, "<h2>Review material</h2><pre>{}</pre><h2>Review history</h2><pre>{}</pre><h2>Application history</h2><pre>{}</pre></main></html>", document(&shown.basis)?, document(&shown.reviews)?, document(&shown.receipts)?).unwrap();
    Ok(page.into_bytes())
}

#[cfg(test)]
mod tests {
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
