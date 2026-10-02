---
format: aep.planning-md/3
id: story:sdk-store-checks
kind: story
status: active
title: Store health, the code-names check and fact quality are SDK calls
relations:
- depends_on: story:sdk-session-transport
- depends_on: story:store-quality-report
- depends_on: story:store-reading-code-names-no-contents
- depends_on: story:fact-quality-by-judged-sample
- serves: vision:o5
- decomposes: epic:consumer-sdk
scope:
- confidence: inferred
  path: crates/ekr-sdk/src/checks.rs
- confidence: inferred
  path: crates/ekr-sdk/src/lib.rs
- confidence: inferred
  path: crates/ekr-sdk/tests/checks.rs
- confidence: inferred
  path: docs/sdk.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T11:10:06Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-01T11:10:06Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
---
## Context

A consumer instance's health report, data-free code check and sampled fact-quality check
(~1,650 lines) each have an engine story. The SDK's part is a typed call per verb, plus a `Judge`
hook, because the runtime does no judging (invariant 7).

## Build

- `draw_sample(seed, size, filter)` and `report_judged(judgements)`, typed over `ekr sample` and
  `ekr fact-quality` (0.0.25), the latter returning a rate with its Wilson interval.
- `trait Judge` and a batching driver with no model code.
- Already shipped and out of this story: `quality_report` and `check_code_names`
  (`task:sdk-types-the-check-reads`, 0.0.22).

## Surface (inferred)

`crates/ekr-sdk/src/checks.rs`, `crates/ekr-sdk/tests/checks.rs`.

## Acceptance

- Each SDK result equals the verb's document byte for byte.
- A fake judge with fixed verdicts gets the closed-form Wilson bounds.
- The same seed, size and revision draw the same sample through the SDK on both providers.

## Resume scope (2026-10-02)

Read-only story-scoper inspected main 4832d892. Primary paths, cited unless explicitly new:

- `crates/ekr-sdk/src/checks.rs`
- `crates/ekr-sdk/tests/checks.rs`
- `crates/ekr-sdk/src/lib.rs`
- `docs/sdk.md`

The consumer group shares views.yaml, CLI dispatch, typed SDK exports and documentation;
its artifacts are implemented serially in one managed consumer unit. Generated conformance
suites and planning writes belong to the coordinator. New SDK modules are inferred.
