---
format: aep.planning-md/2
id: story:observe-domain-model
kind: story
status: implemented
title: Model the ekr.observe domain before any observation-layer code
relations:
- serves: vision:o5
- decomposes: epic:p2-observation-layer
scope:
- confidence: cited
  path: systems/ekr/components.yaml
- confidence: inferred
  path: systems/ekr/conformance/baseline.json
- confidence: inferred
  path: systems/ekr/conformance/suite.json
- confidence: cited
  path: systems/ekr/domains/observe.yaml
- confidence: inferred
  path: systems/ekr/system.yaml
revision: 11
---
## Context

Epic `epic:p2-observation-layer`, "Before decomposition" item 1: model the observation layer first, as an `ekr.observe` ESS domain and an `ekr-observe` component under `systems/ekr/`. That follows the one-ESS-domain-per-crate convention of roadmap § 3. This story is only the model: no Rust.

Scope for the first P2 slice:

- `SourceUnit`, `SourceCheckpoint` and `PollHealth` (`checked_through`, `attempt | complete | partial | failed`, A8);
- the observation idempotency key (design § 56: source identity, source-native id, content hash);
- the form an observation of one source record (one message) takes.

Redaction (A6), the coverage report and adapter declarations are **out of scope** for this story.

Design sections served: § 15, § 54, § 55, § 56, § 57; A8 (`docs/predecessors.md` § 2 and § 8).

Package: `systems/ekr` (`components.yaml`, a new `domains/observe.yaml`). No crate changes.

## Domain relations

Inferable, and to be declared as found:

- `Assertion → Support`: one-to-many, the assertion owns its supports. From `systems/ekr/domains/graph.yaml`, entity `ekr.graph.Assertion`, relation `support` (`kind: owns`, `cardinality: many`, `via: assertion_id`).
- `Support → Evidence`: many-to-one, references. From `systems/ekr/domains/graph.yaml`, entity `ekr.graph.Support`, relation `evidence` (`kind: references`, `cardinality: one`, `via: evidence_id`).
- `Evidence → Observation`: many evidence to zero or one observation, references, no ownership. This is inferable, but **inferred**: `systems/ekr/domains/graph.yaml:578` declares it as the optional field `ekr.graph.Evidence.observation_id`, not as a `relations:` entry, and code confirms it at `crates/ekr-graph/src/evidence.rs:74` (`EvidenceSource::Observation(ObservationId)`). No ess/1 document declares the relation.
- The `ekr.graph` domain owns `Observation`. From `systems/ekr/components.yaml:41` (component `ekr-graph` owns domain `ekr.graph`) and `systems/ekr/domains/graph.yaml:590` (entity `ekr.graph.Observation`). The new domain therefore references `ekr.graph.Observation` and does not redeclare it. Epic "Before decomposition" item 2 proposes moving it to `ekr-observe`. That would be a separate change to these two documents, and this story does not make it.

Held open, each as an `UNMAPPED:` marker naming its blocker. Do not guess:

- where an observation is retained, and whether it depends on a revision: `decision-blocker:observation-retention-path`;
- `SourceUnit → Observation`, and what a unit is: `decision-blocker:source-unit-granularity`;
- `SourceCheckpoint → SourceUnit` cardinality and lifecycle: `decision-blocker:checkpoint-unit-cardinality`;
- `SourceAdapter → SourceUnit` ownership: `decision-blocker:adapter-unit-ownership`.

## Acceptance

`ess specify validate systems/ekr` exits 0 with an `ekr.observe` domain, owned by a new `ekr-observe` component, that declares `SourceUnit`, `SourceCheckpoint`, `PollHealth` and the observation idempotency key, references `ekr.graph.Observation` without redeclaring it, and carries one `UNMAPPED:` marker naming each of the four decision-blocker ids above.

## Scope

Derived 2026-09-26 by `story-scoper`. Every line is **cited** (read from the story or the tree) or
**inferred** (a reading that could be wrong).

- **Primary surface:** `systems/ekr` (the ESS contract only, no crate) — cited, story "Package: `systems/ekr` … No crate changes"
- **Files (new):** `systems/ekr/domains/observe.yaml` — cited: `SourceUnit`, `SourceCheckpoint`, `PollHealth`, idempotency key, four `UNMAPPED:` markers
- **Files (modify):** `systems/ekr/components.yaml`, new `ekr-observe` component owning `ekr.observe` — cited
- **Files (modify):** `systems/ekr/system.yaml`, `ekr.observe` in `domains:` — inferred
- **Files (modify):** `systems/ekr/conformance/suite.json` — inferred: `provenance.spec_digest` digests the whole compiled system (ess `crates/verify/ess-diff/src/delta.rs:130`); `task conform-check` (`Taskfile.yml:77-78`) regenerates and `cmp`s it; earlier domain edits `194761a`, `cd2a2be` changed it
- **Files (modify):** `systems/ekr/conformance/baseline.json` — inferred: `suite_digest` follows the suite, asserted at `crates/ekr/tests/conformance.rs:203`
- **Symbols:** `ekr.observe`, `ekr-observe`, `ekr.graph.Observation` (referenced only, `systems/ekr/domains/graph.yaml:590`) — cited
- **Not in scope:** `graph.yaml`, `crates/ekr-core/src/identity.rs`, `crates/ekr-core/tests/identity_serde.rs` (the Observation-owner move is a separate change) — inferred
- **Confidence:** high for `observe.yaml` and `components.yaml`; medium for `system.yaml` and the two conformance files
- **Would collide with:** any unit changing any file under `systems/ekr/domains/`, or `components.yaml`, `system.yaml`, `conformance/suite.json`, `conformance/baseline.json` — every ESS-touching unit rewrites the suite digests
