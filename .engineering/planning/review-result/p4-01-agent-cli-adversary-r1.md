---
format: aep.planning-md/2
id: review-result:p4-01-agent-cli-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p4-01 agent-cli unit
relations:
- reviews: story:agent-discoverable-cli
revision: 1
---
Adversary pass 1 on unit agent-cli (wave p4-01), `aep-drive:adversary`, against `e74a7fb`.

Owners: 6 findings, 6 implementor (introduced).

Verdict: NEEDS-CHANGE (1 blocker, 1 warning), CONFIRMED (1 warning, 3 notes). cases: executed 140→144, red 4.

Cases in `crates/ekr/tests/adversary_p4_01_agent_cli.rs` (4, red): unsupported kinds documented as writable; example ids absent from the printed seed; no-configuration verbs broken by a bad EKR_* variable; an empty variable refused without its name.

Routing (coordinator): all six back to the implementor, together with the six friction points of blind agent trial 1 (`trial-1.md`, PASS in 24 invocations).

```findings
[
  {"file": "crates/ekr/src/cli/agent.rs", "line": 217, "category": "contract-drift", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "ekr operations documents DefineNodeType, DefineEdgeType, ModifyProperty and MergeEntity as writable while the kernel rejects every proposal of them as unsupported-operation."},
  {"file": "crates/ekr/src/cli/agent.rs", "line": 345, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The heading ids from ekr example ekr-seed/2 is false for 7 of 12 examples, and UpdateProperty and Invoke cannot validate against the printed seed at all."},
  {"file": "crates/ekr/src/cli/mod.rs", "line": 54, "category": "boundary", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "A malformed or empty EKR_* variable makes guide, operations, example and mint exit 2, although the guide says they need no configuration."},
  {"file": "crates/ekr/src/cli/mod.rs", "line": 61, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "An empty EKR_STORE/EKR_HOST/EKR_BACKEND is refused by clap with a message that names only the flag, never the variable."},
  {"file": "crates/ekr/tests/agent_cli.rs", "line": 519, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The per-verb help check is satisfied by the shared SEE footer, so removing any verb own input-format text stays green."},
  {"file": "crates/ekr/tests/agent_cli.rs", "line": 432, "category": "acceptance", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Acceptance 7 properties-by-name-and-id is never exercised because the seed declares no properties and the assertion only checks for an array."}
]
```
