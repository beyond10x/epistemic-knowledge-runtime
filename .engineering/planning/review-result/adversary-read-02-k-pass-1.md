---
format: aep.planning-md/3
id: review-result:adversary-read-02-k-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit K: checkpoint cadence'
relations:
- reviews: task:checkpoint-cadence-costs-each-commit
revision: 1
---
unit: K, task:checkpoint-cadence-costs-each-commit, commit 2f72f2a3 (worktree ekr-r2-k, impl/checkpoint-cadence) plus the adversary's untracked test file
verdict: NEEDS-CHANGE
cases: executed 369→375, red 2
origin: introduced 2 / pre-existing 0 / undecided 0

Neither red case is a correctness defect: every reopen reached the same roots, graphs and records
as a full replay. Both are cost and contract problems with the new cadence.

## Cases added (kept as crates/ekr-kernel/tests/adversary_checkpoint_cadence_p1.rs)

| case | asserts | at 2f72f2a3 |
|---|---|---|
| a full-replay handle writes a checkpoint at each commit | § 99.1's sentence, one handle making 3 commits | red: `left: 1 right: 2` |
| an early-basis validation is covered by the next commit | after a validation against revision 1 with the checkpoint at 4, the next commit restores a from-checkpoint open | red: `after the commit of revision 6, the open still replays from the seed` |
| crash states reopen as a full replay | pointer older than head; pointer naming an older genuine checkpoint; checkpoint blob deleted; pointer naming another history's checkpoint | green |
| mixed histories v1, v2, v3 | 36-step fixed-seed runs on both providers with four kinds of handle; after each step a fresh open and both session handles equal a full replay; after each commit the checkpoint is fewer than 4 revisions and 512 operations behind | green |

## What held

Reopen after 0–22 commits on both providers under profiles v1–v3 (commits of 260 operations,
schema changes, rejections, stale commits, pending validations) always equalled a full replay;
session handles never answered a stale root; a pointer from another history or with older
coverage was never trusted; the 4-revision and 512-operation edges held; rejected and stale
decisions do not count; under v3 checkpoints written from a restored-then-replayed state were
admitted by checkpoint-held-identities; a checkpoint from another history was ignored.

```findings
- file: crates/ekr-kernel/src/checkpoint.rs
  line: 347
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: a validation against a revision before the retained checkpoint's head makes every open replay from the seed until the next due commit, up to three commits later, where the base's next commit covered it
- file: docs/epistemic-knowledge-runtime-design.md
  line: 4578
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: section 99.1 says a --full-replay handle writes a checkpoint at each commit, but a long-lived full-replay handle writes one only at its first
- file: crates/ekr-kernel/src/commands.rs
  line: 494
  category: concurrency
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: write_checkpoint returns Ok when the pointer append loses twice, so checkpoint_retained records a checkpoint never written and that handle can exceed the cadence bound
```
