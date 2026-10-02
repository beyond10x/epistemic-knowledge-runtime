---
format: aep.planning-md/3
id: task:quality-counts-seed-evidence-and-constrained-types
kind: task
status: active
title: ekr quality counts seed evidence and constrained types
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T10:34:31Z", actor: "agent:codex-ekr-x7b", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T10:34:31Z", actor: "agent:codex-ekr-x7b", revision: 4}
---
## What is wrong

A consumer tried, on 2026-10-02, to replace its own store checks with 0.0.25's `ekr quality`, and
every number differed from its own:

- `with_item_evidence` counts 0 where the consumer counts 2: its assertions cite corpus files kept
  beside the store, which the runtime does not hold.
- There is no count of assertions citing seed evidence.
- Shares are basis points; the consumer writes a 0–1 float.
- It counts constrained properties; the consumer counts constrained types.

## Decisions (coordinator, 2026-10-02)

- Evidence the runtime does not retain is not counted: `ekr quality` measures the store. Files kept
  beside a store become evidence through `!AddEvidence` or, once it ships,
  `story:evidence-attaches-to-a-held-assertion`.
- Shares stay integer basis points, so every host prints the same bytes; the documentation gives
  the conversion.
- Added: `assertions.with_seed_evidence`, and `properties.constrained_types` beside the property
  count.

## Acceptance

- `ekr.store-quality/1` carries both new counts, specified in `views.yaml` first, on both providers
  and through the SDK's typed read.
- The views fixtures' existing counts are unchanged.

## Resume scope (2026-10-02)

Read-only story-scoper inspected main 4832d892. Primary paths, cited unless explicitly new:

- `crates/ekr-views/src/quality.rs`
- `crates/ekr-sdk/src/read/checks.rs`
- `systems/ekr/domains/views.yaml`

The consumer group shares views.yaml, CLI dispatch, typed SDK exports and documentation;
its artifacts are implemented serially in one managed consumer unit. Generated conformance
suites and planning writes belong to the coordinator. New SDK modules are inferred.
Typed task scope is unavailable: AEP 0.64.0 restricts the scope field to stories.

## Resume decisions (2026-10-02)

Count active assertions with retained seed evidence, including evidence attached after creation.
Count constrained declaring node and edge types, once per type with at least one directly declared
constrained property, matching the existing declaration-based property count. Preserve all old
counts and basis-point units. Documentation converts a share by division by 10000.
