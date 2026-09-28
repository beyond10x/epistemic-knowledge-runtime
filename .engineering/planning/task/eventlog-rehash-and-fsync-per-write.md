---
format: aep.planning-md/3
id: task:eventlog-rehash-and-fsync-per-write
kind: task
status: draft
title: The eventlog file provider re-hashes the log after a write and syncs 28 times per verb
relations:
- serves: vision:o5
- derived_from: task:write-verbs-cost-most-of-an-ingest
revision: 1
---
## What is wrong

Measured in wave read-01 unit W (commit `659a913f`, release profile of write verbs in one
`ekr session` on a 19,971,175-byte file store): 42.3% of the time was the eventlog file provider's
`journal::resumed_committed` re-hashing the whole log on every call made within 2 s of a write
(`UNTRUSTED_NS`), and each atomic group costs 8–10 `fsync` calls, 28 per verb, 12–32 ms per 4 KB
synchronous write on this machine's disk. Both are in the `eventlog` repository, not in this one.

## What closes this

An eventlog release in which a handle that wrote the log itself does not re-hash it within that
window, and an atomic group needs fewer `fsync` calls, adopted here with the write-verb measurement
of `task:write-verbs-cost-most-of-an-ingest` repeated.
