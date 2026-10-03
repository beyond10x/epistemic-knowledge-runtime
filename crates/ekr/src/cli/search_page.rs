//! Server-rendered search entry; the route supplies results in index order.

use std::fmt::Write as _;

pub(super) const RESULT_LIMIT: usize = 20;
pub(super) const EVIDENCE_LIMIT: usize = 3;
const LABEL_LIMIT: usize = 256;
pub(super) const QUERY_LIMIT: usize = 2048;

pub(super) struct EvidenceLink {
    pub id: String,
    pub label: String,
}

pub(super) struct SearchResult {
    pub id: String,
    pub name: String,
    pub alias: Option<String>,
    pub evidence: Vec<EvidenceLink>,
}

pub(super) enum State<'a> {
    Initial,
    Results {
        matches: &'a [SearchResult],
        total: u64,
    },
    Unavailable,
    MissingRevision,
}

pub(super) struct Page<'a> {
    pub guide_url: &'a str,
    pub query: &'a str,
    pub revision: Option<u64>,
    pub requested_revision: Option<u64>,
    pub state: State<'a>,
}

pub(super) fn render(page: &Page<'_>) -> String {
    let mut html = String::from(START);
    html.push_str("<header><a class=\"brand\" href=\"/find\">EKR</a><span>Knowledge search</span></header><main><h1>Find what you know.</h1><p class=\"intro\">Search names and aliases, then follow the evidence.</p>");
    html.push_str("<p><a href=\"");
    escaped(&mut html, page.guide_url, usize::MAX);
    html.push_str("\">Connect an agent</a></p>");
    html.push_str("<form action=\"/find\" method=\"get\"><label for=\"query\">Search names and aliases</label><div class=\"search-box\"><input id=\"query\" type=\"search\" name=\"q\" maxlength=\"2048\" placeholder=\"Enter a name or alias\" value=\"");
    escaped(&mut html, page.query, QUERY_LIMIT);
    html.push_str("\"><button type=\"submit\">Search</button></div>");
    if let Some(revision) = page.requested_revision {
        write!(
            html,
            "<input type=\"hidden\" name=\"revision\" value=\"{revision}\">"
        )
        .unwrap();
    }
    html.push_str("<p class=\"hint\">Matches parts of names and aliases, including differences in letter case. Evidence text is not searched.</p></form>");
    if let Some(revision) = page.revision {
        write!(html, "<nav aria-label=\"Store revision\"><span>Revision {revision}</span><a href=\"/#revision={revision}\">Explore graph</a>").unwrap();
        if page.requested_revision.is_some() {
            html.push_str("<span>Historical view</span><a href=\"/find?q=");
            escaped(&mut html, &parameter(page.query, QUERY_LIMIT), usize::MAX);
            html.push_str("\">Search latest revision</a>");
        }
        html.push_str("</nav>");
    }
    html.push_str("<section aria-label=\"Search results\">");
    match &page.state {
        State::Initial => html.push_str("<div class=\"empty\"><h2>Start with a name.</h2><p>Enter a name or alias to find records in this store.</p></div>"),
        State::Unavailable => html.push_str("<div class=\"empty\" role=\"status\"><h2>Search is unavailable</h2><p>The store could not be read. Your query is kept above; try again when the store is available.</p></div>"),
        State::MissingRevision => html.push_str("<div class=\"empty\"><h2>This revision is unavailable</h2><p>The store does not hold the requested revision. <a href=\"/find\">Search the latest revision</a> or choose an earlier revision.</p></div>"),
        State::Results { matches: [], .. } => {
            html.push_str("<div class=\"empty\"><h2>No matches</h2><p>Try part of a name or an alias. This search does not answer questions or search evidence text.</p></div>");
        }
        State::Results { matches, total } => {
            let shown = matches.len().min(RESULT_LIMIT);
            if *total > shown as u64 {
                write!(html, "<h2>Showing {shown} of {total} results</h2><p class=\"hint\">Refine your search to narrow the results.</p>").unwrap();
            } else {
                write!(html, "<h2>{total} {}</h2>", if *total == 1 { "result" } else { "results" }).unwrap();
            }
            html.push_str("<ol class=\"results\">");
            for result in matches.iter().take(RESULT_LIMIT) {
                html.push_str("<li class=\"result\"><h3>");
                let linked = page.revision.is_some() && result.id.len() <= 128;
                if let Some(revision) = page.revision.filter(|_| linked) {
                    write!(html, "<a href=\"/#revision={revision}&amp;node={}\">", parameter(&result.id, 128)).unwrap();
                }
                escaped(&mut html, &result.name, LABEL_LIMIT);
                if linked {
                    html.push_str("</a>");
                }
                html.push_str("</h3>");
                if let Some(alias) = &result.alias {
                    html.push_str("<p class=\"alias\">Matched alias: ");
                    escaped(&mut html, alias, LABEL_LIMIT);
                    html.push_str("</p>");
                }
                html.push_str("<p class=\"hint\">Open graph details to inspect this record and its history.</p>");
                if let Some(revision) = page.revision {
                    for evidence in result.evidence.iter().filter(|item| item.id.len() <= 128).take(EVIDENCE_LIMIT) {
                        write!(html, "<a class=\"evidence\" href=\"/evidence/{}?revision={revision}\">", parameter(&evidence.id, 128)).unwrap();
                        escaped(&mut html, &evidence.label, LABEL_LIMIT);
                        html.push_str("</a>");
                    }
                }
                html.push_str("</li>");
            }
            html.push_str("</ol>");
        }
    }
    html.push_str("</section></main><footer>Knowledge carries evidence. Open a record to see what supports it.</footer></body></html>");
    html
}

fn escaped(out: &mut String, text: &str, limit: usize) {
    let mut chars = text.chars();
    for ch in chars.by_ref().take(limit) {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    if chars.next().is_some() {
        out.push('…');
    }
}

fn parameter(text: &str, limit: usize) -> String {
    let mut out = String::new();
    for ch in text.chars().take(limit) {
        let mut bytes = [0; 4];
        for byte in ch.encode_utf8(&mut bytes).bytes() {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                out.push(char::from(byte));
            } else {
                write!(out, "%{byte:02X}").unwrap();
            }
        }
    }
    out
}

const START: &str = r#"<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>Search knowledge · EKR</title><link rel="describedby" href="/llms.txt">
<style>
:root{color-scheme:light;--ink:#183532;--muted:#526560;--line:#dce4de;--paper:#fafbf8;--accent:#185f4b}
*{box-sizing:border-box}body{margin:0;background:var(--paper);color:var(--ink);font:1rem/1.6 system-ui,sans-serif}a{color:var(--accent);text-underline-offset:.2em}a:hover{text-decoration-thickness:2px}header,main,footer{width:min(100% - 3rem,52rem);margin:auto}header{display:flex;gap:1rem;align-items:center;padding:2rem 0;color:var(--muted);font-size:.875rem}.brand{font-weight:800;letter-spacing:.08em;text-decoration:none}main{padding:3rem 0}h1{font-size:clamp(2.2rem,6vw,3.5rem);line-height:1.15;letter-spacing:-.045em;margin:0 0 1rem}h2{font-size:1.15rem}h3{font-size:1.25rem;margin:0;overflow-wrap:anywhere}.intro{font-size:1.15rem;color:var(--muted);margin:0 0 2rem}label{display:block;font-weight:650;margin-bottom:.6rem}.search-box{display:flex;gap:.6rem}input,button{font:inherit;border-radius:.65rem}input{min-width:0;flex:1;border:1px solid #9caf9f;padding:.9rem 1rem;background:white;color:var(--ink)}button{border:1px solid var(--accent);background:var(--accent);color:white;font-weight:650;padding:.9rem 1.4rem;cursor:pointer}:focus-visible{outline:3px solid #c58229;outline-offset:3px}.hint{font-size:.85rem;color:var(--muted);margin:.65rem 0}nav{display:flex;align-items:center;flex-wrap:wrap;gap:.5rem 1rem;border-top:1px solid var(--line);border-bottom:1px solid var(--line);padding:.9rem 0;margin:1.6rem 0;font-size:.85rem;color:var(--muted)}.results{list-style:none;padding:0}.result{padding:1.3rem 0;border-bottom:1px solid var(--line)}.alias{font-size:.9rem;margin:.4rem 0;overflow-wrap:anywhere}.evidence{display:inline-block;margin:.5rem 1rem 0 0;font-size:.85rem;overflow-wrap:anywhere}.empty{padding:2.2rem 0;max-width:40rem}.empty p{color:var(--muted)}footer{padding:1.5rem 0 2rem;border-top:1px solid var(--line);font-size:.8rem;color:var(--muted)}@media(max-width:480px){header,main,footer{width:calc(100% - 2rem)}main{padding:1.5rem 0}.search-box{flex-direction:column}button{width:100%}}
</style></head><body>"#;
