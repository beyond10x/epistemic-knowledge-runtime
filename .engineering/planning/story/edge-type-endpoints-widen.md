---
format: aep.planning-md/3
id: story:edge-type-endpoints-widen
kind: story
status: implemented
title: A schema change widens an edge type's source and target types
relations:
- serves: vision:o5
- decomposes: epic:p5-frontier-schema-scheduler
scope:
- confidence: inferred
  path: crates/ekr-ontology/src/evolve.rs
- confidence: inferred
  path: docs/schema-evolution.md
- confidence: inferred
  path: systems/ekr/domains/kernel.yaml
- confidence: inferred
  path: systems/ekr/domains/ontology.yaml
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T11:07:35Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-09-29T11:07:36Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-09-29T18:41:24Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

A consumer instance measured on 0.0.16 (2026-09-29): a delta ingest bringing a second source into a
store built from a first reused existing edge types with new endpoint types (AUTHORED
Person→WorkItem 1,232 facts, RELEASE_OF →Component 245, REFERENCES WorkItem→ 138, others 153);
1,768 of 2,312 facts were skipped because no schema change widens an edge type's ends.

Today `DefineEdgeType` on a declared id is refused as `type-already-declared`
(`crates/ekr-ontology/src/evolve.rs:94–96`, `:164–170`), and `ModifyProperty` changes properties
only. `EdgeType` already holds sets of `source_types` and `target_types`
(`crates/ekr-ontology/src/types.rs:111`, `:114`). Without widening, a consumer whose knowledge grows
across sources mints parallel edge types (AUTHORED_WORKITEM), which fragments the graph.

## Build

Spec first, in `systems/ekr/domains/ontology.yaml` and `kernel.yaml`: a schema change that widens an
existing edge type's `source_types` and/or `target_types` by adding declared node types, additive
only (removing an end is refused), under validation profile v2 and v3, in a schema-only transaction
like `ModifyProperty`; each widening is a new schema version. Refusals: an unknown edge type, an
unknown node type, a change that removes an end, a change without effect (`WithoutEffect`). Existing
edges stay valid by construction. `docs/schema-evolution.md`, `docs/cli.md` and `ekr operations`
document it.

## Acceptance

- A store with AUTHORED Person→Document widens it to Person→{Document, WorkItem} in one schema-only
  transaction, and a following transaction creates Person→WorkItem AUTHORED edges, on both providers.
- `ekr ontology --at` shows the new schema version and its parent; replay reproduces the roots.
- Removing an end, naming an unknown type and a no-op widening are each refused by name.
- The kernel conformance suite gains the scenarios.
