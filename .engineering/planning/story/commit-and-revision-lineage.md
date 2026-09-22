---
format: aep.planning-md/1
id: story:commit-and-revision-lineage
kind: story
status: draft
title: Commit, revision roots, optimistic concurrency, retraction
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:transaction-and-validators
- depends_on: story:eventlog-store
- implements: executable-system-specification:ekr-v1
- depends_on: story:kernel-validated-seed
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/apply.rs
- confidence: inferred
  path: crates/ekr-kernel/src/commit.rs
- confidence: inferred
  path: crates/ekr-kernel/src/revision.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/replay.rs
revision: 7
---
## Context

Design § 34: each successful commit produces a new immutable root with a parent, four sub-roots
and the transaction hash. Design § 71–72: validation records the revision it ran against, and a
commit that finds canonical state moved must not apply. Design § 36 and § 65: retraction and
supersession end an assertion's active life without deleting history.

## Acceptance

Through both SQLite and file providers and the real kernel authority: seed, validate, apply, persist, close the process, reopen, query the changed graph, retract, and reconstruct an earlier revision. The graph hash changes for the state-changing commit and reproduces on restart. The persisted ontology is content-bound and reopening does not substitute a caller-supplied different ontology.

## Tests the story ships

- Two commits validated against the same head cannot both publish: the loser returns Stale and changes no canonical graph.
- A crash before the conditional revision append exposes no partial state; retry after a lost successful response returns the original committed result.
- Retraction retains its original acceptance evidence and remains readable at an earlier revision. Latest-state queries exclude the retracted belief.
- Supersession records its replacement and supplied valid-time boundary. Queries before and after that boundary return the correct belief, including the design section 65 CLI example.
- Replay verifies persisted payload addresses, schema content, validation basis and revision ancestry. Missing or corrupt required payloads produce named errors, not revision-zero fallback.
- Property multiplicity and list-valued properties remain distinct; accepted multivalued input is not silently collapsed by application.

## Scope

- `crates/ekr-kernel/src/commit.rs` — `commit(ValidatedTransaction) -> Result<Root, CommitError>`;
  applies each `GraphOperation` to the canonical graph; computes the four sub-roots and the new
  `Root`; `Stale` when `validated_against != head`; fills the `RevisionEvent` and appends it to
  the `RevisionLog`
- `crates/ekr-kernel/src/apply.rs` — one function per `GraphOperation` variant, including
  `RetractAssertion` (sets `AssertionStatus::Retracted`), supersession (`Superseded { by }`,
  closes `valid_to`), `MergeEntity` (alias preserved, lineage recorded)
- `crates/ekr-kernel/src/revision.rs` — `Root` hashing, lineage walk, replay
- `crates/ekr-kernel/tests/replay.rs`

## Notes

Depends on `story:transaction-and-validators` and `story:eventlog-store` (the `RevisionLog`
trait the commit appends to). Crate dependencies are the skeleton's: `ekr-kernel` → `ekr-core`,
`ekr-ontology`, `ekr-graph`, `ekr-store`. `Root` and `RevisionEvent` are defined in `ekr-graph`
(`story:graph-model-and-assertions`); this story computes the one and fills the other. Commits
are serialised: one writer, the kernel. Adds no dependency beyond the skeleton's.

## Scope correction, 2026-09-22

The earlier scope sentence requiring MergeEntity application is superseded by the approved phase boundary: explicit merge/split semantics arrive in P3; schema evolution arrives in P5. P1 refuses these operations instead of certifying them. Ordinary graph changes, declared lifecycle operations, retraction and supersession are in scope.

The earlier reference to active() is obsolete. Acceptance is expressed through snapshot revision and valid time. Populated ontology state must contribute its canonical content to ontology_root. An actually absent agent registry may use a specified empty root; populated agent/validator state must be bound once introduced. All persisted shape changes require an explicit version and a preserving migration path, which refuses unreconstructable history.
