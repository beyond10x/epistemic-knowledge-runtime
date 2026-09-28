---
format: aep.planning-md/3
id: story:changes-since-read
kind: story
status: active
title: Agents ask what changed since a revision or a time
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
- informed_by: story:mcp-read-tools
scope:
- confidence: inferred
  path: crates/ekr-views/src/lib.rs
- confidence: inferred
  path: crates/ekr-views/src/query.rs
- confidence: inferred
  path: crates/ekr-views/tests/fixtures/conformance/scenarios
- confidence: inferred
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: crates/ekr/src/cli/view.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/conformance/views-baseline.json
- confidence: inferred
  path: systems/ekr/conformance/views-suite.json
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-09-28T18:16:21Z", actor: "agent:claude-coordinator", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-28T22:44:17Z", actor: "agent:claude-coordinator", revision: 3}
---
## Context

Asked by a consumer instance (2026-09-28): planning agents react to change, so they need to ask
the store what changed since a revision or a time. Today an agent must diff two snapshots itself.
No `ekr.views` read answers it.

## Build

A read `ChangesSince(store, since, at?)` in `ekr.views`, specified in ESS first with its format
and scenarios, where `since` is a revision, a valid time or a transaction time: every node and
edge created, and every assertion added, superseded or retracted, between `since` and `at` (the
head when absent), each with its revision, its kind of change and its evidence ids; bounded and
paged like `ExpandNeighbourhood`. Served by `ekr view` as an endpoint and by `ekr mcp` as the tool
`changes_since`.

## Acceptance

- On a fixture store with known changes per revision, the read lists exactly those changes for
  `since` given as a revision, a valid time and a transaction time, on both providers.
- A page limit and a cursor page through a large change set with nothing lost or repeated.
- Two reads of one (since, at) pair are byte-identical, before and after an unrelated commit.
- The MCP tool's result equals the read's document.
