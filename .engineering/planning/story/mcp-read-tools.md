---
format: aep.planning-md/3
id: story:mcp-read-tools
kind: story
status: draft
title: Agents read canonical state through read-only MCP tools
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
scope:
- confidence: inferred
  path: crates/ekr-mcp/src/lib.rs
- confidence: inferred
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: crates/ekr/tests/mcp.rs
- confidence: inferred
  path: docs/cli.md
revision: 5
---
## Context

Agents read canonical state today only through one-shot CLI reads (`snapshot`, `explain`,
`resolve`) or `ekr view`'s HTTP reads for a browser page. Roadmap P4 plans `ekr-mcp` read tools,
lifted from the v2 instance's MCP facade (A14); design § 70. The reads they need are already
specified: `ekr.views` (`ProjectOverview`, `ExpandNeighbourhood`, `DescribeNode`, `SearchNodes`,
`ProjectTimeline`) and the kernel's explain.

## Build

`ekr mcp`: a read-only MCP server over stdio exposing search, describe node, expand
neighbourhood, timeline and explain assertion, each returning the `ekr.views` or explain document
unchanged. Record text is untrusted evidence and is returned as data, never as instructions (A14).
No write tools.

## Acceptance

- Each tool's result equals the corresponding `ekr.views` read or `ekr explain` output for the
  same store state, on both providers.
- The server exposes no tool that proposes, validates or commits.
- `docs/cli.md` documents the verb and its tools.
