---
format: aep.planning-md/2
id: task:typed-value-canonical-field-is-undecided
kind: task
status: archived
title: TypedValue.canonical is declared a String and was described as the content-hash bytes, which it cannot be
relations:
- serves: vision:o2
revision: 6
---
## What is wrong

`systems/ekr/domains/graph.yaml` declares `ekr.graph.TypedValue` with a `canonical: String` field,
and its comment said that string is "the same bytes the content hashes are computed over".

It is not, and it cannot be. Content hashes are computed over the binary encoding of
`crates/ekr-core/src/canonical.rs` — tagged, length-prefixed, whose own module doc says "the bytes
are an internal format, not a wire format: nothing outside this runtime reads them, and they are not
a serialisation — there is no decoder, because a hash never needs one." A `String` cannot carry
them, and nothing in the workspace converts between the two.

Found by the coordinator in wave p1-05 while correcting a different false claim in the same comment.
That is two false sentences in one comment, in a file that four crates project from.

## What is undecided

`TypedValue` has no Rust type bound to it — `crates/ekr-graph/tests/domain_projection.rs` binds it
to the empty list — so nothing reads the field today and nothing is broken. What is missing is a
decision about what `canonical` is for:

- a **human-readable rendering** for an operator surface or an export, in which case it is not
  canonical and should not be called that;
- or a **textual canonical form** with its own rules, in which case those rules have to be written
  and it has to agree with the binary encoding on equality — two values equal under one are equal
  under the other;
- or the field is **removed** and `TypedValue` carries the kind and the value's own serde shape,
  which is what `ekr_ontology::Value` already has and what the crate actually uses on the wire.

The third is the smallest and is probably right, but it is a decision and nobody has taken it.

## What closes this

A decision, recorded, and `graph.yaml` matching it. If the field survives, a case binding
`ekr.graph.TypedValue` to a Rust type so that `domain_projection.rs` stops carrying it as an
unbound declaration.

## Why it has not bitten yet

Nothing deserialises a `TypedValue`. The Rust wire form for a property value is
`ekr_ontology::Value`'s adjacently-tagged shape, which goes through `CanonicalValue`'s
`#[serde(try_from)]`. The day a store or an export writes a `TypedValue` column from this
declaration is the day the sentence would have been believed.

## Reconciliation, 2026-09-22

The false claim that the String carries binary hash bytes was corrected, but the residual projection remains explicit: `crates/ekr-graph/tests/domain_projection.rs` records TypedValue as an unbound flattening. A corrected comment alone does not supply an executable wire representation. Keep the residual projection work open and reconcile it with the versioned persisted-contract work before durable application. The approved plan does not authorize inventing canonical text or silently migrating existing values.

## Wave p1-14 rescope

Rescoped in wave p1-14 (2026-09-23): the plan review of 72779f9 found this task's premise already
resolved in `systems/` at b2b64f8. The only remaining obligation is a binding case in the owning
crate's `tests/domain_projection.rs` that fails if the declaration and the Rust type drift. If that
case already exists, the wave cites it and archives this task.

## Scope

Derived 2026-09-23 by `story-scoper` (wave p1-14). Verdict: already-held.

- `systems/ekr/domains/graph.yaml:165-171` declares `ekr.graph.TypedValue { kind: ekr.graph.CanonicalValueKind, canonical_bytes: Bytes }`; the `canonical: String` field is gone — cited
- bound to `CanonicalValue` (`crates/ekr-graph/src/value.rs:46`) in `crates/ekr-graph/tests/domain_projection.rs:464` (PROJECTIONS) and `:517` (FUSIONS), since `094f043` — cited
- held by `crates/ekr-graph/tests/domain_projection.rs::every_declaration_of_the_domain_is_carried_field_for_field` — cited
- stale doc: `crates/ekr-graph/src/node.rs:28` still says `TypedValue { kind, canonical }` — cited
- Confidence: high

## Wave p1-14 close

Archived in wave p1-14: `ekr.graph.TypedValue` declares `canonical_bytes: Bytes`, bound to `CanonicalValue` since `094f043`.

- `crates/ekr-graph/tests/domain_projection.rs::every_declaration_of_the_domain_is_carried_field_for_field`: `test result: ok` (bindings unit run, 2026-09-23).
