---
format: aep.planning-md/3
id: story:source-adapter-contract
kind: story
status: draft
title: The engine specifies the source-adapter contract consumers implement
relations:
- serves: vision:o5
- decomposes: epic:p2-observation-layer
- informed_by: architecture-decision-record:0012-source-adapters-are-a-contract
revision: 1
---
## Context

`architecture-decision-record:0012-source-adapters-are-a-contract` keeps third-party connectors
out of the engine: consumers implement adapters against a contract the engine specifies. The
engine already maps JSONL records to deterministic observations (`story:fixture-records-become-observations`)
and models `ekr.observe` (`story:observe-domain-model`), but nothing defines what an external
adapter implements.

## Build

- `ekr.observe` in ESS, then a Rust trait: an adapter declares its source units, polls a unit from
  a checkpoint, and yields observation candidates plus the window it proves; the engine records
  observations idempotently, stores the checkpoint and reports poll health and coverage against
  the declared units.
- A fixture adapter in the engine's tests, and a documented way for a consumer to run its own
  adapter against a store (a library crate API, a CLI verb reading an adapter's output, or both —
  settled in the specification).
- Depends on the answers to `decision-blocker:observation-retention-path`,
  `source-unit-granularity` and `checkpoint-unit-cardinality`.

## Acceptance

- The fixture adapter polled twice over the same records records each once and reports zero new
  on the second poll, on both providers.
- A consumer adapter outside the engine's crates can be written against the published contract
  alone (a test crate in `crates/ekr/tests` or an example proves it).
- Coverage lists every declared unit with its checkpoint and last poll result.
