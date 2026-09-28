---
format: aep.planning-md/3
id: review-result:adversary-read-01-f-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit F: the views conformance baseline'
relations:
- reviews: task:views-conformance-has-no-independent-floor
revision: 1
---
unit: wave read-01 unit F, task:views-conformance-has-no-independent-floor, commit c8c64e35
verdict: CONFIRMED (one failing case: a gap outside the task's deletion-only acceptance; nothing breaks the acceptance itself)
cases: executed 52→53, red 1
origin: introduced 0 / pre-existing 0 / undecided 4

## Case added

`crates/ekr-views/tests/adversary_views_floor_weakened.rs`: the committed suite with
`ekr.views/authored/a-search-with-no-match-answers-an-empty-list` cut to its first two steps
(equal to real ess 0.36.0 output for the stripped YAML) still satisfied every baseline check:
"kept its name but lost 7 of its 9 steps (3 of 3 expect_event checks), and the weakened suite still
holds the committed baseline: names true, total 41 >= 41, passed 41 >= floor 41, unavailable 0 <=
ceiling 0, failed 0".

## What held

Deleting, renaming or adding an authored scenario; adding a file without regenerating; deleting a
generated scenario; an empty timeline (ess refuses it, ESS-AUTHOR-027); skipped or unsupported
counted as passed (each asserted zero); quarantine must be empty; floor and ceiling edges.

```findings
- file: crates/ekr-views/tests/adversary_views_floor_weakened.rs
  line: 88
  category: acceptance
  severity: warning
  verdict: CONFIRMED
  origin: undecided
  message: an authored scenario regenerated with its three expect_event checks stripped keeps all 41 names and 41 passes, so the name-only baseline cannot detect weakened coverage; a separate gap outside this task's deletion-only acceptance
- file: crates/ekr-views/tests/support/suite.rs
  line: 149
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: authored_scenario_name claims ess names scenarios by file stem, but ess 0.36.0 names them by the YAML scenario field, so the provenance test rejects a legitimate stem/name mismatch
- file: systems/ekr/conformance/views-baseline.json
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: deleting a scenario and removing it from the baseline in the same change passes every test, and the only guard is review of the baseline diff
- file: crates/ekr-views/tests/support/suite.rs
  line: 255
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: answered_floor and unavailable_ceiling have no effect because the exact assertions that follow dominate them, so any floor up to total and any ceiling passes
```
