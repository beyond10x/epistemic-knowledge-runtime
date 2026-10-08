---
format: aep.planning-md/3
id: story:describe-type-read
kind: story
status: draft
title: A describe_type read answers a type id with its declaration
relations:
- serves: vision:o5
- depends_on: story:read-surface-specified
- depends_on: story:read-answers-name-types
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: crates/ekr-views/src/query.rs
- confidence: inferred
  path: crates/ekr/src/cli/mcp.rs
- confidence: inferred
  path: crates/ekr/src/cli/view.rs
- confidence: inferred
  path: docs/cli.md
- confidence: inferred
  path: systems/ekr/domains/views.yaml
revision: 2
---
## Outcome

A read, `describe_type`, answers a node type, edge type or property id with its declaration: its name, its kind, and for an edge type its endpoint types.

## Why

Found 2026-10-06 by the gap inventory for running a company brain entirely on cortex and EKR, from a review of a live deployment on 0.0.30.
`describe_node` on a type id answers `NodeNotFound`, so an agent holding a type id has no way to learn what it is short of reading `overview`'s whole schema history. Split out of `story:read-answers-name-types` in plan-critic round 2.

## Acceptance

On a synthetic store with two node types, two edge types and two properties:
1. `describe_type` on an edge-type id answers its name, kind `edge_type` and both endpoint types.
2. On a property id it answers its name and kind `property`.
3. On a node id it refuses with the same refusal `describe_node` gives today for an unknown id.

## Depends on

`story:read-surface-specified`: `describe_type` is a twelfth operation in its read surface. `story:read-answers-name-types` lands first.

## Files (inferred)

`crates/ekr-views/src/query.rs`, `crates/ekr/src/cli/mcp.rs`, `crates/ekr/src/cli/view.rs`, `systems/ekr/domains/views.yaml`, `docs/cli.md`, `CHANGELOG.md`.
