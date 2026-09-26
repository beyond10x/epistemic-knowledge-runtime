---
format: aep.planning-md/1
id: story:schema-evolution-transactions
kind: story
status: draft
title: Evolve the schema through committed transactions
relations:
- serves: vision:o6
- decomposes: epic:p5-frontier-schema-scheduler
revision: 1
---
## Context

Operator (2026-09-25): an evolving schema is the main feature of the runtime. Design § 26 and § 51 make an ontology change an ordinary validated transaction. The P1 kernel refuses `DefineNodeType`, `DefineEdgeType` and `ModifyProperty` as `unsupported-operation` (`crates/ekr-kernel/src/validate/structural.rs:263-283`; `apply.rs:146-150`), so today a schema is fixed at seed time. The roadmap placed schema evolution in P5; it moves ahead of P2.

## Acceptance

- A committed transaction can add a node type, add an edge type and add or redeclare a property on an existing type, producing a new schema version with lineage; the ontology validators check it against existing canonical state, and a change that existing state would violate is refused with a named issue.
- Existing nodes, edges and assertions stay valid across the change; replay from the seed reproduces every root across schema versions, on both providers.
- `ekr ontology` shows the schema at the head and at a past revision; `ekr operations` stops marking the three kinds as not applied; `docs/cli.md` and `ekr guide` describe evolution.
- Out of scope here: schema discovery from evidence, risk classes and approval gates (P5), `MergeEntity`.
