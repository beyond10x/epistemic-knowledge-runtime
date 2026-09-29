---
format: aep.planning-md/3
id: story:sdk-store-checks
kind: story
status: draft
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
  path: crates/ekr-sdk/tests/checks.rs
revision: 3
---
## Context

A consumer instance's health report, data-free code check and sampled fact-quality check
(~1,650 lines) each have an engine story. The SDK's part is a typed call per verb, plus a `Judge`
hook, because the runtime does no judging (invariant 7).

## Build

- `quality_report(rev)`, `check_code_names(files)`, `draw_sample(seed, size, filter)`,
  `report_judged(judgements)` returning a rate with its Wilson interval.
- `trait Judge` and a batching driver with no model code.
- Each wrapper lands when its verb lands.

## Surface (inferred)

`crates/ekr-sdk/src/checks.rs`, `crates/ekr-sdk/tests/checks.rs`.

## Acceptance

- Each SDK result equals the verb's document byte for byte.
- A fake judge with fixed verdicts gets the closed-form Wilson bounds.
- The same seed, size and revision draw the same sample through the SDK on both providers.
