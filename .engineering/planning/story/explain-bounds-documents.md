---
format: aep.planning-md/3
id: story:explain-bounds-documents
kind: story
status: active
title: explain bounds the evidence text it returns
relations:
- serves: vision:o5
- depends_on: story:ocel-process-map
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/ekr/src/cli/explain.rs
- confidence: inferred
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-06T10:18:11Z", actor: "agent:claude", revision: 6, decided_on: {"recorded":{"review_outcome":5}}}
- {from: "proposed", to: "active", at: "2026-10-06T10:18:11Z", actor: "agent:claude", revision: 7, decided_on: {"recorded":{"review_outcome":5}}}
---
## Outcome

`explain` with documents returns at most a stated number of bytes of each evidence record, centred on the cited span when the evidence names one, with the record's full length; two new arguments, `offset` and `limit` (bytes), read any other part of the record through the same `explain` call.

## Why

Found 2026-10-06 by the gap inventory for running a company brain entirely on cortex and EKR, from a review of a live deployment on 0.0.30.
One `explain` with documents returned 4.9 MB for an assertion whose evidence is one very large record; the cited fact appears in it once. `ekr explain --documents` returns the whole retained record per link (`crates/ekr/src/cli/explain.rs:12-13` at 0.0.30) and the MCP tool passes it through (`crates/ekr/src/cli/mcp.rs:553-565`).

## Acceptance

On a synthetic assertion citing a 5 MB evidence record whose cited text sits at byte 3,000,000:
1. The default answer is at most 64 KiB per record, carries `truncated: true` and the full length, and contains the cited text.
2. `offset: 0, limit: 1000` answers exactly the first 1,000 bytes of the record.
3. Reading the record in `limit`-sized steps from offset 0 to the full length reassembles today's bytes exactly.

## Note

`story:read-surface-specified` models `explain`'s input after this story lands and carries `offset` and `limit` (its body and its edge record it). `story:read-surface-served-over-http` also lands after it. Both edit `systems/ekr/domains/views.yaml` and `crates/ekr/src/cli/mcp.rs`. This story lands after the stories of wave 20261005b, which share `docs/cli.md`, `CHANGELOG.md` and, for `story:ocel-process-map`, `systems/ekr/domains/views.yaml` (its edge records that one).

## Files (from the inventory, unverified)

`crates/ekr/src/cli/explain.rs`, `crates/ekr/src/cli/mcp.rs`, `systems/ekr/domains/views.yaml`, `docs/cli.md`, `CHANGELOG.md`.
