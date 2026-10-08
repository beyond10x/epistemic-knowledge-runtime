---
format: aep.planning-md/3
id: story:hosted-read-serving
kind: story
status: implemented
title: Serve the viewer and read-only MCP through bounded explicit HTTP listeners
relations:
- serves: vision:o5
- derived_from: task:hosted-postgres-copy-and-read-serving
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/ekr-sdk/src/viewer.rs
- confidence: cited
  path: crates/ekr-sdk/tests/session.rs
- confidence: cited
  path: crates/ekr-sdk/tests/spawn_busy.rs
- confidence: inferred
  path: crates/ekr/src/cli/http.rs
- confidence: cited
  path: crates/ekr/src/cli/mcp.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: crates/ekr/src/cli/session.rs
- confidence: cited
  path: crates/ekr/src/cli/view.rs
- confidence: cited
  path: crates/ekr/src/main.rs
- confidence: cited
  path: crates/ekr/tests/adversary_read_only_store.rs
- confidence: cited
  path: crates/ekr/tests/agent_cli.rs
- confidence: inferred
  path: crates/ekr/tests/hosted_http.rs
- confidence: cited
  path: crates/ekr/tests/read_only_store.rs
- confidence: cited
  path: crates/ekr/tests/view_cli.rs
- confidence: cited
  path: crates/ekr/tests/view_stream.rs
- confidence: cited
  path: docs/cli.md
- confidence: cited
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: cited
  path: docs/overview.md
- confidence: cited
  path: docs/sdk.md
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 18
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T01:14:16Z", actor: "agent:codex", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-03T01:27:02Z", actor: "agent:codex", revision: 9}
- {from: "active", to: "implemented", at: "2026-10-03T04:58:21Z", actor: "agent:codex", revision: 17, decided_on: {"recorded":{"test_result":8,"review_outcome":2}}}
---
## Outcome
Keep existing loopback viewer and stdio MCP behavior; add explicit hosted viewer binding and stateless Streamable HTTP MCP. This is a read-only transport over existing views, not an identity provider or canonical writer. Run after the hosted snapshot unit because CLI dispatch and docs overlap.

## Accepted interface
Viewer retains --port and gains --bind IP plus repeatable --allow-host authority. A separate mcp-http verb uses --bind, --port, repeatable --allow-host and --allow-origin. Default network binding is loopback; external binding must have explicitly admitted authorities. Both expose /healthz for process liveness and /readyz that checks an admitted, seeded, complete store. Readiness cannot announce an empty or interrupted-copy destination as ready. Exact interface may be tightened by evidence before release; all documentation and integration tests follow it.

## Acceptance cases
- viewer_defaults_to_loopback_and_accepts_only_configured_authorities: explicit external bind and Host protection coexist; forwarded headers do not confer authority; current loopback tests remain green.
- readiness_refuses_unseeded_incomplete_or_unavailable_store: startup/readiness proves a usable committed store; healthy live process alone is insufficient.
- http_and_stdio_mcp_return_identical_tool_documents: real initialize, notifications/initialized, tools/list and tools/call preserve the existing nine tools and result documents; no mutation method becomes reachable.
- mcp_http_protocol_statuses_and_versions: accepted notification/response returns202 with no body, request returns one JSON response, GET/DELETE without streams return405; invalid or unsupported protocol header returns400; protocol negotiation remains compatible.
- mcp_http_rejects_unapproved_origin_host_and_ambiguous_framing: present unapproved Origin403, exact Host allowlist, no forwarded-header trust, capped request/header bytes, duplicate/ambiguous Content-Length and Transfer-Encoding refused.
- timed_out_requests_do_not_grow_the_queue: bounded connection and request queue capacity, read/write/wait deadlines and stale-job rejection; store access remains outside Tokio.
- http_readers_observe_commits_without_mixing_revisions: current and explicit historical views retain existing consistency.
All cases assert the surviving behavior after the fix. Use synthetic stores only. Final real private-client acceptance belongs to consumers; generic transport tests run here.

## Sources and scope
Cited: crates/ekr/src/cli/mcp.rs Server::message already implements the nine tools and protocol revisions 2025-11-25/2025-06-18. crates/ekr/src/cli/view.rs already limits headers and concurrent connections, but waits without deadline for its store-thread queue. crates/ekr/src/cli/mod.rs owns command arguments and dispatch. crates/ekr/src/main.rs owns stdio selection. Tests mcp.rs/view.rs/docs_cli.rs hold compatibility.
Inferred: new bounded HTTP transport module and targeted transport tests; documentation in docs/cli.md and operational commentary in systems/ekr/domains/views.yaml. Existing view/domain entities are unchanged; no new knowledge entity.

Official protocol source inspected 2026-10-03: https://modelcontextprotocol.io/specification/2025-11-25/basic/transports . Chosen mode is stateless JSON responses; optional session IDs, SSE streams and resumability are not implemented. No credential or record body appears in server diagnostics. External authentication and private routing remain operator responsibilities and are explicitly documented.

## Follow-on
Search-first entry design is separate from changing transport. Incremental publication is separate from all read serving.

## Scoper correction
Read-only inspection confirms `crates/ekr/src/cli/session.rs` must refuse the new long-running verb in its exhaustive dispatch. Existing viewer suites are `tests/view_cli.rs` and `tests/view_stream.rs`, not `tests/view.rs`. The former deliberately forbids `--bind`; replace that old interface assertion while preserving loopback defaults, Host checks and ephemeral-port output. New queue admission must be bounded independently of connection timeouts, and discard expired jobs before store work. Preserve NDJSON streaming separately. Do not silently map a missing MCP protocol header to the newest revision: the stateless fallback in the transport specification is 2025-03-26, which this implementation does not support. Document and test an explicit missing-header policy compatible with the versions it advertises.

The existing viewer has whole-revision name/alias search, clickable details and evidence, but remains graph-oriented and loads graph libraries during startup. This transport story does not claim a search-first UI or natural-language answering.

## Release integration scope

The authorized source release updates Cargo.toml and Cargo.lock, the release section in CHANGELOG.md, README.md and docs/overview.md to the new released interface. These existing release surfaces are cited; docs_cli.rs already guards the README version. Root owns these changes after the search unit is integrated, then runs the combined release gate. No source tag is described as released before its remote verification.

## Final acceptance and release

The full local gate on 70761eaed64e3d5893a93d8ae3d5552c874a9987 passed: 2189 passed, 0 failed, 13 ignored across 358 completed runner summaries. All 10 recorded gate steps exited zero. Required real PostgreSQL and previous-release prerequisites were enabled; these scoped acceptance cases executed. Logs remain at `<cache>/ekr-hosted-runtime/release-gate/`. Existing ignored helper cases are not acceptance passes.

Exact-head GitHub Repository correctness and common / Security and privacy checks passed; correctness job https://github.com/beyond10x/epistemic-knowledge-runtime/actions/runs/37096359423/job/111126998509 completed successfully. Remote main and annotated 0.0.28 tag were verified at this commit. Release https://github.com/beyond10x/epistemic-knowledge-runtime/releases/tag/0.0.28 was read back as published at 2026-10-03T04:56:40Z. This fulfills the scoped source-release contract, not a consumer deployment claim.

Process cases below live in `crates/ekr/tests/hosted_http.rs` unless specified. Acceptance labels sometimes differ from final executable names; this table gives the actual cases.

| Acceptance line | Existing evidence and conclusion |
|---|---|
| `viewer_defaults_to_loopback_and_accepts_only_configured_authorities` | `view_cli::ekr_view_binding_is_explicit_and_defaults_to_loopback` and `external_binding_requires_and_enforces_explicit_authorities` pass. The latter refuses external bind without explicit authority and rejects forged forwarded Host authority. |
| `readiness_refuses_unseeded_incomplete_or_unavailable_store` | `health_stays_live_before_seed_and_readiness_recovers_after_seed`, `readiness_refuses_an_incomplete_copy_even_with_a_seed_and_checkpoint`, `readiness_recovers_when_sqlite_is_replaced_in_place`, and `hosted_viewer_exposes_liveness_and_admitted_seeded_readiness` pass. SDK compatibility is additionally held by unchanged `ekr-sdk/tests/session.rs::viewer_spawn_zero_returns_a_url_whose_head_answers`, the version-gating case, and both `viewer_require_ready_*` process cases. Default lazy health survives; opt-in eager admission rejects before URL announcement and reuses the admitted handle. |
| `http_and_stdio_mcp_return_identical_tool_documents` | `http_and_stdio_mcp_return_identical_nine_tool_documents` passes with real initialization, tool listing and calls, preserving result documents and rejecting a mutation tool. The protocol case in the next row supplies notification coverage. |
| `mcp_http_protocol_statuses_and_versions` | `mcp_http_protocol_statuses_versions_and_origins` and `unavailable_store_does_not_override_transport_refusals` pass: notification/response202, method405, version400 and admission-independent transport routing. Accepted tool errors retain the existing JSON-RPC response contract. |
| `mcp_http_rejects_unapproved_origin_host_and_ambiguous_framing` | The preceding protocol case plus `mcp_http_rejects_ambiguous_framing_and_unapproved_hosts` pass for Origin/Host authority, duplicate/ambiguous lengths, transfer encoding and size bounds. |
| `timed_out_requests_do_not_grow_the_queue` | `cli::http::tests::timed_out_jobs_cannot_grow_the_bounded_queue` and `health_bypasses_a_saturated_store_queue` pass. Existing `adversary_stream_pass1::sixty_four_concurrent_streams_each_deliver_the_whole_page` passes unchanged after restoring the viewer queue capacity; HTTP MCP retains its smaller bounded queue. Deadline/store-thread implementation was inspected in the transport review; no new hosted load benchmark is claimed. |
| `http_readers_observe_commits_without_mixing_revisions` | `http_readers_observe_external_commits_and_keep_historical_documents` passes for File/SQLite, preserving explicit historical responses while latest advances. `both_http_listeners_read_nonwritable_stores_without_changing_files` preserves all source bytes under the physical read-only contract. PostgreSQL read consistency also has the separate kernel acceptance above; this process suite is not a PostgreSQL network deployment test. |

Review anchors: `review-result:hosted-http-final-2`, `hosted-readiness-copy-final-1`, `hosted-viewer-capacity-final-1`, and `hosted-sdk-startup-final-1`. Host/origin allowlists and external authentication documentation establish the generic integration boundary; no identity provider is implemented. The synthetic loopback client smoke is additional interoperability evidence, not consumer deployment acceptance.
