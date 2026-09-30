---
format: aep.planning-md/3
id: task:lineage-shows-widened-ends-and-modified-properties
kind: task
status: active
title: The schema lineage lists widened edge ends and modified properties
relations:
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T00:47:41Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-09-30T00:47:41Z", actor: "human:timo", revision: 3}
---
## Context

A consumer's operator saw it in `ekr view` 0.0.20 on a real store (reported 2026-09-30): the
schema-lineage panel says "adds no types or properties" for schema versions made only by
`WidenEdgeType`. Those versions changed the schema: `ekr ontology --at N` diffed against N-1 shows,
for example, an edge type's target gaining a node type, and another edge type's source and target
each gaining types.

The cause is in the read, not only the page: `ekr.views.OverviewSchema` gives each lineage version
`added` (node types, edge types, properties, unknown) and `removed`, and nothing for a widened edge
end or a modified property (`crates/ekr-views/src/index.rs`, `schema()`); the viewer's `addedChips`
(`crates/ekr/src/cli/viewer/index.html`) prints "adds no types or properties" when `added` is empty.
`ModifyProperty` versions show the same.

## Build

- Each lineage version in `ekr.views.OverviewSchema` also lists its widened edge ends (edge type,
  side, the node types added) and its modified properties (property, owner, and what changed:
  value type, cardinality, required, constraints), computed against its parent. Specified in
  `systems/ekr/domains/views.yaml` first, with views scenarios.
- The viewer's lineage panel shows them, and says the version adds nothing only when its ontology
  equals its parent's.
- The SDK's overview read model carries the new fields.

## Acceptance

- A store with a `WidenEdgeType` version and a `ModifyProperty` version: the overview lists each
  change for its version, on both providers.
- A version whose ontology equals its parent's is the only one the page calls empty.
- A store without such versions answers byte-identically to before.
