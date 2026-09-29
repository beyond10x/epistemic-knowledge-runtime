---
format: aep.planning-md/3
id: review-result:adversary-perf-01-b-pass-1
kind: review-result
status: active
title: Adversary, wave perf-01 unit B, pass 1
relations:
- reviews: story:retained-bytes-shared-not-copied
revision: 1
---
## Verdict

NEEDS-CHANGE, one finding; the integrity path held: no request answered from bytes that were never
verified.

```
unit: B story:retained-bytes-shared-not-copied, ekr-p1-b at 7f299158 plus 2 adversary test files
verdict: NEEDS-CHANGE
cases: executed 546→552, red 1
```

## Cases added

`crates/ekr-kernel/tests/adversary_retained_bytes_shared_p1.rs`: the seed input's payloads are the
retained allocations (red: "46 seed evidence bytes are a copy of the retained objects, and the
runtime keeps 46 of them after the read is dropped", /2 and /3, both providers); dropping the runtime
frees the seed input; a seed payload damaged on disk is refused while a verified copy is still held;
a runtime reading on three threads sees another runtime's AddEvidence commit.
`crates/ekr-store/tests/adversary_retained_bytes_shared_p1.rs`: no retained byte outlives its handle
(sqlite, file). Kept in `d491038b`.

## Attacked and not broken

The pointer-equality path (trusted only while the registered copy is alive and unchangeable);
tamper on disk (both providers refuse before the kernel compares); the cached seed input across
stores (keyed by seed hash, one authority per `Commit`); refusals on later reads; memory release on
drop.

```findings
- file: crates/ekr-kernel/src/commit.rs
  line: 246
  category: contract-drift
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "The first read of each runtime copies every seed evidence payload (seed.rs:162 to_vec) into a seed input the runtime keeps for life; retained_bytes_shared.rs:5 and the acceptance say they are the store's shared bytes, not a copy."
```
