---
format: aep.planning-md/2
id: review-result:p1-15-ess-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-15 ESS unit
relations:
- reviews: story:pin-ess-0-32
revision: 1
---
Adversary pass 1 on unit p1-15-ess (wave p1-15), `aep-drive:adversary`, against `897a969`.

Owners: 2 findings, 2 implementor (introduced).

Verdict: CONFIRMED (1 warning, 1 note). cases: executed 159→161, red 0 on the tree; red against a Taskfile mutant with `preconditions:` misspelled.

Cases in `crates/ekr/tests/adversary_p1_15_ess_pin.rs` (2, green): `task` itself refuses ess 0.29.0, 0.32.1, 0.32.0-rc.1 and a trailing space, and admits 0.32.0.

Could not break: one rev everywhere; suite byte-identical on re-synthesis; the lockfile moved only the 5 ESS packages; `serde_yaml` already reached `ekr` at the base.

Routing (coordinator): both back to the implementor for a small correction; the adversary cases stay.

```findings
[
  {"file": "crates/ekr/tests/conformance.rs", "line": 341, "category": "mutant", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "taskfile_guard accepts a `- sh:` line under any key and runs it with /bin/sh instead of task, so a Taskfile whose `preconditions:` is misspelled `precondition:` leaves both checks unguarded (task 3.53.1 runs a stand-in ess 0.29.0, exit 0) while the case stays green"},
  {"file": "Taskfile.yml", "line": 65, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the guard checks only the version string, so an ess built from ess main 932d456, which also reports 0.32.0 and changes 22 crate files after the tag, passes, while the guard message claims the Cargo.toml and CI pin"}
]
```
