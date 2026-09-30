---
format: aep.planning-md/3
id: review-result:adversary-reads-04-v-pass-1
kind: review-result
status: active
title: Adversary, wave reads-04 unit V, pass 1
relations:
- reviews: task:viewer-compact-mode
revision: 1
---
## Verdict

CONFIRMED on `48aa1512` (with the shift+click type solo it carries): no attack broke the unit. The
adversary's eight cases in `b4c44f5f` close the unit's gaps (`compact=left`, the early fold, the
shift+click solo had no test). Notes filed as `task:viewer-compact-follow-ups`: a chosen node's
detail renders into a collapsed panel with no sign, chips and fold tabs have no keyboard path, and
windows under 650 px leave the graph no width.

```findings
- file: crates/ekr/tests/view_page.rs
  line: 2635
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "no unit case read or wrote compact=left or clicked the left fold tab"
- file: crates/ekr/src/cli/viewer/index.html
  line: 1884
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the shift+click type solo merged into the unit had no test case"
- file: crates/ekr/src/cli/viewer/index.html
  line: 2325
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "with the right sidebar collapsed a chosen node's detail renders into the hidden panel with no sign"
- file: crates/ekr/src/cli/viewer/index.html
  line: 1878
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: "type chips and the new fold tabs have no keyboard path or accessible names"
- file: crates/ekr/src/cli/viewer/index.html
  line: 21
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: "below 650 px with both sidebars shown the graph has no width"
```
