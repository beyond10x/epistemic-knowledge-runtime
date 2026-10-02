---
format: aep.planning-md/3
id: story:commit-cost-flat-with-store-size
kind: story
status: implemented
title: A transaction no longer re-reads every evidence stream as the store grows
relations:
- decomposes: epic:read-and-storage-cost
- serves: vision:o5
scope:
- confidence: cited
  path: crates/ekr-store/src/eventlog.rs
- confidence: inferred
  path: crates/ekr-store/src/verified.rs
- confidence: inferred
  path: crates/ekr-store/tests/eventlog_object_memo.rs
- confidence: inferred
  path: crates/ekr/tests/commit_scaling.rs
revision: 12
transitions:
- {from: "draft", to: "proposed", at: "2026-09-30T15:36:16Z", actor: "human:timo", revision: 7}
- {from: "proposed", to: "active", at: "2026-09-30T18:15:39Z", actor: "human:timo", revision: 8}
- {from: "active", to: "implemented", at: "2026-10-01T18:18:10Z", actor: "human:timo", revision: 10, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
## Context

A consumer reported on 2026-09-30 that with ekr 0.0.24 the cost of one transaction grows with the
size of the store, so a large delta applies more slowly the further it gets. Measured on a synthetic
store (no customer data), SQLite provider, driven through the ekr-sdk 0.0.24 `Batcher`, at load
average 13–34 (absolute seconds are noisy; the growth is the signal):

| transaction | 10,000-fact delta | 80,000-fact delta |
|---|---|---|
| 2,000-operation data transaction (propose, validate, commit) | 5–9 s each | 13–21 s each, flat within the delta |
| AddEvidence transaction, in sequence | not reported | 5.6 s first, 65.7 s last |

Both deltas sit over the same base store. After the consumer removed a quadratic lookup of its own,
its process held 5.7% of CPU samples and the `ekr` child 94%. A 76,830-fact import took 16,540 s.

Hypothesis, not verified: a node or evidence lookup in propose, validate or commit that scans the
store instead of an index; for the evidence sequence, work that replays or revalidates the growing
history (compare `task:rebuilt-revision-reuses-retained-verdicts`).

## Build

1. A probe: a reproducible synthetic store at two delta sizes, driven through the SDK, and a profile
   of the `ekr` child that attributes the per-transaction time to functions. The attribution is
   written into this story before any fix is chosen.
2. The fix the attribution points to, so that propose, validate and commit cost grows at most with
   log n of the store's node and evidence counts.

## Acceptance

Revised by the coordinator on 2026-10-01 (store audit; `review-result:adversary-extract-06-c-pass-1`;
`review-result:next-waves-1001-acceptance-r1`). Two changes. First, the bound is on whole
transactions, not on validate alone: validate still grows 3.2× inside the large delta, and that half
of the original title moved to `task:validate-cost-flat-with-store-size` with its own 1.2× bound;
this story was retitled to what it built and moved to implemented on the whole-transaction bound.
Second, the statistic:
"an idle machine" is not available here (every recorded run was at load 13–43), and a ratio of
three transactions at each end swings with load. The statistic is fixed and the load is recorded.

- Counted, in the default gate: a second history load in one handle reads no object stream of a
  held Provenance object whose stream has not moved, on both providers
  (`crates/ekr-store/tests/eventlog_object_memo.rs`).
- Timed, by `crates/ekr-sdk/tests/commit_scaling.rs` at its default sizes (20,000-fact base;
  10,000- and 80,000-fact deltas; release build): over the median of 15 transactions at each end,
  the large delta costs at most 1.5× the small, and the last 15 at most 1.5× the first 15, on SQLite
  and file; the load average is printed beside the numbers.
- Refusals and roots byte-identical before and after (the kernel differentials and conformance
  suites pass).

Measured for 0.0.25 (`274924cb`, load 30–43): SQLite 1.40× and 1.43×, file 1.45× and 1.38× over the
15-transaction medians. The validate growth inside the large delta (277 → 891 ms) is
`task:validate-cost-flat-with-store-size`, which has its own bound.

## Probe result (2026-09-30)

The consumer re-ran the 80,000-fact delta against a frame-pointer build of ekr 0.0.24 (main
`ec6bbf38`, `force-frame-pointers`, line tables): 821,361 samples at 499 Hz, all threads. The run
took 2,798.8 s (29 facts/s) against 93.9 s for the base store, and 124 transactions committed 14,800
nodes, 27,999 edges and 80,000 assertions. Per the call trace, validate took 1,016.9 s over 124
calls, commit 929.2 s, propose 613.4 s, and resolve 113.7 s over 15,100 calls.

Measured, inclusive shares of the `ekr session` process:

| share | where |
|---|---|
| 71.1% | `eventlog_sqlite` `read_stream`, on the tokio blocking pool |
| 49.8% | `sqlite3LockAndPrepare`: the statement is compiled again on every call (`sqlite3RunParser` 48.0%, `sqlite3Reprepare` 25.7%) |
| 33.0% | `sqlite3_step`: running it |
| 21.1% | the session's main thread (`serve`, `respond`) |

Read from the code, not yet counted:

- `eventlog-sqlite` `read_stream` (`crates/eventlog-sqlite/src/lib.rs:1869`, eventlog at
  `fe8a0a7e`) calls `prepare`, not `prepare_cached`, so each call parses the same SELECT again.
- `load_history_requiring` (`crates/ekr-store/src/eventlog.rs`) starts each history with no objects
  and asks `required` for every object the history needs. `required` (`:906`) serves a verified
  object from the handle's memo only when its class is `Canonical`. For any other class it reads the
  object's stream again, since another handle may have raised it. Evidence payloads are `Provenance`
  objects (`crates/ekr-kernel/src/commit.rs:562`). So every history load reads one stream per
  evidence object, and the number of `read_stream` calls per verb grows with the evidence already
  in the store. That matches the data-transaction cost following the evidence count.

## Fix

1. In `ekr-store`: a verified non-canonical object is re-read only when the log has advanced past
   the position at which this handle verified it. One provider read of the log's position replaces
   one stream read per object, and a class raised by another handle is still seen, because raising
   it appends an event.
2. Upstream in `beyond10x/eventlog`: `eventlog-sqlite` uses `prepare_cached` for `read_stream` and the
   other per-call statements. This is a separate change in that repository, and the pin moves once it
   ships. It is not needed for this story's acceptance.

## Not established

- The number of `read_stream` calls per verb. The fix's first test counts them through
  `crate::verified::count_stream_read`, which today counts only revision and checkpoint streams, and
  the count is written here.
- Whether the file provider shows the same growth. Its `read_stream` has a different cost.
- Why SQLite re-prepares (`sqlite3Reprepare` 25.7%) inside a single step.

## Scope

- `crates/ekr-store/src/eventlog.rs`: `required`, `verified_objects`, `remember_verified` and the
  held-object memo (cited).
- `crates/ekr-store/src/verified.rs`: the stream-read counter gains an object-stream count (inferred).
- `crates/ekr-store/tests/`: a counting test in which a second history load in one handle reads no
  object stream while the log has not advanced, and reads it again after another handle raises the
  object's class (inferred).
- A Rust scaling harness, one `#[ignore]` test in `crates/ekr/tests/` driving the SDK over a base
  store and two synthetic deltas (inferred).

Would collide with: `story:preparation-blobs-are-reclaimed` and `task:divergence-is-a-typed-store-error`
(both in extract-07, both touching `eventlog.rs`). No extract-06 item touches `crates/ekr-store`.
