---
format: aep.planning-md/1
id: story:schema-evolution-transactions
kind: story
status: active
title: Evolve the schema through committed transactions
relations:
- serves: vision:o6
- decomposes: epic:p5-frontier-schema-scheduler
scope:
- confidence: inferred
  path: crates/ekr-graph/src/root.rs
- confidence: cited
  path: crates/ekr-kernel/src/apply.rs
- confidence: cited
  path: crates/ekr-kernel/src/authority.rs
- confidence: cited
  path: crates/ekr-kernel/src/commit.rs
- confidence: cited
  path: crates/ekr-kernel/src/read.rs
- confidence: cited
  path: crates/ekr-kernel/src/replay.rs
- confidence: cited
  path: crates/ekr-kernel/src/seed.rs
- confidence: cited
  path: crates/ekr-kernel/src/transaction.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/candidate.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/cardinality.rs
- confidence: inferred
  path: crates/ekr-kernel/src/validate/mod.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/ontology.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/reference.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/structural.rs
- confidence: cited
  path: crates/ekr-kernel/src/validate/types.rs
- confidence: cited
  path: crates/ekr-kernel/tests/adversary_membrane_pass_two.rs
- confidence: cited
  path: crates/ekr-kernel/tests/encoding_field_order.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/seed.rs
- confidence: cited
  path: crates/ekr-kernel/tests/transaction_document.rs
- confidence: cited
  path: crates/ekr-kernel/tests/validate_properties.rs
- confidence: cited
  path: crates/ekr-kernel/tests/validation.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/verified_read.rs
- confidence: inferred
  path: crates/ekr-ontology/src/canonical.rs
- confidence: inferred
  path: crates/ekr-ontology/src/evolve.rs
- confidence: inferred
  path: crates/ekr-ontology/src/lib.rs
- confidence: cited
  path: crates/ekr-ontology/src/schema.rs
- confidence: cited
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/src/cli/mod.rs
- confidence: cited
  path: crates/ekr/src/cli/ontology.rs
- confidence: cited
  path: crates/ekr/tests/adversary_p4_01_agent_cli.rs
- confidence: cited
  path: crates/ekr/tests/agent_cli.rs
- confidence: cited
  path: crates/ekr/tests/docs_cli.rs
- confidence: cited
  path: docs/cli.md
- confidence: inferred
  path: docs/epistemic-knowledge-runtime-design.md
- confidence: cited
  path: docs/roadmap.md
- confidence: inferred
  path: systems/ekr/conformance/baseline.json
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
- confidence: cited
  path: systems/ekr/domains/ontology.yaml
revision: 41
---
## Context

Operator (2026-09-25): an evolving schema is the main feature of the runtime. Design § 26 and § 51 make an ontology change an ordinary validated transaction. The P1 kernel refuses `DefineNodeType`, `DefineEdgeType` and `ModifyProperty` as `unsupported-operation` (`crates/ekr-kernel/src/validate/structural.rs:263-283`; `apply.rs:146-150`), so today a schema is fixed at seed time. The roadmap placed schema evolution in P5; it moves ahead of P2.

## Acceptance

- A committed transaction can add a node type, add an edge type and add or redeclare a property on an existing type, producing a new schema version with lineage; the ontology validators check it against existing canonical state, and a change that existing state would violate is refused with a named issue.
- Existing nodes, edges and assertions stay valid across the change; replay from the seed reproduces every root across schema versions, on both providers.
- `ekr ontology` shows the schema at the head and at a past revision; `ekr operations` stops marking the three kinds as not applied; `docs/cli.md` and `ekr guide` describe evolution.
- Out of scope here: schema discovery from evidence, risk classes and approval gates (P5), `MergeEntity`.
