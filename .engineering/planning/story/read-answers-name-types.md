---
format: aep.planning-md/3
id: story:read-answers-name-types
kind: story
status: draft
title: Read answers name every type, edge type and property they cite
relations:
- serves: vision:o5
- depends_on: story:read-surface-specified
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
revision: 4
---
## Outcome

Every read that answers a node type, edge type or property id carries its name beside it, and a new read, `describe_type`, answers a type id with its declaration (name, kind, and for an edge type its endpoint types).

## Why

Found 2026-10-06 by the gap inventory for running a company brain entirely on cortex and EKR, from a review of a live deployment on 0.0.30.
Every type, edge type and predicate was answered as a bare id; `describe_node` on a type id answers `NodeNotFound`, and names exist only inside `overview`'s schema history (`crates/ekr-views/src/query.rs:551-554` at 0.0.30), whose answer is very large on a big store. An agent cannot tell edge types apart. `story:resolve-by-type-name` covers names on input only.

## Acceptance

On a synthetic store with two node types, two edge types and two properties:
1. `search`: every `type` id in the answer has a `type_name` equal to the ontology's.
2. `describe_node`: every assertion's predicate id and every neighbour's edge-type id has its name.
3. `expand`: every edge-type id has its name.
4. `timeline`: every predicate id has its name.
5. `explain`: every type, edge-type and predicate id it cites has its name.
6. `describe_type` on an edge-type id answers its name, kind and endpoint types; on a node id it refuses with the same refusal `describe_node` gives today for an unknown id.

## Depends on

`story:read-surface-specified`: `describe_type` is a twelfth operation in its read surface, and that story's list grows by it. `story:explain-bounds-documents` lands first (both edit `explain`).

## Files (from the inventory, unverified)

`crates/ekr-views/src/query.rs`, `crates/ekr-views/src/timeline.rs`, `crates/ekr/src/cli/explain.rs`, `crates/ekr/src/cli/mcp.rs`, `crates/ekr/src/cli/view.rs`, `systems/ekr/domains/views.yaml`, `docs/cli.md`, `CHANGELOG.md`.
