---
format: aep.planning-md/3
id: epic:read-and-storage-cost
kind: epic
status: active
title: Reads and storage cost what the work needs, not what the history's length dictates
relations:
- serves: vision:o5
- decomposes: initiative:epistemic-knowledge-runtime
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T18:18:11Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"review_outcome":1}}}
- {from: "proposed", to: "active", at: "2026-10-01T18:18:11Z", actor: "human:timo", revision: 5, decided_on: {"recorded":{"review_outcome":1}}}
---
## Outcome

Reads and storage cost what the work needs at a consumer's size, not what the history's length
dictates: `explain`, `snapshot`, the views reads and the store's size grow with the change, not with
every committed document.

## Context

`epic:ingestion-throughput` met its outcome (resolve 145 → 3 ms, a 1,561-operation batch 3.5 →
0.94 s, CHANGELOG 0.0.14–0.0.18). The performance audit of 2026-09-29
(`task:perf-audit-2026-09-29-remaining`) measured what is left on the read and storage side at 1× /
3× / 10× of a consumer's shape. This epic holds that work, split into stories as it is scheduled.

## Acceptance

- `explain` at 1× answers in under 200 KB through the session and the one-shot verb and under 400 KB
  through MCP, with the § 62 chain unchanged, and explain's own work on an open store takes under
  0.5 s (`story:explain-reads-an-index`, met in 0.0.25). The 0.5 s per call, which opening the store
  dominates, is `task:store-open-verifies-blobs-once`.
- A 1× store after a full ingest is under half its 0.0.24 size, with every retained record intact
  (invariant 5).
- Each story keeps its audit measurement as its acceptance.
