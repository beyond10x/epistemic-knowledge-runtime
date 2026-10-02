---
format: aep.planning-md/3
id: task:fact-quality-states-the-empty-interval
kind: task
status: draft
title: ekr fact-quality states the interval when nothing was judged
relations:
- decomposes: epic:consumer-sdk
- serves: vision:o5
revision: 1
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
