---
format: aep.planning-md/1
id: task:canonical-newtype-discriminant
kind: task
status: active
title: The canonical encoding tags sum types and stays structural for newtypes
relations:
- informed_by: review-result:adversary-identity-pass-1
- derived_from: story:kernel-identity-and-hashing
- informed_by: story:graph-model-and-assertions
- serves: vision:o2
revision: 5
---
## Context

Found by the adversary in wave p1-02 (`review-result:adversary-identity-pass-1`, finding 5). The
canonical encoding of `ekr-core` is structural: a newtype encodes as the shape it wraps. So
`RevisionNumber(7)` produces the same bytes as a bare `7u64`, and a `NodeId` produces the same
bytes as an `EdgeId` over the same UUID. Verified by a scratch probe:
`ContentHash::of(&RevisionNumber::new(7)) == ContentHash::of(&7u64)`.

## Decision, 2026-09-21, wave p1-03

**Sum types carry a required variant tag. Newtypes carry no discriminant, and that is the stated
contract rather than an open question.**

The two halves of the original question turn out to have different answers because they have
different blast radii.

**Sum types: a tag, enforced by a helper.** `RevisionEvent` (`story:graph-model-and-assertions`,
wave p1-04) has seven variants. Under a structural encoding, two variants carrying the same payload
shape encode identically unless every `encode` implementation remembers to write a discriminating
byte, and nothing enforces that. A convention an implementation can forget is the class of defect
`Encoder` exists to remove — its own module doc says the helpers are there "so an implementation
cannot forget a tag or a length". So `Encoder` gains a `variant` writer, and it becomes the only
sanctioned way to encode a sum type.

**Newtypes: no discriminant.** The hazard requires hashing a *bare* newtype at top level and
comparing it against a bare primitive. Every artefact this runtime content-addresses — an
observation, a piece of evidence, an assertion, a transaction, a revision root — is a structured
value whose own encoding carries its field structure, and a field's position already distinguishes
it from a field of another type in the same position. No artefact is a bare `RevisionNumber`.

Against that narrow and unreachable hazard, a per-type discriminant costs a stable tag per type,
permanently. Derived from the Rust type name, a rename silently moves every address that type ever
reached. Hand-assigned, it is a registry that can never be reordered or reused, maintained across
every crate that implements `Canonical`. That is a durable cost for a collision the runtime's own
artefacts cannot reach.

**This decision moves no recorded address.** No sum type is encoded anywhere in the crate today —
`Option` already carries its own `NONE`/`SOME` tags — so adding the writer changes no existing
bytes and no pinned digest. Deciding it in p1-03 rather than p1-06 is what makes it free; by p1-06
the graph, the store's folds and the kernel's validation hashes would all rest on it.

## Acceptance

Two sum-type values whose variants differ but whose payloads are byte-identical encode differently,
and `Encoder`'s variant writer is the only path in the crate that produces a variant tag.

## Scope

- `crates/ekr-core/src/canonical.rs` — `tag::VARIANT`, `Encoder::variant`, and rule 5 rewritten
  from an open question to the settled contract
- `crates/ekr-core/tests/canonical_encoding.rs` — the acceptance above

## Notes

The mechanism lands in wave p1-04, with `story:graph-model-and-assertions`, because
`RevisionEvent` is the first sum type to need it and a helper with no caller is a helper nobody has
tested against a real shape. That story's scope and its shipped tests carry it. The `blocks` edge on
`story:commit-and-revision-lineage` is therefore taken back: what blocked that story was the
undecided question, and it is decided.
