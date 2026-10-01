---
format: aep.planning-md/3
id: task:rendering-cost-test-passes-under-load
kind: task
status: active
title: The rendering-cost test passes under load
relations:
- decomposes: epic:p6-maintenance-observability
- serves: vision:o5
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T11:10:07Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-01T11:10:07Z", actor: "human:timo", revision: 3}
---
## What is wrong

`crates/ekr-views/tests/adversary_add_evidence_views.rs`, case
`rendering_costs_linear_time…`, failed once in a fail-fast `cargo test -p ekr-views` run during wave
extract-06 (unit J's adversary pass, 2026-10-01, load average 37–44) and passed on the rerun. It is
recorded earlier in `review-result:adversary-ingest-02-a-pass-1`. A timing assertion that fails
under load fails the gate for a reason that is not a regression.

## Build

The case asserts the growth it was written for without depending on wall-clock time under load:
count work (calls, bytes, allocations) or compare ratios measured in one process with a margin the
load cannot cross, as `task:two-tests-pass-under-load` did for two other cases.

## Acceptance

- The case passes 20 runs in a row with the machine at load above 30, and still fails when the
  rendering is made quadratic (shown once, by a temporary mutation).
