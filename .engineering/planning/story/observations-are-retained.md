---
format: aep.planning-md/3
id: story:observations-are-retained
kind: story
status: draft
title: Observations are retained, so a second ingest records nothing new
relations:
- serves: vision:o5
- decomposes: epic:p2-observation-layer
scope:
- confidence: inferred
  path: crates/ekr-observe/src/lib.rs
- confidence: inferred
  path: crates/ekr-observe/tests/retained.rs
- confidence: inferred
  path: systems/ekr/domains/observe.yaml
revision: 4
---
## Context

`ekr-observe` maps a JSONL file to observations with deterministic ids (`story:fixture-records-become-observations`),
but nothing keeps them: a second ingest cannot tell what it has already seen, and evidence cannot
cite an observation. Roadmap P2 exit: the same delta ingested twice produces zero new observations.
Where observations live is `decision-blocker:observation-retention-path`; what a source unit is,
`decision-blocker:source-unit-granularity`.

## Build

Retain observations as the retention decision says (recommended: outside revisions, pinned when
cited), keyed by the idempotency key `ekr-observe` already derives; a checkpoint per source stream
as `decision-blocker:checkpoint-unit-cardinality` says. Declared in `systems/ekr/domains/observe.yaml`
first.

## Acceptance

- Recording the same records twice yields zero new observations, on both providers.
- An observation cited by committed evidence is kept; one nothing cites is reclaimable.
- `ekr.observe` scenarios for both, in the conformance suite.
