---
format: aep.planning-md/3
id: task:mcp-serves-the-head
kind: task
status: draft
title: ekr mcp serves the head as a tool
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
revision: 1
---
## Context

0.0.14 removed `head` from every `ekr.views` document (views rule 5) and gave the viewer
`GET /head`, answering `{"format":"ekr.view-head/1","head":N}`. `ekr mcp` got no counterpart: its
eight tools (`crates/ekr/src/cli/mcp.rs:376-383`) are overview, search, describe_node, expand,
timeline, changes_since, explain and resolve, and none answers the head. An agent that took the
head from an MCP answer before 0.0.14 now has no MCP read for it (reported by a consumer instance,
2026-09-29). The gap was introduced by wave read-02.

## Build

An MCP tool `head` with no arguments, answering the same `ekr.view-head/1` document as `GET /head`
in `structuredContent`, listed by `tools/list`, documented in `docs/cli.md` § `ekr mcp`.

## Acceptance

- `tools/call head` returns the document `GET /head` returns for the same store, before and after a
  commit made by another process.
- `tools/list` lists `head` with an empty input schema; an argument is refused with -32602.
- `crates/ekr/tests/docs_cli.rs` passes with the tool documented.
