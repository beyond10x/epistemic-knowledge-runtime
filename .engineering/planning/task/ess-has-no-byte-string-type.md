---
format: aep.planning-md/1
id: task:ess-has-no-byte-string-type
kind: task
status: draft
title: A byte is declared as an unbounded Integer, because ess/1 has no byte-string type
relations:
- serves: vision:o2
revision: 4
---
## What is wrong

`ess/1` has no byte-string type. `systems/ekr/domains/store.yaml` needs one — `ekr.store.ObjectStored`
carries the bytes an object was stored as — and declares it `List<Integer>`, which says a byte is an
unbounded integer.

That is the first declaration in this system to need bytes at all, which is why nothing has hit it
before. `ekr.graph.ObservationContent` has a `Blob` variant (design amendment 86) and it is an enum
variant with no payload declared, so it has the same gap and does not yet feel it.

## Why it matters, and why it is not urgent

A projection from this document would generate a list of arbitrary-precision integers where a byte
array belongs. Nothing generates from it today — `ess generate` is not in this repository's gate —
so the cost is a document that says something slightly false, not a wrong program.

It becomes urgent the first time anything projects a schema, an OpenAPI document or a wire format
from these domains, which `docs/roadmap.md` puts in a later phase.

## What closes this

One of:

- `ess/1` gains a `Bytes` scalar, which is an ESS change in `beyond10x/ess` and not this
  repository's, and would be the right answer for every consumer rather than only this one;
- or the field is declared a `String` with a stated encoding — hexadecimal or base64 — and the
  encoding written down, which makes the document honest at the cost of putting an encoding
  decision in a store domain;
- or the bytes leave the event and the object store keeps them outside the log, which changes the
  design rather than the declaration.

The first is the real answer. Ask the ESS repository before doing either of the others.

## Where it is named

`systems/ekr/domains/store.yaml`, in the comment above the field, so a reader of the declaration
finds this rather than assuming the type was chosen.

## Upstream capability rechecked

The premise is stale: the pinned ESS source at a5f1bea13294510819b266561c83be9509e6ba57
includes Primitive::Bytes and synthesizes base64 witnesses (ess-conformance/src/witness.rs).
This is source inspection, not execution of this repository's byte projection.
The remaining work is to select and bind the actual ObjectStored wire representation and update
its projection guard. Changing List<Integer> to Bytes changes JSON representation to base64;
coordinate that with versioned records rather than silently changing existing event bytes.
Keep the task open until the representation and executable agreement case land.

## Wave p1-14 rescope

Rescoped in wave p1-14 (2026-09-23): the plan review of 72779f9 found this task's premise already
resolved in `systems/` at b2b64f8. The only remaining obligation is a binding case in the owning
crate's `tests/domain_projection.rs` that fails if the declaration and the Rust type drift. If that
case already exists, the wave cites it and archives this task.

## Scope

Derived 2026-09-23 by `story-scoper` (wave p1-14). Verdict: already-held.

- `systems/ekr/system.yaml:1` is `format: ess/7`; `List<Integer>` appears nowhere under `systems/`; `Bytes` is used (`store.yaml:53-54`, `graph.yaml:171`) — cited
- `ekr.store.ObjectStored` (`store.yaml:280`) is metadata-only; payload bytes go through provider blob bindings — cited
- held by `crates/ekr-store/tests/domain_projection.rs::every_event_the_crate_writes_carries_the_fields_the_domain_declares` (exact field-set equality on the written body) and `crates/ekr-store/tests/current_legacy_object_refusal.rs` — cited
- not covered: the store guard compares field names, not types; `ekr.graph.ObservationContent::Blob` has no payload case — cited, left for P2 (Blob, amendment 86)
- Confidence: high
