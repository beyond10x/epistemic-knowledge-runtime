---
format: aep.planning-md/3
id: review-result:adversary-extract-06-c-pass-1
kind: review-result
status: active
title: Adversary, extract-06 unit C, pass 1
relations:
- reviews: story:commit-cost-flat-with-store-size
revision: 1
---
NEEDS-CHANGE

Adversary pass 1 on unit C (`impl/commit-cost-flat` at `88e6ad95`; cases committed as `cc4ccc37`, one red and ignored). `cargo test -p ekr-store`: 149 passed, 1 failed (the red case), 1 ignored; `cargo test -p ekr --test conformance`: 7 passed.

On SQLite, a handle that already holds a Provenance payload keeps serving it after the payload's stream event is redacted in place, because no feed position moves; at base the held stream was re-read and the redacted event refused (`stream-envelope-disagrees`). Nothing in EKR calls `redact` today; roadmap D1 and P6 route deletion requests through it. The file provider refuses as diverged, as at base.

Held: a class raise by another handle on both providers; blob deletion (as at base); a replaced store; every object-stream read counted; conformance refusals; memo growth.

```findings
- file: crates/ekr-store/src/eventlog.rs
  line: 938
  category: acceptance
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: "a held SQLite handle serves a Provenance payload whose stream event was redacted in place, where base refused it as stream-envelope-disagrees; no shipped caller redacts yet"
- file: CHANGELOG.md
  line: 18
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the entry claims refusals are unchanged, which a held handle after an in-place redaction contradicts
- file: crates/ekr-store/src/eventlog.rs
  line: 1114
  category: mutant
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: the mark-everything-moved fallback has no case and no shipped provider state was found that triggers it
- file: crates/ekr-sdk/tests/commit_scaling.rs
  line: 55
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the scaling harness measures 2,664 and 15,984 fact deltas where the acceptance names 10,000 and 80,000
```
