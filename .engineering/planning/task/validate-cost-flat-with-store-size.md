---
format: aep.planning-md/3
id: task:validate-cost-flat-with-store-size
kind: task
status: active
title: Validate and commit cost stays flat in a large delta
relations:
- decomposes: epic:read-and-storage-cost
- serves: vision:o5
- derived_from: story:commit-cost-flat-with-store-size
revision: 11
transitions:
- {from: "draft", to: "proposed", at: "2026-10-01T11:10:07Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-01T11:10:08Z", actor: "human:timo", revision: 3}
---
## What is wrong

Wave extract-06 unit C made a held evidence object re-read only when its own stream moves. At full
size (20,000-fact base; 10,000- and 80,000-fact deltas; release; load 30–43;
`crates/ekr-sdk/tests/commit_scaling.rs`) SQLite still grows within the large delta: averaged over
transactions 1–15 against 106–120, validate 277 → 891 ms, commit 1,444 → 2,321 ms, propose
1,082 → 1,315 ms. Ratios: large/small 1.61× and last/first 3.28× over 3 transactions at each end;
1.40× and 1.43× over 15-transaction medians. The file provider: 1.18× and 0.76× (1.45× and 1.38×).
The story's acceptance bound is 1.5×.

Hypothesis, not verified: `crates/ekr-kernel/src/validate/lifecycle.rs:20–27` copies every claim in
the graph when a transaction holds a retract or supersede (found while scoping
`story:evidence-attaches-to-a-held-assertion`), and the consumer's transactions supersede.

## Build

A frame-pointer profile of `validate` and `commit` in the large SQLite delta (all threads of the
`ekr` child, inclusive), written here with the load average it ran at, then the change it points to.

## Acceptance

Revised by the coordinator on 2026-10-01 (`review-result:next-waves-1001-acceptance-r1`): this
acceptance first pointed at `story:commit-cost-flat-with-store-size`'s whole-transaction bound. That
story closed on it in 0.0.25 while validate alone still grew 3.2×, so the validate half of its
original title is bounded here on its own. The statistic matches the story's: medians of 15
transactions, load printed, no idle machine required.

- In the large SQLite delta of `crates/ekr-sdk/tests/commit_scaling.rs` at its default sizes, the
  median validate time of the last 15 transactions is at most 1.2× that of the first 15 (measured
  for 0.0.25: 277 → 891 ms averaged, 3.2×), and the same for commit; the load average is printed
  beside the numbers.
- Before the change, the profile result written into this task names one counted quantity per
  transaction (for example claims copied, or objects read); a counting test in the default gate
  asserts that quantity is equal in two stores whose assertion counts differ at least fourfold
- Refusals byte-identical (the kernel differentials and conformance suites pass).

## Recovered profile and resume boundary (2026-10-02)

The previous implementor's retained transcript reports frame-pointer samples for the
20,000-fact base and 80,000-fact large delta. Its small profiling lane was reduced to 2,700,
so this is diagnostic evidence, not the default-size acceptance measurement.
Per-transaction normalized samples from transactions 18–28 to 111–120 were reported as:
evidence loading 272.5 to 539.2 (validate 50.0 to 110.2; commit 100.3 to 150.4),
lifecycle checking 0 to 0. This contradicts the initial lifecycle-copy hypothesis for that run.
Other reported growth remained in checkpoint writes, root calculation and transaction-map cloning.
Source: the retained unit-W transcript and profile text, recovered by the read-only resume audit;
raw perf stacks had already been deleted by the prior session. The profile's load is retained in
its harness log and must be carried into the final measurement report, not guessed here.

WIP 39b37674 isolates repeated decoding of retained evidence on cached command replays.
Its counted quantity is retained objects placed into command histories, including memo hits; this is not a count of native provider reads. Its retained mutation result was
490 versus 196 reads with evidence skipping disabled, followed by an eight-case green
add_evidence lane with skipping restored. An earlier successive-transaction case still grew;
there is no claim that the full 1.2x acceptance passed. Complete the default-size measurement
and differential refusal tests before closing this task.

Reconcile with released main first: replay_history must use the current entered() guard,
and skipping evidence must not hide an appended withdrawal event or a replaced SQLite store.

## Default-size measurement and next correction (2026-10-02)

The resumed runtime at a8f087094 was measured with the default fact sizes on SQLite in release mode. The ignored benchmark exited 101. Verbatim per-verb medians and ratios from scaling.log:

```text
Sqlite first 15 medians            propose    645.5 ms  validate     76.0 ms  commit    648.5 ms  total   1370.0 ms
Sqlite last 15 medians             propose    923.1 ms  validate    182.7 ms  commit   1208.1 ms  total   2313.9 ms
Sqlite median ratios: validate 2.404x; commit 1.863x; load 14.50 13.53 14.69 8/6629 90184
```

These miss this task's bound. The counted retained-object test still passes, and two independent replay attacks pass; neither substitutes for measured acceptance. See review-result:adversary-extract-07b-w-pass-1. The task remains active.

The new frame-pointer capture is retained with its harness log. The implementor's profile attributes substantial validation work to copying retained transaction records through ReplayState::transactions_mut. A new counted test was written and failed before the correction:

```text
running 1 test
test replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers ... FAILED

failures:

---- replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers stdout ----

thread 'replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers' (143448) panicked at crates/ekr-kernel/src/replay.rs:1492:13:
assertion `left == right` failed: file=false: prior document buffers copied
  left: [4, 16]
 right: [0, 0]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    replay::tests::validating_one_transaction_does_not_copy_prior_document_buffers

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 13 filtered out; finished in 0.42s

error: test failed, to rerun pass `-p ekr-kernel --lib`
```

The next correction shares individual internal records while preserving the public full-transaction snapshot API and its unchanged-read sharing. Session settle must read only the states it needs, so constructing that public snapshot does not simply move the copying into another part of validate/commit. This expands the implementation surface to replay state, runtime state access and CLI session settle; O also edits session responses, in a separate region. No wire format or canonical root encoding changes. Re-measure before claiming completion.

## Second full-size measurement (2026-10-02)

The second full-size SQLite run covers f9d910a3b, using unchanged sizes 20000/10000/80000 and
unchanged first/last full-transaction windows. The run remains red. Exact measured lines:

```text
Sqlite first 15 medians            propose    676.6 ms  validate     57.9 ms  commit    673.6 ms  total   1408.2 ms
Sqlite last 15 medians             propose    753.5 ms  validate    115.1 ms  commit    994.9 ms  total   1863.5 ms
Sqlite median ratios: validate 1.986x; commit 1.477x; load 11.71 18.76 17.85 4/6321 874580
MEASUREMENT_EXIT=101
```

Raw scaling.log, scaling.perf.data and profile analysis remain under <cache>/ekr-extract-07b/w/records.
The implementor reports that record-copy samples vanished; validation still scales with candidate
and alias scans and old-graph destruction. Commit samples grow in knowledge/evidence root hashing,
checkpoint work and graph copying. These are profile-derived hypotheses for further counted probes,
not a green acceptance claim. Current SHA-256 canonical roots must remain byte-identical.
The coordinator continues correction within the user's full extract-07b scope; no bound is relaxed
and no release is claimed. Temporary Rust throughput probes may measure alternative hash execution
before any production dependency choice; no dependency change has been accepted on that basis yet.
