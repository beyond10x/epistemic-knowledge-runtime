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
    page.push_str("</main></html>");
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
