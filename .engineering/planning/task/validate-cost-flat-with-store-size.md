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
revision: 7
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
