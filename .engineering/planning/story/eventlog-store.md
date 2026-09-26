---
format: aep.planning-md/2
id: story:eventlog-store
kind: story
status: implemented
title: 'Eventlog-backed store: revision log, fold, snapshots, content-addressed objects'
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:graph-model-and-assertions
- implements: executable-system-specification:ekr-v1
- serves: vision:o2
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/ekr-store/Cargo.toml
- confidence: cited
  path: crates/ekr-store/src/eventlog.rs
- confidence: cited
  path: crates/ekr-store/src/lib.rs
- confidence: cited
  path: crates/ekr-store/src/log.rs
- confidence: cited
  path: crates/ekr-store/src/objects.rs
- confidence: cited
  path: crates/ekr-store/src/snapshot.rs
- confidence: cited
  path: crates/ekr-store/tests/adversary2_event_vocabulary.rs
- confidence: cited
  path: crates/ekr-store/tests/adversary2_membrane_bounds.rs
- confidence: cited
  path: crates/ekr-store/tests/adversary2_retention_event_contract.rs
- confidence: cited
  path: crates/ekr-store/tests/adversary_membrane_and_schema.rs
- confidence: cited
  path: crates/ekr-store/tests/adversary_objects_and_append.rs
- confidence: cited
  path: crates/ekr-store/tests/domain_projection.rs
- confidence: cited
  path: crates/ekr-store/tests/fixture/mod.rs
- confidence: cited
  path: crates/ekr-store/tests/fold_rules.rs
- confidence: cited
  path: crates/ekr-store/tests/lineage/mod.rs
- confidence: cited
  path: crates/ekr-store/tests/membrane_boundary.rs
- confidence: cited
  path: crates/ekr-store/tests/providers.rs
- confidence: cited
  path: crates/ekr/tests/story_contract.rs
- confidence: cited
  path: systems/ekr/domains/store.yaml
revision: 15
---
## Context

ADR 0001: persistence goes through `beyond10x/eventlog`. A committed revision is an event, the
canonical graph is a fold over the log, a snapshot is an eventlog snapshot, and bytes are
content-addressed under a storage class (design § 37, § 57). Two providers from the start, SQLite
and file, so a property proved in a test is proved for the deployment.

## Acceptance

A store written through the SQLite provider, closed and reopened, folds to the same head `Root`
hash it reported before closing.

## Tests the story ships

- The same property over the file provider.
- Storing identical bytes twice yields one `StoredObject`.
- `replay(from)` over a log of N events yields the same fold as `fold()`.

## Scope

- `crates/ekr-store/src/log.rs` — the `RevisionLog` trait: `append(RevisionEvent)`,
  `fold() -> CanonicalGraph`, `head() -> Option<Root>`, `replay(from)`
- `crates/ekr-store/src/eventlog.rs` — the eventlog-backed implementation over
  `eventlog-sqlite` and `eventlog-file`
- `crates/ekr-store/src/objects.rs` — `StoredObject`, `StorageClass`, the content-addressed
  object store (`systems/ekr/domains/store.yaml`)
- `crates/ekr-store/src/snapshot.rs` — materialised fold at a revision
- `crates/ekr-store/src/lib.rs`
- `crates/ekr-store/tests/providers.rs`

## Notes

Depends on `story:graph-model-and-assertions` only. Crate dependencies are the skeleton's:
`ekr-store` → `ekr-core`, `ekr-graph`; it persists `Root` and `RevisionEvent` from `ekr-graph` and
defines no event shape of its own — the kernel, above this crate, fills and publishes them
(`systems/ekr/components.yaml`). The fold applies committed events only; it does not re-run
validation. Tenancy: one tenant per store in P1, named in the stream coordinate as eventlog
requires. The eventlog crates and `tempfile` are declared for `ekr-store` by
`story:workspace-crate-skeleton`.

**Corrected on 2026-09-21, before wave p1-05 dispatched.** This paragraph said the story adds no
dependency and does not touch `Cargo.lock`, and that was the basis for running it beside
`story:transaction-and-validators`. It is false. `eventlog-core`'s `EventStore` trait is async —
every method returns a `BoxFuture` — and `eventlog-sqlite` wraps a synchronous `rusqlite` behind a
`tokio` runtime context, so a future driven outside one panics rather than failing. This workspace
declares no runtime at all.

`architecture-decision-record:0006-ekr-store-bridges-the-async-port` settles it: `tokio` joins the
workspace dependencies with the `rt` feature, `ekr-store` owns a current-thread runtime, and its
`RevisionLog` is synchronous so that `ekr-kernel`, the CLI and everything above stay synchronous.
`Cargo.toml`, `Cargo.lock` and `crates/ekr-store/Cargo.toml` are therefore in this story's scope,
for that change and no other.

The parallel-safety conclusion survives the correction: `story:transaction-and-validators` touches
none of those three files, so the two units are still disjoint on every file. It now rests on a
reading of the dependency rather than on this paragraph's wrong sentence.

## Acceptance correction, 2026-09-22

The operator keeps this story implemented. Its reopen acceptance used a substitute CommitAuthority (`tests/lineage/mod.rs` and `tests/providers.rs`), establishing provider behavior rather than durable authorization through the kernel. The independent review of 209dd5e reproduced a real-kernel commit that advances the reported revision without applying CreateNode, then reopens at revision zero. `story:commit-and-revision-lineage` owns closure through both backends with the real authority and a changing graph hash. The original acceptance is not evidence of that stronger property.
