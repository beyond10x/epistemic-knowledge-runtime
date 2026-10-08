---
format: aep.planning-md/3
id: story:search-first-viewer-entry
kind: story
status: implemented
title: Offer a search-first browser entry without graph dependencies
relations:
- serves: vision:o5
- derived_from: task:hosted-postgres-copy-and-read-serving
scope:
- confidence: inferred
  path: crates/ekr/src/cli/mod.rs
- confidence: inferred
  path: crates/ekr/src/cli/search_page.rs
- confidence: cited
  path: crates/ekr/src/cli/view.rs
- confidence: inferred
  path: crates/ekr/tests/search_page.rs
- confidence: cited
  path: docs/cli.md
- confidence: inferred
  path: docs/epistemic-knowledge-runtime-design.md
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T01:45:34Z", actor: "agent:codex", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-03T01:45:34Z", actor: "agent:codex", revision: 7}
- {from: "active", to: "implemented", at: "2026-10-03T04:58:22Z", actor: "agent:codex", revision: 10, decided_on: {"recorded":{"test_result":2}}}
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

## Final acceptance and release

The full local gate on 70761eaed64e3d5893a93d8ae3d5552c874a9987 passed: 2189 passed, 0 failed, 13 ignored across 358 completed runner summaries. All 10 recorded gate steps exited zero. Required real PostgreSQL and previous-release prerequisites were enabled; these scoped acceptance cases executed. Logs remain at `<cache>/ekr-hosted-runtime/release-gate/`. Existing ignored helper cases are not acceptance passes.

Exact-head GitHub Repository correctness and common / Security and privacy checks passed; correctness job https://github.com/beyond10x/epistemic-knowledge-runtime/actions/runs/37096359423/job/111126998509 completed successfully. Remote main and annotated 0.0.28 tag were verified at this commit. Release https://github.com/beyond10x/epistemic-knowledge-runtime/releases/tag/0.0.28 was read back as published at 2026-10-03T04:56:40Z. This fulfills the scoped source-release contract, not a consumer deployment claim.

All cases below live in `crates/ekr/tests/search_page.rs` and pass in the complete local gate.

| Acceptance line | Existing evidence and conclusion |
|---|---|
| `search_entry_works_without_graph_libraries` | `entry_has_an_accessible_get_form_without_scripts_or_external_assets` and `live::search_entry_works_without_graph_libraries_and_keeps_response_protections` prove the GET form, no external/script dependency, and response protections. No browser screenshot or assistive-technology audit is claimed. |
| `search_results_preserve_query_revision_and_evidence` | `historical_results_keep_authoritative_order_and_pin_every_link`, `results_and_visible_untrusted_fields_have_hard_bounds`, and `live::historical_search_keeps_its_results_and_retained_evidence_pinned_after_a_commit` prove ranked bounded results and revision-pinned graph/evidence links. The story's preview clause is conditional: no inline payload preview is rendered, so no preview provenance is claimed. |
| `search_html_escapes_untrusted_text` | `untrusted_text_is_escaped_and_identity_link_parameters_are_encoded` and `live::live_search_escapes_query_refuses_bad_bounds_and_matches_json_ranking` prove escaping, validated/encoded local links and input bounds; the live entry test above checks cache/CSP/content headers. |
| `search_entry_handles_empty_unavailable_and_historical_reads` | `blank_no_match_and_unavailable_states_explain_the_next_action`, `live::unavailable_search_keeps_query_in_a_useful_html_page`, and the historical live case prove useful states/statuses and retained revisions. Existing graph/view and JSON search suites pass in the combined gate; the ranking case compares the JSON API. |

Review anchor: `review-result:search-entry-final-1`, including its explicit independent execution and final equivalent-renderer delta limits. Search is names/aliases over the existing index, not natural-language answering or full-text evidence search.
