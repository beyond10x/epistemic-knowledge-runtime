---
format: aep.planning-md/3
id: story:property-constraints-are-enforced
kind: story
status: draft
title: Declared property constraints are enforced on write
relations:
- serves: vision:o2
- decomposes: epic:p5-frontier-schema-scheduler
scope:
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/ekr-kernel/src/validate/ontology.rs
- confidence: inferred
  path: crates/ekr-kernel/src/validate/schema.rs
- confidence: cited
  path: crates/ekr-kernel/tests/schema_evolution.rs
- confidence: cited
  path: crates/ekr-kernel/tests/seed.rs
- confidence: cited
  path: crates/ekr-kernel/tests/validation.rs
- confidence: cited
  path: crates/ekr-ontology/src/evolve.rs
- confidence: inferred
  path: crates/ekr-ontology/src/types.rs
- confidence: cited
  path: crates/ekr-ontology/tests/schema_evolution.rs
- confidence: cited
  path: crates/ekr/src/cli/agent.rs
- confidence: cited
  path: crates/ekr/tests/docs_cli.rs
- confidence: cited
  path: docs/cli.md
- confidence: cited
  path: docs/overview.md
- confidence: cited
  path: docs/schema-evolution.md
- confidence: inferred
  path: systems/ekr/conformance/suite.json
- confidence: inferred
  path: systems/ekr/domains/ontology.yaml
revision: 4
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

## Scope

Derived 2026-09-30 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `crates/ekr-kernel` (validator 5, `OntologyConstraint`) and `crates/ekr-ontology` (schema compatibility) — cited
- **Files:** `crates/ekr-kernel/src/validate/ontology.rs:88-110` — cited, the property-constraint refusal (`UNSUPPORTED_CONSTRAINT`, line 41) to be replaced by evaluation; lines 139-142 (preconditions/emits) stay
- **Files:** `crates/ekr-ontology/src/evolve.rs:518-528,649-660` — cited, `Incompatibility::ConstraintChanged` refuses any constraint change on a type with instances
- **Also likely:** `crates/ekr-ontology/src/evolve.rs:331` `InstanceState` and its kernel impl `crates/ekr-kernel/src/validate/schema.rs:257-290` — inferred, naming a violating fact needs a value-level accessor
- **Also likely:** a new evaluator module in `crates/ekr-ontology/src/` — inferred; placement follows the constraint language
- **Tests asserting the old refusal:** `crates/ekr-kernel/tests/validation.rs:2747`, `crates/ekr-kernel/tests/seed.rs:667`, `crates/ekr-kernel/tests/schema_evolution.rs:495`, `crates/ekr-ontology/tests/schema_evolution.rs:1007-1045` — cited
- **Documents:** `docs/cli.md:1200,2453,2488`, `docs/schema-evolution.md:40,401`, `docs/overview.md:231`, `README.md:33`, `crates/ekr/src/cli/agent.rs:478`, `crates/ekr/tests/docs_cli.rs:2084-2100` — cited
- **Spec:** `systems/ekr/domains/ontology.yaml:369-372` (constraint language `UNMAPPED`) and `systems/ekr/conformance/suite.json` — inferred
- **Confidence:** medium — the refusal and compatibility sites are cited; the evaluator's home depends on the undecided constraint language
- **Blocked:** `decision-blocker:constraint-language`
- **Would collide with:** any unit touching validator 5 (`validate/ontology.rs`), schema compatibility (`evolve.rs`), the `docs/cli.md` issue-code table, or the kernel suite
- **Safety fact:** keeping `PropertyDefinition::constraints` as text parsed at evaluation leaves the canonical schema encoding unchanged (`crates/ekr-ontology/src/canonical.rs:54`) — inferred
