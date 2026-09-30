---
format: aep.planning-md/3
id: review-result:adversary-ops-05-w-pass-1
kind: review-result
status: active
title: Adversary, wave ops-05 unit W, pass 1
relations:
- reviews: task:validate-builds-one-view-per-command
revision: 1
---
## Verdict

NEEDS-CHANGE on `0dcf4107` (warnings, no correctness defect), fixed in `20bc4bf7`. Held under
attack: a seeded differential over multi-revision histories (every session verdict equals a fresh
pipeline on the replayed basis, v1–v3, both providers, rejected, stale and older revisions), the
verdict's key (a freed graph's address cannot be reused while its weak reference lives), the index
released with its graph, contention from another thread.

```findings
- file: crates/ekr-kernel/src/replay.rs
  line: 743
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: undecided
  message: "validate --against a released revision revalidates the history to rebuild it (9 views at revision 7 of 8); the claim is narrowed and the cost filed as task:rebuilt-revision-reuses-retained-verdicts"
- file: crates/ekr-kernel/src/replay.rs
  line: 618
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "the first authorization took the verdict, so a retry after an append conflict validated again; it is now read, not taken"
- file: docs/epistemic-knowledge-runtime-design.md
  line: 3866
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "section 91.5 said the admitting replay reruns validation; amended in § 91.5.1"
```
