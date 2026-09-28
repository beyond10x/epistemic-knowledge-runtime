---
format: aep.planning-md/3
id: story:store-quality-report
kind: story
status: draft
title: A read reports a store's quality beyond its size
relations:
- serves: vision:o6
- decomposes: epic:p6-maintenance-observability
revision: 2
---
## Context

A store's quality is more than its size (asked by a consumer instance, 2026-09-28). Roadmap P6
plans epistemic health metrics (`ekr-metrics`, design § 61, § 75, A9); no story exists.

## Build

A read verb that reports, for a revision: share of assertions with evidence and with per-item
evidence, share of properties under a constraint, nodes of one type sharing an alias or a
canonical name, open ambiguities recorded by resolve, and validation refusals in the last N
transactions. Deterministic, one JSON document, specified in ESS before code.

## Acceptance

- On a fixture store with known counts, every figure equals the fixture's count, on both
  providers.
- Two reads of one revision are byte-identical.

## Consumer input (2026-09-29)

A consumer instance runs its own health report (node, edge and evidence counts, identity and schema
checks, 456 lines) and asks for it in the engine; this story is that report. Its data-free code
check is `story:store-reading-code-names-no-contents`, and its sampled fact-quality method is
`story:fact-quality-by-judged-sample`.
