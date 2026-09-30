---
format: aep.planning-md/3
id: review-result:adversary-ops-05-u-pass-1
kind: review-result
status: active
title: Adversary, wave ops-05 unit U, pass 1
relations:
- reviews: task:viewer-compact-follow-ups
revision: 1
---
## Verdict

NEEDS-CHANGE on `4acd71bf`, four findings, fixed in `f44cfcff`. Held under attack: the 970 px
threshold at 969, 970 and 971, device pixel ratio 2, `compact=0` beside every other address
parameter, edge chips from the keyboard, Space not scrolling, accessible names from the
accessibility tree, the strip mark from the palette and a path, no console errors.

```findings
- file: crates/ekr/src/cli/viewer/index.html
  line: 1841
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "every stream batch re-rendered the chips and dropped a focused chip's focus to body"
- file: crates/ekr/src/cli/viewer/index.html
  line: 2485
  category: acceptance
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "back to the node already shown marked the collapsed strip for a detail the reader had seen"
- file: crates/ekr/src/cli/viewer/index.html
  line: 689
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "back in a window narrowed since the entry was written collapsed both open sidebars"
- file: crates/ekr/src/cli/viewer/index.html
  line: 1927
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "a held key toggled the type on every autorepeat"
```
