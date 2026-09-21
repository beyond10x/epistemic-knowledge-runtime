---
format: aep.planning-md/1
id: story:graph-model-and-assertions
kind: story
status: implemented
title: 'Graph model: nodes, edges, bitemporal assertions, evidence, the canonical/transient membrane'
relations:
- decomposes: epic:p1-kernel-ontology-core
- depends_on: story:ontology-types-and-values
- implements: executable-system-specification:ekr-v1
- serves: vision:o2
scope:
- confidence: cited
  path: crates/ekr-core/src/canonical.rs
- confidence: cited
  path: crates/ekr-core/src/lib.rs
- confidence: cited
  path: crates/ekr-core/src/time.rs
- confidence: cited
  path: crates/ekr-core/tests/canonical_encoding.rs
- confidence: cited
  path: crates/ekr-core/tests/public_surface.rs
- confidence: cited
  path: crates/ekr-core/tests/timestamp.rs
- confidence: cited
  path: crates/ekr-graph/src/assertion.rs
- confidence: cited
  path: crates/ekr-graph/src/canonical.rs
- confidence: cited
  path: crates/ekr-graph/src/edge.rs
- confidence: cited
  path: crates/ekr-graph/src/events.rs
- confidence: cited
  path: crates/ekr-graph/src/evidence.rs
- confidence: cited
  path: crates/ekr-graph/src/lib.rs
- confidence: cited
  path: crates/ekr-graph/src/node.rs
- confidence: cited
  path: crates/ekr-graph/src/root.rs
- confidence: cited
  path: crates/ekr-graph/src/snapshot.rs
- confidence: cited
  path: crates/ekr-graph/src/transient.rs
- confidence: cited
  path: crates/ekr-graph/tests/adversary2_guard_bounds_and_ranges.rs
- confidence: cited
  path: crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs
- confidence: cited
  path: crates/ekr-graph/tests/compile_fail
- confidence: cited
  path: crates/ekr-graph/tests/domain_projection.rs
- confidence: cited
  path: crates/ekr-graph/tests/evidence_and_observations.rs
- confidence: cited
  path: crates/ekr-graph/tests/membrane.rs
- confidence: cited
  path: crates/ekr-graph/tests/node_identity.rs
- confidence: cited
  path: crates/ekr-graph/tests/revision_events.rs
- confidence: cited
  path: crates/ekr-graph/tests/snapshot_reads.rs
- confidence: cited
  path: crates/ekr-ontology/src/lib.rs
- confidence: cited
  path: crates/ekr-ontology/src/schema.rs
- confidence: cited
  path: crates/ekr-ontology/src/value.rs
- confidence: cited
  path: crates/ekr-ontology/tests/domain_projection.rs
- confidence: cited
  path: crates/ekr-ontology/tests/hierarchy_specificity.rs
- confidence: cited
  path: crates/ekr-ontology/tests/inheritance_and_declaration_coherence.rs
- confidence: cited
  path: crates/ekr-ontology/tests/ontology_load.rs
- confidence: cited
  path: crates/ekr-ontology/tests/type_hierarchy.rs
- confidence: cited
  path: crates/ekr-ontology/tests/value_type_checking.rs
- confidence: cited
  path: crates/ekr/tests/graph_assertion_serde.rs
- confidence: cited
  path: crates/ekr/tests/graph_events_serde.rs
- confidence: cited
  path: systems/ekr/domains/graph.yaml
revision: 21
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
answers `valid_at(t)` with Bob for every `t` at or after the handover, with Alice for every `t`
before it, and the handover instant itself belongs to exactly one of them.

## Tests the story ships

- A `trybuild` compile-fail test: `CanonicalGraph` cannot hold a `TransientRef`, and cannot reach
  one through a `CanonicalRef` parameterised over a transient type.
- `valid_at` never returns a `Retracted` or `Superseded` assertion, at any `t`.
- `RevisionEvent` round-trips through serde for every variant.
- Two `RevisionEvent` variants carrying byte-identical payloads encode differently, because each
  carries a variant tag. `Encoder::variant` is added in `ekr-core` for this story and is the only
  path in the workspace that writes one.
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
  it, with the one read the § 65 example needs: `valid_at(Timestamp)`
- `crates/ekr-graph/src/lib.rs`
- `crates/ekr-graph/tests/compile_fail/` (trybuild)

Added on 2026-09-21, before wave p1-04 dispatched, by
`architecture-decision-record:0004-timestamp-in-ekr-core`:

- `crates/ekr-core/src/time.rs` — `Timestamp`, a newtype over `i64` milliseconds since the Unix
  epoch, with its `Canonical` impl and its decimal text form
- `crates/ekr-core/src/lib.rs` — the re-export
- `crates/ekr-core/tests/timestamp.rs` — its cases, which the workspace public-surface guard
  requires
- `crates/ekr-ontology/src/value.rs`, `schema.rs` — the three sites that carry a bare `i64` today:
  `Value::Timestamp`, `ValueKind::Timestamp`'s payload, and `SchemaVersion::created_at`
- `crates/ekr-ontology/tests/domain_projection.rs` — the guard that this crate's citations of
  `systems/ekr/domains/ontology.yaml` stay true, which the change above moves

Test files, written during wave p1-04 and read from the merged tree:

- `crates/ekr-graph/tests/snapshot_reads.rs` — the restated acceptance in both directions, the
  handover instant, and what an empty graph answers
- `crates/ekr-graph/tests/evidence_and_observations.rs` — evidence, its sources and its support
- `crates/ekr-graph/tests/membrane.rs` and `tests/compile_fail/` — three `trybuild` cases holding
  `AGENTS.md` invariant 2
- `crates/ekr-graph/tests/revision_events.rs` — the six variants, their indices, and that
  declaration order equals the numbering
- `crates/ekr-graph/tests/node_identity.rs` — a thousand renames against the real `Node`
- `crates/ekr-graph/tests/domain_projection.rs` — every declaration `graph.yaml` carries, bound to
  the Rust types that project it, with each fusion carrying its reason
- `crates/ekr-graph/tests/adversary_snapshot_and_assertion.rs` and
  `tests/adversary2_guard_bounds_and_ranges.rs` — the two adversary passes, re-aimed
- `crates/ekr-core/tests/timestamp.rs` — ADR 0004's newtype, its text form and its bounds
- `crates/ekr/tests/graph_events_serde.rs` and `tests/graph_assertion_serde.rs` — the round trips,
  in the only crate that has both a format and the graph

`systems/ekr/domains/graph.yaml` is in scope for one hunk, written by the coordinator: the comment
at lines 50-51 claimed a payload carrier for four validation states and there is one.

## Notes

Depends on `story:ontology-types-and-values`. Crate dependencies, as the skeleton declared them:
`ekr-graph` → `ekr-core`, `ekr-ontology`. `RevisionEvent` and `Root` live here so that
`ekr-kernel` (publisher, above this crate) and `ekr-store` (persister, above this crate) share
one definition; neither of those two depends on the other. `TransientGraph` exists so the
membrane is typed from the start; the incubation forest that uses it, and the `KnowledgeState`
ladder of design § 29 on `GraphRoot`, are P3. The `QueryScope` selector of design § 47 is P4;
P1's snapshot answers only the two reads above. Uses only dependencies
`story:workspace-crate-skeleton` declared for `ekr-graph`.

`task:canonical-newtype-discriminant` was settled on 2026-09-21, in wave p1-03: sum types carry a
variant tag, newtypes carry no discriminant. `RevisionEvent` is the first sum type the runtime
encodes, so this story adds `Encoder::variant` and `tag::VARIANT` to
`crates/ekr-core/src/canonical.rs` and the case above to that crate's tests. No existing encoding
changes — nothing in the workspace is a sum type today, so no recorded address moves.

`Timestamp` was settled before dispatch by
`architecture-decision-record:0004-timestamp-in-ekr-core`: an `ekr-core` newtype over `i64`
milliseconds. The roadmap had put it in `ekr-graph`; `ontology.yaml:100-101` declares
`SchemaVersion.created_at` of type `Timestamp` and `ekr-ontology` does not depend on `ekr-graph`,
so `ekr-graph` would have left one domain scalar with two unrelated Rust representations.


## The acceptance was restated on 2026-09-21, after adversary pass 1

It read "answers `active()` with Bob", and `active()` as built was `is_current()` and valid time
with no known end — which is the same read as `valid_at(i64::MAX)`, measured over 56 records of
every shape. Two ordinary facts fall on the wrong side of it: a fixed-term fact that is true today
is excluded because its end is known, and an announced successor whose tenure has not begun is
included. Neither is the current world, and "the current-world query" is what design § 65 asks for.

The runtime has no clock and P1 declares no time crate, so a clock-free current-world read cannot
exist. `valid_at(t)` is the whole of it: asked at the caller's now it is the current-world query,
asked at any other instant it is the historical query. That is what bitemporality means, and one
function carrying both roles is the honest shape. `active()` is removed rather than renamed —
nothing in P1 needs "valid time with no known end", and a public read whose name asserts something
the runtime cannot know is the defect being fixed.
