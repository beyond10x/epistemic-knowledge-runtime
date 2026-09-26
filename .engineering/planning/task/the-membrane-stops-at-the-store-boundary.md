---
format: aep.planning-md/2
id: task:the-membrane-stops-at-the-store-boundary
kind: task
status: implemented
title: A candidate's document deserialises into a canonical node, because the two carry identical bytes
relations:
- serves: vision:o2
- derived_from: story:seed-and-explain
- derived_from: story:kernel-validated-seed
revision: 7
---
## What is wrong

`ekr_graph::CanonicalValue` serialises **through** `ekr_ontology::Value`
(`#[serde(into = "Value", try_from = "Value")]`), so a node in canonical state and a candidate node
in the incubation forest produce **byte-identical documents**. Nothing on a stored node records
which space it came from: `ekr.graph.Space` is declared on `GraphRoot`, not on the node.

So a candidate's document deserialises into a canonical `Node` without complaint whenever it happens
to carry no float. The type that carries the membrane is chosen by the caller at the deserialisation
site, not by the bytes.

Found by the adversary of wave p1-05, unit 0, pass 2. It declined to write a case because asserting
a refusal would invent a requirement nobody has stated — which is the right call, and is why this is
a task rather than a finding against that unit.

## Why the guarantee still holds where it was claimed

`architecture-decision-record:0005-float-is-not-canonical`'s amendment says the address exists
exactly where canonical state does, and that is true **inside Rust**: `Node<Value>` has no
`Canonical` implementation and a `trybuild` case holds it. The claim was never about bytes on disk,
and the amendment does not say it was. What is missing is anything saying where the boundary is.

## What reaches it

`story:eventlog-store` is the first crate to write a node to bytes and read one back, and
`story:seed-and-explain` and the import path of P3 are the first to read bytes somebody else wrote.
An import that round-trips a candidate through storage and reads it back as canonical is the
reachable failure, and it is silent.

## What closes this

A decision, then a case. The options, smallest first:

- **the store never deserialises into a canonical type from an untrusted document** — it reads into
  `Node<Value>` and converts, so the refusal happens at one named place;
- or a node **records its space**, which makes the document self-describing and costs a field on
  three entities in `graph.yaml`;
- or the two instantiations get **distinct serde representations**, which makes the bytes carry the
  membrane and breaks any document already written — free today, since nothing is stored.

The first is the smallest and is probably right. It is a decision, and it belongs in
`story:eventlog-store`'s brief rather than being discovered.


## Correction, 2026-09-21, after adversary pass 1 on unit 1

**The premise above is half wrong, and the half that is wrong is the expensive half.**

This task says "nothing on a stored node records which space it came from: `ekr.graph.Space` is
declared on `GraphRoot`, not on the node." That is true of a **node**, and the store's crossing does
not take a node. It takes a `GraphDocument`, which is a `GraphRoot` plus four maps — and
`GraphRoot.space` is exactly the marker this task says does not exist. The document **is**
self-describing. The crossing had the marker in hand and copied it through without reading it.

So the smallest option, which the unit took, is smaller still than either of us thought: it is one
comparison at the one named crossing, not a new field on three entities and not a change to the
serde representation. Unit 1's correction round 1 adds it, with a refusal named for it.

What survives of the original finding: two **nodes** are still indistinguishable byte for byte, so a
node lifted out of a transient document and pasted into a canonical one is still undetectable. That
is a smaller hole than the one described above, it is not reachable through any public function of
`ekr-store` once the root is checked, and it is what the remaining two options in this task would
close.

The coordinator wrote the wrong premise from a true sentence about a different type. The adversary
found it by reading what the crossing actually takes rather than what the task said it takes.

## Reconciliation, 2026-09-22

The stale blocks edge to the implemented provider story has been removed. The existing root-space check closes the original whole-document claim, but seed graph semantic validation remains absent, as reproduced by the retained review cases. Seed admission is the kernel's responsibility under the existing design and story; no additional ownership ADR is needed. Keep this residual open until the real kernel seed path refuses invalid documents, and then record the executable evidence.

## Residual closed by the kernel seed path

The reconciled acceptance is implemented by story:kernel-validated-seed and its
published PR #8, retained through the original-format freeze. Seed input is read
as transient values; the fixed kernel validators, actual bootstrap actors and
retained evidence establish canonical admission. Store replay delegates the full
seed envelope to that same authority. A standalone deserializable node does not
itself confer authority to publish canonical state.

Executable cases in crates/ekr-kernel/tests/seed.rs include
`a_seed_with_a_dangling_edge_is_refused_by_both_backends`,
`a_seed_with_an_undeclared_type_is_refused_by_both_backends`,
`a_seed_with_a_caller_verdict_is_refused_by_both_backends`,
`named_seed_refusals_write_neither_the_object_nor_the_revision` and
`legacy_and_tampered_seed_envelopes_are_preserved_but_never_admitted`.
The evidence seed reopen and execution-context tampering cases bind the real
kernel authority on both providers. All run again in the current kernel/store
adoption suite; the published wave page retains original integration evidence.

This closes the seed boundary residual, without claiming the forthcoming durable
transaction writer, original-history migration or later import policy complete.
