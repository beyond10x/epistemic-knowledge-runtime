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
revision: 3
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

A profile of `validate` and `commit` in the large SQLite delta on an idle machine, written here,
then the change it points to.

## Acceptance

- `commit_scaling.rs` at its default sizes passes its 1.5× bounds on SQLite and file, on an idle
  machine, and closes `story:commit-cost-flat-with-store-size`'s acceptance.
- Refusals byte-identical (the kernel differentials and conformance suites pass).
