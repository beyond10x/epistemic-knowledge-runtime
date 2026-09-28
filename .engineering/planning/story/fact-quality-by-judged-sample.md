---
format: aep.planning-md/3
id: story:fact-quality-by-judged-sample
kind: story
status: draft
title: Fact quality is reported from a judged, reproducible sample with a Wilson interval
relations:
- serves: vision:o6
- decomposes: epic:p6-maintenance-observability
revision: 1
---
## Context

A consumer instance judges the quality of extracted facts by drawing a seeded sample, having each
fact judged against its cited evidence, and reporting the pass rate with a Wilson interval; the
consumer sets the sample size and the bar (reported 2026-09-29). Epic P6 plans epistemic health
metrics; invariant 7 keeps deterministic validators model-free, so the judging itself belongs to
an agent, not to a validator.

## Build

A read that draws a reproducible sample of assertions from a revision (seed, size, optional type
filter) and prints each with its evidence bytes, and a verb that takes the judged sample back and
reports the pass rate with its Wilson interval at a stated confidence. The runtime does no
judging.

## Acceptance

- The same seed, size and revision draw the same sample on both providers.
- For a judged fixture of known results the reported rate and interval equal the closed-form
  Wilson values.
