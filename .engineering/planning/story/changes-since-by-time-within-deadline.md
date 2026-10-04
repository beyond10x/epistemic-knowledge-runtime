---
format: aep.planning-md/3
id: story:changes-since-by-time-within-deadline
kind: story
status: draft
title: A changes query by recorded time costs what the same query by revision costs
relations:
- serves: vision:o5
- decomposes: epic:read-and-storage-cost
revision: 1
---
## Context

On 2026-10-04, ekr 0.0.30 `mcp-http` on the PostgreSQL provider answered `changes_since` with
`since_recorded` one week back and `limit=2000` with `request deadline exceeded`. That refusal is
`crates/ekr/src/cli/http.rs:448`, and the deadline is `READ_TIMEOUT` plus `WAIT_TIMEOUT`, 5 s + 30 s
(`http.rs:15-16`, `:438`). A `since_revision` request with `limit=500` on the same store answered. No cause is
established: the time query, the page size, the provider and the evidence entries in the window are all
candidates.

## Build

- Diagnose first: a red-capable loop that times `since_recorded` against `since_revision` over the same window
  on a synthetic store of comparable revision count and evidence volume, on both providers. Then probes that
  vary one input each.
- Fix the measured cause.

## Acceptance

- The diagnosis is recorded as evidence with its commands and numbers.
- A `since_recorded` request selecting the same revisions as a `since_revision` request costs no more reads
  than it, counted rather than timed, on both providers.
