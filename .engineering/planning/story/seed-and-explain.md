---
format: aep.planning-md/1
id: story:seed-and-explain
kind: story
status: active
title: Seed loading and the explain chain
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:commit-and-revision-lineage
- depends_on: story:eventlog-store
- implements: executable-system-specification:ekr-v1
- depends_on: story:kernel-validated-seed
- serves: vision:o2
scope:
- confidence: cited
  path: crates/ekr-kernel/src/explain.rs
- confidence: cited
  path: crates/ekr-kernel/src/lib.rs
- confidence: cited
  path: crates/ekr-kernel/tests/explain.rs
- confidence: cited
  path: systems/ekr/domains/kernel.yaml
revision: 16
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

- crates/ekr-kernel/src/explain.rs: new typed Snapshot/Explain results and methods
  over one real kernel-owned VerifiedRead, cited by the activated dispatch.
- crates/ekr-kernel/src/lib.rs: only the new explain module and its public re-exports.
- crates/ekr-kernel/tests/explain.rs: new real-provider origin, lifecycle,
  replacement, evidence, captured-boundary and corruption controls.
- systems/ekr/domains/kernel.yaml: coordinator-owned shared response declarations.

Existing seed tests, minimal fixture, Runtime and verified-read construction
remain with the durable implementor. Their behavior is a read dependency here,
not a second write grant. The CLI owner may consume the public result but may
not implement a separate explanation or valid-time query.

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

The read-only scope and contract reviews are retained in
`.engineering/reviews/p1-cli-explain-scope.md` and
`.engineering/reviews/p1-cli-explain-contract-review.md`. The selected correction
is `.engineering/waves/p1-cli-explain-contract-r2.md`. Its read-result declarations
are now active in kernel ESS in every participating tree. The original unapplied
patch remains historical preparation evidence and must not be applied again.
DESIGN 93 already binds ordinary command results; no retained format changes here.

Explain selects one kernel-owned VerifiedRead, preserves actual origin and
lifecycle records, follows accepted replacements even when they predate
supersession, and selects support by the exact R2 recipe. The kernel projection
must verify all selected payloads, deduplicate evidence by stable identity and
report its actual output length. Snapshot uses that same captured boundary and
the graph's shared valid-time query, retaining the complete graph/root separately
from selected assertion IDs. These handler claims remain unexecuted.

The durable worker has agreed the public Runtime and VerifiedRead seams; a
coherent committed source checkpoint is required before dependent compilation.
An explicitly partitioned read/CLI implementation can then proceed alongside
the remaining writer fault controls, in the existing approved completion unit.
This is overlapping source preparation, not a declaration that the writer is
implemented or that this story's dependency has closed. Integration and closure
still require the writer's completed controls and the coupled full gate.

The read source owner receives only new src/explain.rs, new tests/explain.rs and
the module/re-export lines for explain in src/lib.rs, within ekr-kernel. It adds
methods over VerifiedRead rather than editing Runtime or reaching a raw store.
The durable worker retains read capture, runtime, recovery, apply, replay, seed
implementation and all existing kernel tests. This measured partition replaces
the earlier planned ownership of existing seed.rs tests and minimal seed fixture.
