---
format: aep.planning-md/1
id: task:canonical-newtype-discriminant
kind: task
status: draft
title: Decide whether the canonical encoding carries a type discriminant
relations:
- informed_by: review-result:adversary-identity-pass-1
- derived_from: story:kernel-identity-and-hashing
- blocks: story:commit-and-revision-lineage
revision: 1
---
## Context

Found by the adversary in wave p1-02 (`review-result:adversary-identity-pass-1`, finding 5). The
canonical encoding of `ekr-core` is structural: a newtype encodes as the shape it wraps. So
`RevisionNumber(7)` produces the same bytes as a bare `7u64`, and a `NodeId` produces the same
bytes as an `EdgeId` over the same UUID. Verified by a scratch probe:
`ContentHash::of(&RevisionNumber::new(7)) == ContentHash::of(&7u64)`.

Nothing is wrong today, and that is why it is a task rather than a finding on the unit: no
composite type exists yet to hold such a field. The first one arrives with
`story:graph-model-and-assertions`.

## Why it matters when it matters

A content address is supposed to move when the value it addresses changes. Under a structural
encoding, a later change of a struct field from `u64` to `RevisionNumber`, or from `NodeId` to
`EdgeId`, keeps the address — a type change the compiler treats as significant and the address
does not. The revision root is a hash over that encoding, so two revisions that differ only in
such a field would be indistinguishable by root.

## Acceptance

`ContentHash::of(&RevisionNumber::new(7))` differs from `ContentHash::of(&7u64)`, and the two
id types over one UUID differ from each other.

## Notes

The obvious shape is a type discriminant in the encoding — a stable name or number per newtype,
written before the wrapped value. It is a breaking change to every recorded address, so it is
cheapest before the first canonical commit exists and most expensive after. Decide it no later
than `story:commit-and-revision-lineage`, which is what first writes a root hash anybody keeps.

The alternative is to accept the structural encoding and document it as the contract, which is
what `canonical.rs` does today at the coordinator's instruction. That choice is not yet made.

## Scope

- `crates/ekr-core/src/canonical.rs`
- `crates/ekr-core/tests/canonical_encoding.rs`
