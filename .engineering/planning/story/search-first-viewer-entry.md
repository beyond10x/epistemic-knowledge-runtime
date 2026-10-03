---
format: aep.planning-md/3
id: story:search-first-viewer-entry
kind: story
status: active
title: Offer a search-first browser entry without graph dependencies
relations:
- serves: vision:o5
- derived_from: task:hosted-postgres-copy-and-read-serving
scope:
- confidence: inferred
  path: crates/ekr/src/cli/search_page.rs
- confidence: cited
  path: crates/ekr/src/cli/view.rs
- confidence: inferred
  path: crates/ekr/tests/search_page.rs
- confidence: cited
  path: docs/cli.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T01:45:34Z", actor: "agent:codex", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-03T01:45:34Z", actor: "agent:codex", revision: 7}
---
## Outcome
Offer an ontology-independent, search-first browser page at `/find`, alongside the existing graph viewer. Render HTML in Rust; use an ordinary GET form and CSS, without adding JavaScript or graph-library dependencies. This is search over existing names and aliases, not natural-language answering or full-text evidence search.

## Acceptance
- search_entry_works_without_graph_libraries: GET /find renders an accessible prominent search form and honest empty state without scripts, external fonts or CDN assets; its initial form does not need to load the graph.
- search_results_preserve_query_revision_and_evidence: GET /find?q=... searches the existing index, shows bounded results and the served revision, and links to the corresponding graph detail/history and retained evidence. Any displayed evidence preview comes from retained bytes, bounded and clearly attributed; missing retained text is not invented.
- search_html_escapes_untrusted_text: names, aliases, query text and any previews are escaped in HTML/text attributes; generated links use validated identities and encoded parameters. The page retains no-store, anti-framing and content-type protections.
- search_entry_handles_empty_unavailable_and_historical_reads: no matches and unavailable stores have useful pages with correct statuses; historical requests use one index/revision for every result and evidence reference. Existing JSON search and graph routes remain unchanged.

## Scope
Confidence medium from direct source inspection. Cited: crates/ekr/src/cli/view.rs owns page routes, indexes and evidence access; crates/ekr-views/src/query.rs already produces search and node-detail documents. Inferred new Rust renderer crates/ekr/src/cli/search_page.rs, integration tests crates/ekr/tests/search_page.rs, and docs/cli.md. Reuse current view/query authority; no new knowledge entity or generic store mechanism. No engine schema-specific terms or consumer identity enters page copy or fixtures.

## Sequencing
This follows the HTTP unit because view.rs and CLI docs overlap. Root may prepare the standalone renderer in isolation, but route wiring and integration wait for the reviewed HTTP result. The operator's existing request for a search-like staff interface and authorization to implement upstream cover this unit. This does not defer deployment indefinitely for a graph UI redesign.
