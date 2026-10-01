---
format: aep.planning-md/3
id: task:store-open-verifies-blobs-once
kind: task
status: draft
title: Opening a store does not re-hash every retained blob
relations:
- decomposes: epic:read-and-storage-cost
- serves: vision:o5
revision: 2
---
## What is wrong

Wave extract-06 unit X (2026-10-01) measured `explain` at the 1× shape (a synthetic SQLite store
of 490 MB: 4,127 nodes, 14,421 edges, 66,995 assertions) after it answered by reference. One-shot
`ekr explain` costs 3.85–4.97 s CPU, of which opening the store (`runtime.read(None)`) is 4.6–4.7 s;
about 60% of the open is SHA-256 in the eventlog provider's blob integrity check
(`blob_integrity_sha256`). `ekr head` costs 20 ms. Through a session, the read before each call
costs 250–375 ms. Every one-shot read verb pays the open, so the cost is not explain's.

Source: `review-result:adversary-extract-06-x-pass-1`; the bench is
`crates/ekr/tests/explain_by_reference.rs` (ignored), logs in the unit's report.

Also measured in the same release bench: a session call spends 250–375 ms capturing the head and
90–175 ms re-hashing the roots (`bound()`) before explain's own work; both are per call and not
explain's, so they belong here.

## Build

A probe first: attribute the one-shot open and the per-call session read at the 1× shape. Then the
change it points to, for example verifying a retained blob's bytes once per content hash and
process, or reading only the blobs a verb needs, without weakening the integrity guarantee a fresh
handle gives (every byte it serves was verified).

## Acceptance

- One-shot `ekr explain` at the 1× shape costs under 0.5 s CPU, and an `explain` call through a
  session under 0.5 s, measured by the existing bench.
- Every integrity refusal a fresh handle gives today, it still gives.
