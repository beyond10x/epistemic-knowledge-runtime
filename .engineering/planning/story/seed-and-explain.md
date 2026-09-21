---
format: aep.planning-md/1
id: story:seed-and-explain
kind: story
status: draft
title: Seed loading and the explain chain
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:commit-and-revision-lineage
- depends_on: story:eventlog-store
- implements: executable-system-specification:ekr-v1
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/explain.rs
- confidence: inferred
  path: crates/ekr-kernel/src/seed.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/fixtures/seed-minimal.yaml
- confidence: inferred
  path: crates/ekr-kernel/tests/seed.rs
revision: 4
---
## Context

Design § 8: the system begins from a minimal trusted seed — kernel rules, an ontology, validator
specs, agent roles, initial assertions — as small as practical. Design § 62: every canonical
assertion is explainable through a chain that never ends at "the model inferred it". The seed is
where the first ontology enters; explain is how anything that entered is audited.

## Acceptance

`explain` on an assertion loaded from the seed prints a chain whose last link is the seed's own
evidence.

## Tests the story ships

- Loading the same seed document twice yields `AlreadySeeded` on the second load with revision 0
  unchanged.
- Revision 0's `knowledge_root` is identical for the same seed document on two runs.
- `explain` on an unknown id returns `AssertionNotFound` with the requested id.

## Scope

- `crates/ekr-kernel/src/seed.rs` — `Seed` (design § 8), a YAML document format for it,
  `load(seed) -> Root` producing revision 0 through the same commit path as any transaction
- `crates/ekr-kernel/src/explain.rs` — `explain(AssertionId) -> Chain`: assertion → committing
  transaction → validation result → evidence → observation → external source (design § 62)
- `crates/ekr-kernel/tests/seed.rs`
- `crates/ekr-kernel/tests/fixtures/seed-minimal.yaml` — the smallest seed that loads: the
  meta-types the kernel needs and nothing domain-shaped

## Notes

Depends on `story:commit-and-revision-lineage` and `story:eventlog-store`. Crate dependencies are
the skeleton's (`ekr-kernel` → `ekr-core`, `ekr-ontology`, `ekr-graph`, `ekr-store`); `serde_yaml`
for the seed document is declared there. The seed carries no `Person`, `Project` or other domain
type (design § 9); a fixture that needs one declares it in the fixture's own ontology. Amendment
81's `HumanStatement` evidence kind is what a seed's initial assertions cite. Adds no dependency
beyond the skeleton's.
