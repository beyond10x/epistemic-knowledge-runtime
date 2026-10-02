---
format: aep.planning-md/3
id: task:fact-quality-states-the-empty-interval
kind: task
status: active
title: ekr fact-quality states the interval when nothing was judged
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-10-02T10:34:32Z", actor: "agent:codex-ekr-x7b", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-02T10:34:32Z", actor: "agent:codex-ekr-x7b", revision: 4}
---
## What is wrong

A consumer (2026-10-02): `ekr fact-quality` prints no interval when no fact was judged (n = 0),
where the consumer writes (0, 1). The consumer also sees its own Wilson bounds differ from the
runtime's in the last bits (202 of 231 cases, all within 1e-12); the runtime's z is within 4 ULP of
60-digit quantiles (`review-result:adversary-extract-06-q-pass-1`), so that difference is the
consumer's and not changed here.

## Decision (coordinator, 2026-10-02)

At n = 0 the report states the vacuous interval: `lower` 0 and `upper` 1, with `rate` null.

## Acceptance

- `ekr fact-quality` on a judgement document of no judged facts prints `lower` 0, `upper` 1 and
  `rate` null, specified in `views.yaml` first; every other output is byte-identical.

## Resume scope (2026-10-02)

Read-only story-scoper inspected main 4832d892. Primary paths, cited unless explicitly new:

- `crates/ekr-views/src/sample.rs`
- `systems/ekr/domains/views.yaml`

The consumer group shares views.yaml, CLI dispatch, typed SDK exports and documentation;
its artifacts are implemented serially in one managed consumer unit. Generated conformance
suites and planning writes belong to the coordinator. New SDK modules are inferred.
Typed task scope is unavailable: AEP 0.64.0 restricts the scope field to stories.
