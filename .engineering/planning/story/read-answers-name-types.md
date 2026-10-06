---
format: aep.planning-md/3
id: story:read-answers-name-types
kind: story
status: draft
title: Read answers name every type, edge type and property they cite
relations:
- serves: vision:o5
scope:
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

`search`, `describe_node`, `expand`, `timeline` and `explain` carry the name beside each node type, edge type and property id they answer, and one read answers a type id with its declaration.

## Why

Found 2026-10-06 by the gap inventory for running Company Brain v3 entirely on cortex and EKR (cb3 `initiative:run-on-cortex`); no story covered it.
A review of a live deployment (EKR 0.0.30) found every type, edge type and predicate answered as a bare id; `describe_node` on a type id answers `NodeNotFound`, and names exist only inside `overview`'s schema history (`crates/ekr-views/src/query.rs:551-554` at 0.0.30), whose answer is very large on a big store. An agent cannot tell edge types apart. `story:resolve-by-type-name` covers names on input only; `story:read-surface-specified` lists no operation that answers a type.

## Acceptance

On a synthetic store: every type id in a `search` answer has a name equal to the ontology's; a type read on a type id answers its name and kind; on a node id it refuses as today.

## Files (from the inventory, unverified)

`crates/ekr-views/src/query.rs`, `crates/ekr/src/cli/mcp.rs`, `crates/ekr/src/cli/view.rs`, `systems/ekr/domains/views.yaml`, `docs/cli.md`. Overlaps `story:read-surface-specified`; one owner should take both.
