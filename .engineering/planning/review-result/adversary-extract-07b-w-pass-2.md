---
format: aep.planning-md/3
id: review-result:adversary-extract-07b-w-pass-2
kind: review-result
status: active
title: Selected state and lazy snapshot adversary pass two
relations:
- reviews: task:validate-cost-flat-with-store-size
revision: 1
---
unit: W transaction_states and lazy snapshot correction at f9d910a3bb8002a140904d55323b8cc83b991fd7 plus test-only additions
verdict: NEEDS-CHANGE
cases: executed 17→19, behavioral red 0; retained acceptance measurement red 1
origin: introduced 0 / pre-existing 0 / undecided 1
wrote-outside-worktree: assigned W review2 scratch and exclusively handed-over W build target
needs-coordinator: carry the unmet W timing acceptance forward; zero new behavioral findings does not resolve it

1. Test-only diff

```text
 crates/ekr-kernel/tests/add_evidence.rs  | 102 ++++++++++++++++++++++++++++
 crates/ekr-kernel/tests/verified_read.rs | 112 +++++++++++++++++++++++++++++++
 2 files changed, 214 insertions(+)
```

Read the complete f9d910a3 delta and the existing lifecycle, retained-evidence, shared-read and clone-count cases before choosing these attacks. Both new cases were written before their first execution. Only new Rust tests were appended; no source or existing test was changed. The W implementor finished every cargo/performance process and explicitly handed over its existing target before these executions.

2. First cases

Commands used `cargo test --locked -p ekr-kernel --test <target> <case> -- --exact --nocapture`, with the exclusive W target and bounded build settings. Full raw logs retain compiler progress and private paths; the exact runtime summaries are below.

`first-lifecycle.log`, exit 0:

```text
running 1 test
test adversary_selected_states_follow_peer_decisions_without_mutating_held_snapshots ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 7 filtered out; finished in 0.24s
```

The case runs on File and SQLite. It requires NotSeeded for an empty selection before seed, then follows actual peer proposals, validation, rejection, commitment and staleness. Selected states must equal the independently expected map with reversed, duplicate and unknown IDs. Previously captured empty, proposed and validated full snapshots retain identical serialized bytes. A consumer's private Arc mutation cannot change subsequent reads. Reopen from checkpoint and explicit full replay agree with the current complete record map.

`first-empty-unknown.log`, exit 0:

```text
running 1 test
test adversary_empty_and_unknown_state_requests_still_refuse_withdrawn_history ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 10 filtered out; finished in 0.14s
```

This case commits an actual assertion and retained evidence on each provider and first proves normal known/unknown/empty behavior. It then appends an unsupported retention envelope through the provider, using the established repository test mechanism. Hot, checkpoint-warmed and cold readers must repeatedly return the exact same refusal as full replay for empty, unknown and duplicate-unknown selections. The held complete record snapshot remains unchanged after those failed reads.

3. Subsequent suite and checks

`suite.log`: `cargo test --locked -p ekr-kernel --test verified_read --test add_evidence`, exit 0. `add_evidence` has 11 passed; `verified_read` has 8 passed; neither has failures, ignored or filtered cases. Baseline 10 plus 7 is measured in the implementor's retained `store-kernel-final.log`, before these additions; after count is 19.

`fmt.log`: `cargo fmt --all -- --check`, exit 0.

`clippy.log`: `cargo clippy --locked -p ekr-kernel --test verified_read --test add_evidence -- -D warnings`, exit 0.

`git diff --check` exits 0. No full workspace gate was run in this bounded review.

4. Findings and limits

No new behavioral defect was found in these two attacks. They exercise the newly introduced selected-state boundary and the lazy full-snapshot invalidation, including replay errors after a held snapshot. They do not establish asymptotic or wall-clock acceptance.

W remains NEEDS-CHANGE. The retained current f9 measurement in <cache>/ekr-extract-07b/w/records/scaling.log reports default-size SQLite final/initial 15-transaction medians of validation 57.9→115.1 ms (1.986×), commit 673.6→994.9 ms (1.477×). Both exceed the task's independent 1.2× bounds. The log records MEASUREMENT_EXIT=101. These are measured performance results from the implementor's completed run, not an additional execution by this reviewer. No base-commit benchmark was run, so regression origin remains undecided; the unmet acceptance itself is established.

| File:line | Verdict | Origin | Measured | What reaches it |
|---|---|---|---|---|
| crates/ekr-sdk/tests/commit_scaling.rs:484 | NEEDS-CHANGE | undecided | Current records/scaling.log: validation 1.986× and commit 1.477×, each above 1.2×; measurement exits 101 | Default-size SQLite scenario, 20,000 base facts and an 80,000-fact large delta; first and last 15 transaction medians |

This carries the existing acceptance blocker forward. It identifies an unmet task bound, not a proven source-level cause or a newly discovered behavioral defect. The W implementor owns correction and a new full-size measurement. The coordinator owns preserving that unresolved status when recording this report.

Owners: 1 finding, 0 coordinator, 1 implementor. Zero NEW behavioral findings; one still-unmet performance acceptance finding.

5. Outside-tree files and handoff

The test changes are confined to <worktree-W>/crates/ekr-kernel/tests/{verified_read,add_evidence}.rs. `attack.patch`, `diff-stat.txt`, the two first-case logs, `suite.log`, `fmt.log`, `clippy.log`, matching `.exit` files, commit-message.txt and this report are retained under <cache>/ekr-extract-07b/w/review2. Builds used <cache>/b10x-target/ekr-x7b-w exclusively. No source edits, pushes, cleanup or planning writes were performed.

Bot test-only commit: abafdf4ce0e555b13df2180c3ea69d95831fe783. Author and committer both verified b10x-bot[bot]; tree clean at handoff. Own review lease ended and exclusive target/build slot explicitly returned to W's implementor before further W work.

```findings
- file: crates/ekr-sdk/tests/commit_scaling.rs
  line: 484
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: undecided
  message: Current f9 SQLite measurement still misses W's independent 1.2x bounds; validate medians are 57.9 to 115.1 ms (1.986x) and commit medians are 673.6 to 994.9 ms (1.477x). This carries the existing acceptance blocker forward; no base-commit benchmark established regression origin.
```
