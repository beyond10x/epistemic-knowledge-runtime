---
format: aep.planning-md/2
id: review-result:p1-12-recovery-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-12 recovery unit
relations:
- reviews: story:commit-and-revision-lineage
revision: 1
---
Adversary pass 1 on unit p1-12-recovery (wave p1-12), `aep-drive:adversary`, against the unit's
untracked tests on `e6af002` (no `src/` change).

Owners: 3 findings, 1 implementor (introduced), 2 pre-existing.

Verdict: CONFIRMED. cases: executed 5→14, red 1.

Cases in `crates/ekr-kernel/tests/adversary_p1_12_recovery_1.rs`, both providers: a pending
Bootstrap retried under a changed valid host anchor (red: `Other("AuthorityMismatch")` instead of
`PublicationInputConflict`; nothing written, clock not sampled, original anchor still resumes); a
different seed document (green); an unresolved commit overtaken by a competitor ends Stale at its
elected time (green); unrelated movement during uncertainty keeps the elected occurrence for every
kind (green); two unresolved revision-1 commits resolve to one commit in either order (green); two
real processes racing for revision 1 commit it exactly once, 4 rounds per provider (green); reads
over a pending preparation write nothing (green); a forged successor under a new occurrence id is
refused (green); a Validate preparation in the Commit slot is refused (green).

Finding F3 of the review of `165a577` (two handles both commit revision 1) did not reproduce under
any forced interleaving, including a two-process race.

```findings
- file: crates/ekr-kernel/src/commit.rs
  line: 247
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a pending Bootstrap slot retried under a different valid host anchor returns raw AuthorityMismatch instead of the PublicationInputConflict §94.1/§94.3 name, because the preparation is authorized under the new anchor before the input comparison
- file: crates/ekr-kernel/tests/recovery.rs
  line: 347
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the contention counter counts rounds where both handles called prepare, not rounds that overlapped past the store read, and the interleaved half's loser never writes, so the loser-binds-nothing check only reaches the refused-write path when a thread race happens to overlap
- file: .engineering/reviews/p1-durable-application-remaining.md
  line: 1
  category: acceptance
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: remaining-item-1 and §94.3 list retained receipts and exact retry after erasure, but no case covers them and no EKR erasure path exists to drive
```
