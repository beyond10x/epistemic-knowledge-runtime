---
format: aep.planning-md/2
id: story:kernel-identity-and-hashing
kind: story
status: implemented
title: Stable identity and content hashing
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:workspace-crate-skeleton
- implements: executable-system-specification:ekr-v1
- serves: vision:o2
scope:
- confidence: cited
  path: crates/ekr-core/src/canonical.rs
- confidence: cited
  path: crates/ekr-core/src/hash.rs
- confidence: cited
  path: crates/ekr-core/src/identity.rs
- confidence: cited
  path: crates/ekr-core/src/lib.rs
- confidence: cited
  path: crates/ekr-core/tests/adversary2_encoding_vector.rs
- confidence: cited
  path: crates/ekr-core/tests/adversary2_public_surface.rs
- confidence: cited
  path: crates/ekr-core/tests/adversary2_revision_number_text_form.rs
- confidence: cited
  path: crates/ekr-core/tests/adversary_encoder_order.rs
- confidence: cited
  path: crates/ekr-core/tests/adversary_encoding_vector.rs
- confidence: cited
  path: crates/ekr-core/tests/adversary_id_text_form.rs
- confidence: cited
  path: crates/ekr-core/tests/canonical_encoding.rs
- confidence: cited
  path: crates/ekr-core/tests/content_hash.rs
- confidence: cited
  path: crates/ekr-core/tests/identity_serde.rs
- confidence: cited
  path: crates/ekr-core/tests/public_surface.rs
- confidence: cited
  path: crates/ekr-core/tests/rename_stability.rs
revision: 21
---
## Context

Design § 6.4 and § 10: human-readable names are not identities; every persistent object carries a
stable id that survives renames, aliases, merges and schema migration. Design § 57: raw payloads
and immutable artifacts are content-addressed. Everything above builds on these two primitives, so
they come first, alone, in the bottom crate `ekr-core`, which depends on no other crate of the
workspace.

## Acceptance

Two values that share a name never share an id: over arbitrary pairs of names, minted `NodeId`s
are always distinct.

## Tests the story ships

- `hash.rs`: the same canonical bytes produce the same `ContentHash` on every run, and the hex
  form round-trips.
- `canonical.rs`: two `BTreeMap`s with the same entries in different insertion order encode to the
  same bytes.
- Every id type round-trips through serde as a string and refuses a malformed one.

## Scope

- `crates/ekr-core/src/identity.rs` — the id newtypes of `systems/ekr/domains/kernel.yaml`
  (`AgentId`, `TransactionId`, `RevisionId`, `IssueId`), `ontology.yaml` (`TypeId`, `PropertyId`,
  `SchemaVersionId`) and `graph.yaml` (`GraphRootId`, `NodeId`, `EdgeId`, `AssertionId`,
  `SupportId`, `EvidenceId`, `ObservationId`), each a `u128` newtype minted as UUIDv7, with serde;
  and `RevisionNumber`, which is not one of them — `kernel.yaml` declares it `of: Integer` and
  design § 34 carries it as a `u64`, so it is a counter with a successor, not a minted id
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

`ekr.store.SnapshotId` is the tree's fifteenth `of: Uuid` newtype and is deliberately absent: it
belongs to the fourth domain and to `ekr-store`, and this story names only kernel, ontology and
graph.

The `RevisionNumber` line above was corrected on 2026-09-21, during wave p1-02, after the
implementor checked it against `systems/ekr/domains/kernel.yaml` and design § 34 before building
on it.

The acceptance was restated on 2026-09-21, during wave p1-02, after the adversary proved the
original could not fail. It read "a property test renames a node's `canonical_name` a thousand
times and its `NodeId` never changes", and `Node` does not exist in `ekr-core`: the fixture
standing in for it renamed its own field and compared a `Copy` id to a copy of itself, so the case
passed with `mint()` mutated to return a constant for every id. The invariant it was reaching for
— an id survives a rename — is stated where a `Node` exists, in
`story:graph-model-and-assertions`. What `ekr-core` can verify, and now asserts, is the half that
lives here: an id is not a function of any name.
