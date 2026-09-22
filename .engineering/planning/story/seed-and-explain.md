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
- depends_on: story:kernel-validated-seed
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/explain.rs
- confidence: inferred
  path: crates/ekr-kernel/src/lib.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/explain.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/fixtures/seed-minimal.yaml
- confidence: inferred
  path: crates/ekr-kernel/tests/seed.rs
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
revision: 9
---
## Context

Design § 8 supplies the minimal seed; § 62 requires an explain chain for each
canonical assertion. Seed admission and replay were split into
`story:kernel-validated-seed`. The coupled persisted-contract/writer unit now
owns the versioned seed and retained-result behavior. This story owns the
remaining explain chain after durable transaction application exists.

## Acceptance

`explain` on an assertion loaded from the seed returns a chain ending in the
seed's actual retained evidence. On an ordinarily committed assertion it returns
the assertion, committing transaction, actual validation record and supporting
evidence. The chain survives a fresh-process reopen. These cases are unexecuted
until the durable writer and explain handler land.

## Tests the story ships

- A seeded assertion explains through the real retained seed admission record
  and its evidence. No ordinary proposal or observation is invented for a seed.
- An ordinarily committed assertion explains through its exact proposal,
  validation basis/profile and committed result, with retained evidence bytes
  verified against their content addresses.
- A retracted or superseded assertion preserves its original acceptance evidence
  and adds the lifecycle-change provenance. Historical knowledge remains auditable.
- HumanStatement evidence terminates directly. Later Observation evidence extends
  the chain through the real observation/source when P2 admits it; P1 cannot
  fabricate that path to satisfy the general design sketch.
- An unknown assertion returns `AssertionNotFound` with the requested identity.
  Corrupt or missing required records refuse instead of returning a partial chain
  as though it established provenance.

The earlier second-identical-seed `AlreadySeeded` expectation is superseded by
DESIGN § 91 and `ekr.kernel.Seed`: an exact logical retry returns the original
retained SeedResult; differing parsed input or trusted context refuses. That
acceptance belongs to the coupled durable unit and is not duplicated here.

## Scope

- `crates/ekr-kernel/src/explain.rs` — the typed chain over the real retained
  admission, validation, commit and evidence records.
- `crates/ekr-kernel/tests/seed.rs` and its declared minimal fixture — seed-chain
  acceptance and the unknown-assertion control, preserving seed admission cases.
- Explain acceptance for ordinary and lifecycle-changing commits uses the writer's
  final fixture/record APIs. Any additional test file is recorded before dispatch.

## Notes

Depends on `story:commit-and-revision-lineage`, `story:eventlog-store`, and
`story:kernel-validated-seed`. Uses the skeleton's kernel dependencies. Fixture
concepts are runtime vocabulary and declared in the fixture ontology. This story
stays open when seed admission alone passes; it does not own a second seed writer.

## Split, 2026-09-22

The prior split is retained in the governed history. This revision removes
obsolete seed behavior from the remaining explain scope and aligns the planned
acceptance with the activated durable record and assertion lifecycle contracts.

## Dispatch preparation

The read-only cited/inferred scope report is retained in
`.engineering/reviews/p1-cli-explain-scope.md`. Its proposed kernel lib.rs export
and dedicated tests/explain.rs are selected for the remaining explain work;
seed.rs ownership remains with the coupled durable unit and is removed from this
story's remaining write scope. Existing seed acceptance tests remain dependencies
and must not be weakened.

The unapplied command-result/valid-time proposal is
`.engineering/waves/p1-cli-explain-contract-preparation.md` and its adjacent patch.
It compiles and synthesizes, but has not run or become the active shared contract.
Host configuration/authentication transport and final public facade signatures
must be resolved before dispatch. This does not grant a parallel writer to files
owned by the active durable unit.
