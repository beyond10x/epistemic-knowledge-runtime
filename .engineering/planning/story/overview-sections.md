---
format: aep.planning-md/3
id: story:overview-sections
kind: story
status: draft
title: An overview answers only the sections a caller asks for
relations:
- serves: vision:o5
- decomposes: epic:p4-operator-surface
revision: 1
---
## Context

`ekr.graph-overview/1` always carries the whole ontology, the schema history, the role table and the
timeline, whatever `limit` asks for: `limit` bounds only `top`. An agent that wants the counts and the
type list receives the rest too. On a deployed store, `overview` with `limit=5` answered a document too
large for the agent's tool-result budget, so the agent read it from a file. Observed 2026-10-04 with
ekr 0.0.30 `mcp-http`.

## Build

- `overview` takes `sections`, a set of `ontology`, `schema`, `roles` and `timeline`. Absent: `meta`,
  `node_types`, `edge_types` and `top` only.
- The answer is `ekr.graph-overview/2`. Its determinism rules are /1's, with the omitted sections absent,
  never empty.
- `ekr view` `/overview` and the MCP tool take the same input; `docs/cli.md` documents it.

## Acceptance

- With all four sections named, the answer's content equals /1's on the same revision and only the format
  name differs.
- With none named, the answer's size does not grow with the number of schema versions or ontology
  properties: a test commits more of both and compares sizes.
- Each section can be requested alone.
