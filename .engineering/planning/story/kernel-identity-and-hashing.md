---
format: aep.planning-md/1
id: story:kernel-identity-and-hashing
kind: story
status: active
title: Stable identity and content hashing
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:workspace-crate-skeleton
- implements: executable-system-specification:ekr-v1
- serves: vision:o2
scope:
- confidence: inferred
  path: crates/ekr-core/src/canonical.rs
- confidence: inferred
  path: crates/ekr-core/src/hash.rs
- confidence: inferred
  path: crates/ekr-core/src/identity.rs
- confidence: inferred
  path: crates/ekr-core/src/lib.rs
revision: 8
---
## Context

Design § 6.4 and § 10: human-readable names are not identities; every persistent object carries a
stable id that survives renames, aliases, merges and schema migration. Design § 57: raw payloads
and immutable artifacts are content-addressed. Everything above builds on these two primitives, so
they come first, alone, in the bottom crate `ekr-core`, which depends on no other crate of the
workspace.

## Acceptance

A property test renames a node's `canonical_name` a thousand times and its `NodeId` never changes.

## Tests the story ships

- `hash.rs`: the same canonical bytes produce the same `ContentHash` on every run, and the hex
  form round-trips.
- `canonical.rs`: two `BTreeMap`s with the same entries in different insertion order encode to the
  same bytes.
- Every id type round-trips through serde as a string and refuses a malformed one.

## Scope

- `crates/ekr-core/src/identity.rs` — the id newtypes of `systems/ekr/domains/kernel.yaml`
  (`AgentId`, `TransactionId`, `RevisionId`, `IssueId`, `RevisionNumber`), `ontology.yaml`
  (`TypeId`, `PropertyId`, `SchemaVersionId`) and `graph.yaml` (`GraphRootId`, `NodeId`,
  `EdgeId`, `AssertionId`, `SupportId`, `EvidenceId`, `ObservationId`), each a `u128` newtype
  minted as UUIDv7, with serde
- `crates/ekr-core/src/hash.rs` — `ContentHash([u8; 32])`, SHA-256, hex display
- `crates/ekr-core/src/canonical.rs` — a `Canonical` trait: the deterministic byte encoding a
  hash is computed over; `BTreeMap` ordering, no floats in keys
- `crates/ekr-core/src/lib.rs`

## Notes

Design § 10 leaves the scheme open ("UUIDv7, content-derived identifiers where appropriate");
UUIDv7 for minted ids, content-derived for observations and evidence (which are what they hash).
Nothing in this story names a graph concept beyond the id types. The ESS domain these types
belong to is `ekr.kernel`; the crate is `ekr-core` because Cargo needs them below `ekr-graph`.
Uses only dependencies `story:workspace-crate-skeleton` declared for `ekr-core`; `Cargo.lock` is
not in scope.
