---
format: aep.planning-md/3
id: story:property-constraints-are-enforced
kind: story
status: draft
title: Declared property constraints are enforced on write
relations:
- serves: vision:o2
- decomposes: epic:p5-frontier-schema-scheduler
revision: 1
---
## Context

Property constraints, operation preconditions and emitted events can be declared in a schema, but
a write touching one is refused as `unsupported-constraint` (README "not in 0.0.12";
`docs/overview.md`: "not scheduled in the roadmap"). A consumer instance asks for types that gain
constraints and validation as the schema advances (2026-09-28).

## Build

The ontology-constraint validator enforces declared property constraints on write, and a schema
transaction may add a constraint to an existing property when every current fact satisfies it.

## Acceptance

- A write that violates a declared constraint is refused with a named validation issue; one that
  satisfies it commits, on both providers.
- Adding a constraint that a current fact violates is refused and names that fact.
- `unsupported-constraint` no longer answers a write that touches a property constraint.
