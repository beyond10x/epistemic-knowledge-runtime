---
format: aep.planning-md/3
id: story:read-surface-served-over-http
kind: story
status: draft
title: ekr view and mcp-http answer every read from one dispatcher
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
- depends_on: story:read-surface-specified
revision: 1
---
## Context

`story:read-surface-specified` declares one read surface. Today two dispatchers answer it differently:
`crates/ekr/src/cli/mcp.rs:431` matches nine tool names, and `crates/ekr/src/cli/view.rs:706-719` routes
GET paths with their own query names. A browser page depends on the GET routes.

## Build

- One dispatcher keyed by wire name, taking the arguments object, used by `mcp-http` `tools/call` and by
  `ekr view`.
- `ekr view` answers `POST /views/commands/<wire>`: the body is the MCP `arguments` object, the answer is
  the document the tool returns. `Content-Type: application/json` only, so a browser sends a preflight
  first. The body is bounded, the existing `Host` check stays, and refusals map to the statuses the
  generated OpenAPI declares.
- `mcp-http` gains `describe_evidence` and `projection`.
- Every GET route stays unchanged. Retiring them is a later story.

## Acceptance

- For every operation, the POST answer and the MCP tool answer to the same arguments are byte-identical on a
  synthetic store, on both providers.
- Every path in the generated OpenAPI answers on `ekr view`. A path the document does not declare answers
  404, a non-JSON body 415, and an oversized body 413.
- The existing GET route tests pass unchanged.
