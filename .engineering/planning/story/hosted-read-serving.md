---
format: aep.planning-md/3
id: story:hosted-read-serving
kind: story
status: active
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
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 13
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T01:14:16Z", actor: "agent:codex", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-03T01:27:02Z", actor: "agent:codex", revision: 9}
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
