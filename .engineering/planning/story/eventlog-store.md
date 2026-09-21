---
format: aep.planning-md/1
id: story:eventlog-store
kind: story
status: active
title: 'Eventlog-backed store: revision log, fold, snapshots, content-addressed objects'
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:graph-model-and-assertions
- implements: executable-system-specification:ekr-v1
- serves: vision:o2
scope:
- confidence: inferred
  path: crates/ekr-store/src/eventlog.rs
- confidence: inferred
  path: crates/ekr-store/src/lib.rs
- confidence: inferred
  path: crates/ekr-store/src/log.rs
- confidence: inferred
  path: crates/ekr-store/src/objects.rs
- confidence: inferred
  path: crates/ekr-store/src/snapshot.rs
- confidence: inferred
  path: crates/ekr-store/tests/providers.rs
revision: 8
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
`story:workspace-crate-skeleton`; this story adds no dependency and does not touch `Cargo.lock`,
so it can run beside `story:transaction-and-validators`.
