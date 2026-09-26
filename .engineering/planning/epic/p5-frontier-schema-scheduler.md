---
format: aep.planning-md/2
id: epic:p5-frontier-schema-scheduler
kind: epic
status: draft
title: P5 — Frontier, schema evolution, scheduler
relations:
- decomposes: initiative:epistemic-knowledge-runtime
- depends_on: epic:p3-incubation-integration
- serves: vision:o6
revision: 2
---
## Context

Design § 25–26, § 30–33, § 48–50. The graph drives its own observation, the ontology grows from
evidence, and the loop runs unattended within a budget. v2's `unlimited_budget = true` set by hand
for repair is the failure this epic's stop condition prevents.

## Outcome

Crates `ekr-frontier`, `ekr-schema`, `ekr-runtime`:

- `KnowledgeFrontier` and `FrontierItem` prioritisation (§ 31–32), including `ObligationDue`
  (amendment 84);
- schema discovery over transient roots producing `SchemaProposal` with risk class, migration plan
  and stronger gates (§ 25–26, § 50);
- the epistemic loop as `ekr` verbs ordered by an AEP driver map, in timer and continuous modes
  with independent per-source collectors (A11);
- cost accounting: micro-USD, daily boundaries, reservations, a stop condition that cannot be
  removed without an approval record (A7); trust profiles and consensus (§ 48–49).

## Acceptance

The § 64 ontology-discovery fixture integrates parked facts after schema v(n+1) is approved; a
day's spend stops at the configured limit and the stop is visible in status.

## Carries

A7, A11.

## Depends on

P3 only; runs in parallel with P6.

## Operation assignment capability

Amendment 87 includes operation property assignments (sets) using ValueTemplate. The P1 independent checkpoint found that the input decoder discarded this undeclared field; story:refuse-discarded-ontology-semantics makes unsupported input fail closed. Full project completion must define and implement these assignment semantics, preserve typed cardinality and unordered-write conflict checks, and include them in schema compatibility and migration review. The current design names ValueTemplate without defining its language: keep that question explicit in this phase's decomposition rather than silently accepting or inventing a template grammar. P1 refusal does not discharge this capability.
