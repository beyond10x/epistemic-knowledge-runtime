---
format: aep.planning-md/3
id: story:explain-bounds-documents
kind: story
status: draft
title: explain bounds the evidence text it returns
relations:
- serves: vision:o5
scope:
- confidence: inferred
  path: crates/ekr/src/cli/explain.rs
- confidence: inferred
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: docs/cli.md
revision: 2
---
## Outcome

`explain` with documents returns at most a stated number of bytes of each evidence record (the cited span, or a window around it), with the record's full length and a way to read the rest.

## Why

Found 2026-10-06 by the gap inventory for running Company Brain v3 entirely on cortex and EKR (cb3 `initiative:run-on-cortex`); no story covered it.
On a live deployment one `explain` with documents returned 4.9 MB for an assertion whose evidence is one very large record; the cited fact appears in it once. `ekr explain --documents` returns the whole retained record per link (`crates/ekr/src/cli/explain.rs:12-13` at 0.0.30) and the MCP tool passes it through (`crates/ekr/src/cli/mcp.rs:553-565`).

## Acceptance

A synthetic assertion citing a 5 MB evidence record answers under the bound (for example 64 KiB) with `truncated: true` and the full length; with the bound lifted the bytes equal today's.

## Files (from the inventory, unverified)

`crates/ekr/src/cli/explain.rs`, `crates/ekr/src/cli/mcp.rs`, `docs/cli.md`.
