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
  path: .github/workflows/correctness.yml
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: inferred
  path: Taskfile.yml
- confidence: inferred
  path: crates/ekr-search-web
- confidence: inferred
  path: crates/ekr/Cargo.toml
- confidence: inferred
  path: crates/ekr/assets/search.wasm
- confidence: inferred
  path: crates/ekr/build.rs
- confidence: inferred
  path: crates/ekr/src/cli/agent_help.rs
- confidence: cited
  path: crates/ekr/src/cli/http.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: crates/ekr/src/cli/search_page.rs
- confidence: cited
  path: crates/ekr/src/cli/view.rs
- confidence: cited
  path: crates/ekr/src/cli/viewer/index.html
- confidence: cited
  path: crates/ekr/tests/adversary_agent_guidance.rs
- confidence: cited
  path: crates/ekr/tests/adversary_viewer_compact.rs
- confidence: cited
  path: crates/ekr/tests/agent_cli.rs
- confidence: inferred
  path: crates/ekr/tests/search_live.rs
- confidence: cited
  path: crates/ekr/tests/search_page.rs
- confidence: inferred
  path: crates/ekr/tests/story_contract.rs
- confidence: cited
  path: crates/ekr/tests/view_cli.rs
- confidence: cited
  path: crates/ekr/tests/view_page.rs
- confidence: cited
  path: docs/cli.md
- confidence: cited
  path: docs/overview.md
- confidence: inferred
  path: xtask/src
revision: 20
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T09:01:36Z", actor: "agent:codex", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-03T09:01:37Z", actor: "agent:codex", revision: 5}
---
## Outcome and authorization

The operator requests live results while typing on the search entry, a visible MCP connection guide and an agent-readable llms.txt entry. Implement and publish the generic capability under the standing upstream authorization. Preserve ordinary GET search and read-only serving. Rust remains the implementation language: the optional authored-JavaScript exception was not approved, so proceed on the Rust/WebAssembly baseline where feasible rather than treating that optional question as permission to stop all browser work. Generated interop assets must be emitted by pinned tooling outside committed source; do not hide authored JavaScript inside Rust strings.

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

## Static guidance review checkpoint

The static implementation at 798b7d5fc776143390da5e4f0a6114bef86cc800 passed the focused search-page target and the unchanged HTTP/CLI probe that failed on the released baseline. review-result:static-agent-entry-review-1 preserves the independent bounded review with no findings.

Follow-up unit checkpoint 63f508cad432fea986af8696a5e3e98bb904252c retains the independent review tests and updates the exact CLI-option expectation for the documented guidance flags. The corrected complete view_cli target, retained adversary_agent_guidance target and package all-target Clippy pass. The recorded test_result identifies each receipt and its precise counts, including the earlier failing expectation and existing ignored screenshot exporter. These test-only commits do not change the previously tested source binary.

Live typing, browser acceptance, full workspace gate, source release and deployment remain outstanding. No custom JavaScript is authorized. Rust/WASM remains permitted; a bounded packaging assessment is checking whether the existing single-command tagged CLI installation can preserve embedded browser assets without authored JavaScript or an extra mandatory end-user toolchain. This is a partial checkpoint, not a completed story.

## Rust browser packaging proposal

A read-only feasibility assessment identified a native-install-compatible path: Rust browser logic compiles to a checked raw WASM artifact, a pinned native wasm-bindgen-cli-support build dependency generates its processed WASM and JavaScript interop into OUT_DIR, and the CLI embeds both. Ordinary tagged CLI installation must not require Node, a separately installed binding generator or a WASM target. The raw artifact must be reproducible from pinned source/lock/toolchain, carry no private build paths, and pass a rebuild-and-compare gate.

The ordinary web binding target needs an authored initialization call. Pinned generator 0.2.129's Deno target instead emits automatic initialization through standard browser module APIs. Browser compatibility is an inference from the generator, not an accepted contract: a disposable real-browser smoke must prove Rust start runs under the intended CSP before this proposal becomes source implementation. Keep the module and WASM same-origin, use only the specific wasm-unsafe-eval allowance, and retain the no-script GET fallback. Do not add unrestricted evaluation, inline bootstrap, external scripts or authored JavaScript snippets.

Existing search concepts and wire semantics are declared in systems/ekr/domains/views.yaml (SearchNodes/NodesSearched); the browser is a read-only adapter, not a new storage domain. Its behavior remains the named acceptance cases above. Actual DOM focus/caret, stale responses, composition, clear, failure and historical links require real-browser verification after implementation. Source scope below is inferred packaging work pending the smoke, not a claim that it exists.

References: Cargo build-script OUT_DIR/native dependency contract, https://doc.rust-lang.org/cargo/reference/build-scripts.html; pinned generator source, https://github.com/wasm-bindgen/wasm-bindgen/blob/0.2.129/crates/cli-support/src/js/mod.rs; generated deployment modes, https://wasm-bindgen.github.io/wasm-bindgen/reference/deployment.html.

## Browser packaging smoke result

The disposable smoke passed: actual Chromium ran Rust's wasm-bindgen start function through the untouched pinned Deno-target generated loader. The module changed a DOM marker that was absent from the initial HTML; HTML, JavaScript module and processed WASM all returned 200 under default-src none, script-src self wasm-unsafe-eval, connect-src self, object-src none and base-uri none. No inline bootstrap, authored JavaScript or general unsafe-eval was used. All smoke processes ended. The retained private report is wasm-smoke/report.md, with commands, Rust source, lockfile, browser DOM and request log.

Proceed with Rust browser implementation under the existing scope. This result proves the automatic browser loader only. Native build.rs generation through the cli-support API, native-only installation, reproducible artifact comparison and actual search interaction remain required gates; none is inferred from the loader smoke.

## Release preparation

The coordinator prepared the next source version and its changelog while the implementation completes verification in its own tree. This is an unreleased candidate: independent browser review, integration, the full repository gate, exact-head required checks, annotated tag and published release readback remain required. Nothing in this preparation changes a consumer deployment or supplies missing authentication.

## Live implementation and review

The Rust/WASM implementation checkpoint is 5ebde9dafdfde90cfad192c7abd16c38e670c49c. Its retained live/report.md records real browser behavior, unchanged native installation without external browser tooling, independent-path byte reproducibility and artifact drift refusal. These are scoped verification results, not the full repository gate.

The first live review added two browser cases and found an intermittent document-lifecycle failure in the candidate's no-script acceptance harness. review-result:live-browser-review-1 records the failing runner. Correction1203a043b252816ae094ba8c9bd602971a3980a8 waits for a new loader's load event without suppressing protocol errors or changing product behavior. The second review reran the complete browser target successfully; review-result:live-browser-review-2 retains that outcome and the separate temporary-filesystem quota failure. The first finding's outcome is recorded as fixed. No further finding was returned in the bounded follow-up.

Proceed to integration with the prepared source version and complete repository gate. Exact-head required checks, published tag/release, consumer adoption and deployment remain pending.

## Integrated gate finding

The first integrated candidate 6b1b3e91ec9a7e476972e2d49047024af990d066 passed fmt-check, search-web-check and workspace/all-target Clippy. The coordinator guard lane then exited 101: public_surface::no_public_item_in_any_crate_is_untested reported ekr::guide and ekr::llms. Raw output is retained in the private release-gate attempt-1/pr-guards.log alongside each command exit. Remaining full-gate steps did not run.

Both methods belong to the private cli::agent_help module and its pub(super) Config; their only product callers are in sibling cli::view. Existing real HTTP tests cover their output, but unrestricted pub declarations misstate their intended internal scope to the declaration guard. Correct the declarations to pub(super), matching Config and its callers. Do not exempt the items, weaken the guard, or add token-only test references. Rerun the failed guard and existing HTTP guidance tests, integrate the correction, then run the full gate on the corrected exact source. No release or deployment is claimed.

## Integrated gate correction

Correction 1a016ba0634e44a402aef3fd58285691c827d9aa changes only guide() and llms() visibility to pub(super), matching their existing private module and restricted Config type. The coordinator reviewed the exact two-line diff and unchanged callers. No behavior, guard, assertion or browser asset changed.

The unit visibility/report.md and green.log record all 32 focused cases passing: public_surface 12, adversary_agent_guidance 4 and search_page 16; no failures or ignores. Formatting, package all-target Clippy with warnings denied and text scanning also exited zero. The worker ended its lease and handed back the idle sequential target. Integrate the correction and freeze the combined source for the complete gate; these focused results do not establish a release.
