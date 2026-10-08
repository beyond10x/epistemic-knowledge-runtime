---
format: aep.planning-md/3
id: story:read-answers-name-types
kind: story
status: draft
title: Five read answers name every type, edge type and property they cite
relations:
- serves: vision:o5
- depends_on: story:explain-bounds-documents
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/ekr-views/src/query.rs
- confidence: inferred
  path: crates/ekr-views/src/timeline.rs
- confidence: inferred
  path: crates/ekr/src/cli/explain.rs
- confidence: inferred
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: crates/ekr/src/cli/view.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 6
---
## Outcome

Five reads that answer node type, edge type or property ids (`search`, `describe_node`, `expand`, `timeline`, `explain`) carry each id's name beside it. Other reads are not changed by this story.

## Why

Found 2026-10-06 by the gap inventory for running a company brain entirely on cortex and EKR, from a review of a live deployment on 0.0.30.
Every type, edge type and predicate was answered as a bare id, and names exist only inside `overview`'s schema history (`crates/ekr-views/src/query.rs:551-554` at 0.0.30), whose answer is very large on a big store. An agent cannot tell edge types apart. `story:resolve-by-type-name` covers names on input only. A read that answers one type id with its declaration is `story:describe-type-read`.

## Acceptance

On a synthetic store with two node types, two edge types and two properties:
1. `search`: every `type` id in the answer has a `type_name` equal to the ontology's.
2. `describe_node`: every assertion's predicate id and every neighbour's edge-type id has its name.
3. `expand`: every edge-type id has its name.
4. `timeline`: every predicate id has its name.
5. `explain`: every type, edge-type and predicate id it cites has its name.

## Order

After `story:explain-bounds-documents` (both edit `explain`). Before `story:read-surface-served-over-http`, which rewrites `crates/ekr/src/cli/mcp.rs` and `view.rs` into one dispatcher and carries these fields over (its edge records it). It needs no new read surface, so it does not wait for `story:read-surface-specified`. It lands after the stories of wave 20261005b, which share `docs/cli.md` and `CHANGELOG.md`.

## Files (from the inventory, unverified)

`crates/ekr-views/src/query.rs`, `crates/ekr-views/src/timeline.rs`, `crates/ekr/src/cli/explain.rs`, `crates/ekr/src/cli/mcp.rs`, `crates/ekr/src/cli/view.rs`, `systems/ekr/domains/views.yaml`, `docs/cli.md`, `CHANGELOG.md`.
