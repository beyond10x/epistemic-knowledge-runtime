---
format: aep.planning-md/1
id: task:graph-properties-keyed-by-string
kind: task
status: draft
title: Node and Edge properties are declared keyed by String, and the crate keys by PropertyId
relations:
- serves: vision:o2
revision: 2
---
## What is wrong

`systems/ekr/domains/graph.yaml:173-174` declares `ekr.graph.Node.properties` and `:200-201`
declares `ekr.graph.Edge.properties` as `Map<String, ekr.graph.TypedValue>`. The key is a
`String`. If that `String` is the property's **name**, the declaration contradicts this
repository's invariant 3 in `AGENTS.md` — names are not identities — because a property renamed by
a schema transaction would orphan every value stored under the old key.

`crates/ekr-graph/src/node.rs` and `edge.rs` key by `PropertyId` and say so in their doc comments.

## What is not yet established

Whether `ess/1` can express a map keyed by a declared identity type at all. If it cannot, this is
the same boundary as `ValueType`'s flattening in `ontology.yaml` — a limit of the specification
language rather than a disagreement — and the fix is a sentence in the document saying the key is
the `PropertyId`'s text form, not a change to the type.

Nobody has read the `ess/1` grammar for this. Do that before proposing either change.

## What closes this

A sentence in `graph.yaml` stating what the key is, and a case in
`crates/ekr-graph/tests/domain_projection.rs` that goes red if the crate and the document drift
again.

## Wave p1-14 rescope

Rescoped in wave p1-14 (2026-09-23): the plan review of 72779f9 found this task's premise already
resolved in `systems/` at b2b64f8. The only remaining obligation is a binding case in the owning
crate's `tests/domain_projection.rs` that fails if the declaration and the Rust type drift. If that
case already exists, the wave cites it and archives this task.
