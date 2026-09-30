---
format: aep.planning-md/3
id: story:sdk-evidence-attachment
kind: story
status: implemented
title: Per-item evidence travels with the assertions that cite it
relations:
- depends_on: story:sdk-resolve-and-batch
- depends_on: story:add-evidence-operation
- serves: vision:o5
- decomposes: epic:consumer-sdk
scope:
- confidence: inferred
  path: crates/ekr-sdk/src/evidence.rs
- confidence: inferred
  path: crates/ekr-sdk/tests/evidence.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-09-29T23:07:49Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-09-29T23:07:49Z", actor: "human:timo", revision: 5}
- {from: "active", to: "implemented", at: "2026-09-30T00:47:44Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

A consumer instance keeps per-message evidence outside the store, because evidence enters only
through the seed. `story:add-evidence-operation` (wave ingest-02) adds evidence after the seed.

## Build

- `EvidenceItem` (source identity, observed-at time, bytes) becomes an `AddEvidence` operation with
  a locally computed hash and id.
- The operation is placed in the same group as the first assertion citing it; later groups cite
  the existing id.
- Bisection never separates an assertion from evidence it introduced.

## Surface (inferred)

`crates/ekr-sdk/src/evidence.rs`, `crates/ekr-sdk/tests/evidence.rs`.

## Acceptance

- 100 assertions citing 40 items produce exactly 40 evidence entries.
- `explain` on each assertion returns its item's bytes.
- A planted rejection never commits an assertion without its evidence.
- The SDK's hashes for the same items equal `ekr hash`.
