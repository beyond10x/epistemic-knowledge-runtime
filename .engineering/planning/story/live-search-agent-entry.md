---
format: aep.planning-md/3
id: story:live-search-agent-entry
kind: story
status: active
title: Live search results and discoverable agent connection guidance
relations:
- serves: vision:o5
- derived_from: story:search-first-viewer-entry
scope:
- confidence: inferred
  path: crates/ekr/src/cli/agent_help.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: crates/ekr/src/cli/search_page.rs
- confidence: cited
  path: crates/ekr/src/cli/view.rs
- confidence: cited
  path: crates/ekr/tests/agent_cli.rs
- confidence: inferred
  path: crates/ekr/tests/search_live.rs
- confidence: cited
  path: crates/ekr/tests/search_page.rs
- confidence: cited
  path: crates/ekr/tests/view_cli.rs
- confidence: cited
  path: crates/ekr/tests/view_page.rs
- confidence: cited
  path: docs/cli.md
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T09:01:36Z", actor: "agent:codex", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-03T09:01:37Z", actor: "agent:codex", revision: 5}
---
## Outcome and authorization
The operator requests live results while typing on the search entry, a visible link explaining agent MCP connection, and an agent-readable llms.txt entry. Implement and publish the generic capability under the standing upstream authorization. Preserve ordinary GET search and the existing read-only serving contract. The browser-language exception is pending: Rust remains required until the operator explicitly authorizes a narrow self-contained JavaScript enhancement. Static guidance and Rust rendering can proceed independently.

## Acceptance
A browser user receives correctly ordered current-query results while typing without losing input focus, can open explicit MCP connection instructions from the search page, and an agent following llms.txt reaches accurate data-free guidance; all existing no-script form, revision, escaping and HTTP admission behavior remains valid.

## Named behavioral cases
- live_typing_preserves_focus: actual browser input updates only the result/revision region without submit or navigation; caret and focus survive.
- superseded_query_cannot_win: deliberately reorder controlled responses, cancel superseded reads and verify only the current query renders, without wall-clock performance assertions.
- composition_clear_and_fallback: composition does not issue partial queries, clearing removes stale results, Enter works, and scripting-disabled ordinary GET remains usable.
- historical_links_stay_pinned: reuse existing Rust-rendered ranking, limits and revision-pinned graph/evidence links; no new search semantics or evidence-text indexing.
- failed_read_is_honest: unavailable/failed reads retain the typed query and explicitly clear or mark stale output, with bounded retry behavior and no background loop.
- connection_metadata_is_explicit: a configured absolute MCP URL is advertised exactly; omission does not infer that the viewer's origin hosts MCP. URL validation rejects executable schemes and embedded credentials; HTML, Markdown and displayed shell examples remain safely escaped.
- guide_survives_store_failure: static connection guidance and llms.txt pass authority/method checks and work without opening a knowledge store; no records, inventories or evidence exports appear.
- browser_assets_are_local: no CDN/framework/analytics addition, no broad CSP relaxation; if the narrow language exception is approved, allow only the exact self-contained script via a CSP hash and same-origin connections.

## Scope and boundary
Read-only scoper report cites cli/search_page.rs rendering and view.rs find(), cli/mod.rs viewer options, tests/search_page.rs, the existing Rust-driven browser harness in tests/view_page.rs, CLI argument tests and docs/cli.md. Inferred new helper: a small Rust agent-help renderer. No kernel, provider, MCP dispatch or stored schema change; existing view/search concepts are already declared in the ESS views domain. No new persistent entity is introduced.

The implementation should fetch the same-origin /find response and reuse its Rust-rendered result region. Debounce input, handle composition, cancel superseded requests and also reject stale completions by sequence. Agent help uses an explicit optional MCP URL and optional operator guide URL; neither option starts a service or changes authentication/Host/Origin admission. Without endpoint configuration, state that it is unconfigured rather than inventing a same-host endpoint.

## Agent entry convention
Follow the llms.txt v2 proposal at https://llmstxt.org/: H1, short summary, optional explanatory prose, and H2 Markdown link lists. Add rel=describedby discovery and a local agent-guide.md route. This is operational documentation, not a knowledge export or a promise that clients automatically discover it. Existing full agent GUIDE includes writer workflows and must not be republished unchanged for this read-only viewer.

## Verification and release
Require a real browser for the acceptance lane and controlled response ordering for races. Synthetic fixtures only. Run focused positive/negative HTTP/CLI/browser cases, independent adversarial review, then the repository's complete gate and exact-source required checks before the next source release. No consumer name, data, hostname or secret enters this public repository. Instance adoption and private deployment remain separately owned.
