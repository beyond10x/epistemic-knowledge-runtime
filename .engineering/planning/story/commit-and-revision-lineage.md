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
scope:
- confidence: inferred
  path: crates/ekr-kernel/src/apply.rs
- confidence: inferred
  path: crates/ekr-kernel/src/commit.rs
- confidence: inferred
  path: crates/ekr-kernel/src/revision.rs
- confidence: inferred
  path: crates/ekr-kernel/tests/replay.rs
revision: 4
---
## Context

Design § 34: each successful commit produces a new immutable root with a parent, four sub-roots
and the transaction hash. Design § 71–72: validation records the revision it ran against, and a
commit that finds canonical state moved must not apply. Design § 36 and § 65: retraction and
supersession end an assertion's active life without deleting history.

## Acceptance

Replaying the committed transaction log from the seed reproduces the head's `knowledge_root`
byte for byte.

## Tests the story ships

- Committing a transaction validated against revision N when the head is N+1 returns `Stale`,
  publishes `TransactionStale`, and changes no graph state.
- `RetractAssertion` leaves the assertion readable in an as-of query and absent from `active()`.
- Supersession closes the superseded assertion's `valid_to` and records `Superseded { by }`.

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
