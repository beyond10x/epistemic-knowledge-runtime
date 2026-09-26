---
format: aep.planning-md/2
id: review-result:p1-15-ess-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-15 ESS unit
relations:
- reviews: story:pin-ess-0-32
revision: 1
---
Adversary pass 2 on unit p1-15-ess (wave p1-15), `aep-drive:adversary`, against `125c2c9`.

Owners: 3 findings, 3 implementor (introduced).

Verdict: INFEASIBLE (1 note), CONFIRMED (2 notes). cases: executed 161→162, red 1.

Pass-1 check: both pass-1 findings resolved (the guard case goes red on the misspelled-key mutant; the guard message claims only the version string).

Case: `crates/ekr/tests/adversary_p1_15_ess_pin_r2.rs` (red: `task --force spec-check` skips the precondition and runs ess 0.29.0, exit 0).

Routing (coordinator): A is correct and unreached (no caller passes --force); the coordinator rewrote the case to assert today's state with a message naming the story, and added `no_caller_runs_task_with_force`, which goes red when a caller forces. B and C are no-op: the pass-1 cases that run the real `task` hold the guard, and the layout check stays as a second, stricter line.

```findings
[
  {"file": "Taskfile.yml", "line": 64, "category": "boundary", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "the ESS guard is a go-task precondition, which `task --force` skips, so `task --force spec-check` runs ess 0.29.0 with exit 0; no caller passes --force"},
  {"file": "crates/ekr/tests/conformance.rs", "line": 341, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "taskfile_guard runs only the raw first line of the sh value under /bin/sh, so a YAML continuation line `|| true` leaves it green while task admits ess 0.29.0; only the pass-1 task-driven case catches it"},
  {"file": "crates/ekr/tests/conformance.rs", "line": 341, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "taskfile_guard goes red on six valid Taskfile forms that task honours, so it checks layout rather than what task does"}
]
```
