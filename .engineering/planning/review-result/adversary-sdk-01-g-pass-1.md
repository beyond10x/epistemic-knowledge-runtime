---
format: aep.planning-md/3
id: review-result:adversary-sdk-01-g-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-01 unit G, pass 1
relations:
- reviews: story:viewer-3d-draws-in-batches
- reviews: task:hops-slider-streams-the-neighbourhood
revision: 1
---
## Verdict

NEEDS-CHANGE, four defects and one weak test; all fixed in `f03c975d`. Batch contents after
filters, idle pause and resume, growth past 5,000 edges, 20 view switches, picking (51/51) and memory
over four streams held.

```findings
- file: crates/ekr/src/cli/viewer/index.html
  line: 1950
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a hops depth of 3 streams /expand depth=3, which the server refuses as LimitExceeded (max 2)"
- file: crates/ekr/src/cli/viewer/index.html
  line: 1942
  category: concurrency
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "noteHops records a depth before its read succeeds, so 3 then 2 never streams 2 hops"
- file: crates/ekr/src/cli/viewer/index.html
  line: 1775
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "Expand 2 hops does not record its depth, so the slider at 2 streams the same page again"
- file: crates/ekr/src/cli/viewer/index.html
  line: 1221
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "a 3D drag moves only the held node; neighbours no longer follow"
- file: crates/ekr/tests/view_page.rs
  line: 1902
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the 3D smoke case passes against a page that draws nothing in 3D"
```
