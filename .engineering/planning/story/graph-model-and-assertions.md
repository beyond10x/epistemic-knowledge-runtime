---
format: aep.planning-md/1
id: story:graph-model-and-assertions
kind: story
status: draft
title: 'Graph model: nodes, edges, bitemporal assertions, evidence, the canonical/transient membrane'
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:ontology-types-and-values
- implements: executable-system-specification:ekr-v1
scope:
- confidence: inferred
  path: crates/ekr-graph/src/assertion.rs
- confidence: inferred
  path: crates/ekr-graph/src/canonical.rs
- confidence: inferred
  path: crates/ekr-graph/src/edge.rs
- confidence: inferred
  path: crates/ekr-graph/src/events.rs
- confidence: inferred
  path: crates/ekr-graph/src/evidence.rs
- confidence: inferred
  path: crates/ekr-graph/src/lib.rs
- confidence: inferred
  path: crates/ekr-graph/src/node.rs
- confidence: inferred
  path: crates/ekr-graph/src/root.rs
- confidence: inferred
  path: crates/ekr-graph/src/snapshot.rs
- confidence: inferred
  path: crates/ekr-graph/src/transient.rs
- confidence: inferred
  path: crates/ekr-graph/tests/compile_fail
revision: 8
---
## Context

Design § 13–14 make the assertion the fundamental primitive, with valid time and transaction time
as separate ranges; § 17 and § 36 give its validation and status states; § 21–23 give the canonical
and transient graphs and ask that a `Canonical → Transient` reference be impossible to represent;
§ 34 gives the revision root; § 71 gives the snapshot a reader holds. This story is the data model
of the graph, including the event vocabulary the kernel publishes and the store persists. It has
no writer: mutation arrives in the next stories.

## Acceptance

A `GraphSnapshot` over the § 65 fixture (Alice `CEO_OF` Acme until 2026-03-12, Bob from then)
answers `active()` with Bob.

## Tests the story ships

- The same snapshot answers `valid_at(2025-06-01)` with Alice.
- A `trybuild` compile-fail test: `CanonicalGraph` cannot hold a `TransientRef`.
- Neither `active()` nor `valid_at` returns a `Retracted` or `Superseded` assertion.
- `RevisionEvent` round-trips through serde for every variant.
- A `Node` renamed a thousand times keeps its `NodeId` (design § 6.4). This is the invariant
  `story:kernel-identity-and-hashing` could not state: `Node` did not exist in `ekr-core`, so its
  fixture stood in for one and the case could not fail. It is checkable here, against the real type.

## Scope

- `crates/ekr-graph/src/node.rs`, `edge.rs` — `Node` (with `type_state` from amendment 87),
  `Edge` (`systems/ekr/domains/graph.yaml`)
- `crates/ekr-graph/src/assertion.rs` — `Assertion`, `Subject`, `Predicate`, `Object`,
  `TemporalRange`, `ValidationState`, `AssertionStatus`
- `crates/ekr-graph/src/evidence.rs` — `Evidence`, `EvidenceSource`, `Observation`,
  `ObservationContent` (including `Blob`, amendment 86), `Support`
- `crates/ekr-graph/src/root.rs` — `GraphRoot` (space, schema version, parent, created_at — the
  fields `graph.yaml` declares, nothing more), `Root` (the revision root of design § 34)
- `crates/ekr-graph/src/events.rs` — `RevisionEvent`: `Seeded`, `TransactionProposed`,
  `TransactionValidated`, `TransactionRejected`, `TransactionStale`, `RevisionCommitted`, with
  the field shapes of the `ekr.kernel` events in `systems/ekr/domains/kernel.yaml`, typed with
  the `ekr-core` newtypes; a data type only — the kernel fills and publishes it
  (`components.yaml`), the store persists it
- `crates/ekr-graph/src/canonical.rs`, `transient.rs` — `CanonicalGraph`, `TransientGraph`,
  `CanonicalRef<T>`, `TransientRef<T>` (design § 23)
- `crates/ekr-graph/src/snapshot.rs` — `GraphSnapshot { graph, revision }` as design § 71 gives
  it, with the two reads the § 65 example needs: `active()` and `valid_at(Timestamp)`
- `crates/ekr-graph/src/lib.rs`
- `crates/ekr-graph/tests/compile_fail/` (trybuild)

## Notes

Depends on `story:ontology-types-and-values`. Crate dependencies, as the skeleton declared them:
`ekr-graph` → `ekr-core`, `ekr-ontology`. `RevisionEvent` and `Root` live here so that
`ekr-kernel` (publisher, above this crate) and `ekr-store` (persister, above this crate) share
one definition; neither of those two depends on the other. `TransientGraph` exists so the
membrane is typed from the start; the incubation forest that uses it, and the `KnowledgeState`
ladder of design § 29 on `GraphRoot`, are P3. The `QueryScope` selector of design § 47 is P4;
P1's snapshot answers only the two reads above. Uses only dependencies
`story:workspace-crate-skeleton` declared for `ekr-graph`.
