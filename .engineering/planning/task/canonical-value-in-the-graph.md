---
format: aep.planning-md/1
id: task:canonical-value-in-the-graph
kind: task
status: active
title: A value admissible in canonical state, so an assertion can be content-addressed
relations:
- blocks: story:eventlog-store
- blocks: story:transaction-and-validators
- serves: vision:o2
revision: 3
---
## What this implements

`architecture-decision-record:0005-float-is-not-canonical`, accepted. It is carried as a task rather
than a story because it is a prerequisite both units of wave p1-05 need and neither story owns the
surface it touches.

## Acceptance

`ekr_graph::Assertion` has a total `Canonical` implementation, and a value carrying a `Float` at any
depth cannot inhabit an assertion, a node property or an edge property.

## Scope

- `crates/ekr-ontology/src/value.rs` — whether a `Value` is admissible in canonical state,
  recursively: a `List` or `Record` containing a `Float` is not
- `crates/ekr-graph/src/value.rs` — the newtype only an admissible value inhabits, its
  `TryFrom<Value>`, its refusal type, and its total `Canonical` implementation
- `crates/ekr-graph/src/assertion.rs` — `Object::Value` carries the newtype; `Assertion` gains
  `Canonical`
- `crates/ekr-graph/src/node.rs`, `edge.rs` — the two property maps carry the newtype
- `crates/ekr-graph/src/lib.rs` — the re-exports
- the `ekr-graph` and `ekr-ontology` test files whose fixtures move with the change

## Tests

- an admissible value round-trips through the newtype and encodes to stable bytes
- a `Float`, a `List` containing one and a `Record` containing one are each refused, and the
  refusal names the path
- two assertions equal in every field hash equally, and two differing in one field do not
- the existing `ekr-graph` guards stay green: the projection guard, the membrane, the snapshot
  reads

## Notes

`Float` stays legal in `ekr_ontology::Value`. The transient graph is not content-addressed and an
approximate measurement is a reasonable thing to hold before it becomes canonical.

The kernel-side half — a type validator that refuses an operation carrying an inadmissible value —
is `story:transaction-and-validators`' and is not this task's.
