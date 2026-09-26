---
format: aep.planning-md/2
id: review-result:p4-01-agent-cli-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p4-01 agent-cli unit
relations:
- reviews: story:agent-discoverable-cli
revision: 1
---
Adversary pass 2 on unit agent-cli (wave p4-01), `aep-drive:adversary`, against `0551000`.

Owners: 6 findings, 5 implementor (introduced), 1 pre-existing.

Verdict: NEEDS-CHANGE (1 warning), INFEASIBLE (1 note), CONFIRMED (4 notes). cases: executed 146→151, red 4.

Pass-1 check: all six pass-1 findings resolved; trial friction 1-4 and 6 resolved; friction 5 carried as F1.

Cases in `crates/ekr/tests/adversary_p4_01_agent_cli_r2.rs` (5; 4 red: evidence payloads never printed; same-transaction add-and-retract commits; a self-accepted assertion is recorded by propose; --backend and EKR_BACKEND differ on case; 1 green: flag wins over its variable).

Routing (coordinator): F1-F4 to a last correction the coordinator verifies; F5 held by the adversary green case; F6 filed as a task (read verbs create a store directory at a mistyped path).

```findings
[
  {"file": "crates/ekr/src/cli/agent.rs", "line": 92, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The guide OUTPUT section promises the seed evidence payloads as base64, but no verb prints any payload, so evidence text cannot be read from a seeded store."},
  {"file": "crates/ekr/src/cli/agent.rs", "line": 66, "category": "contract-drift", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "The guide says only accepted assertions can be retracted, yet adding and retracting one in the same transaction validates and commits."},
  {"file": "crates/ekr/src/cli/agent.rs", "line": 64, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The guide calls a self-accepted assertion refused, but propose exits 0 and records it, and only validation rejects it."},
  {"file": "crates/ekr/src/cli/mod.rs", "line": 344, "category": "boundary", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "EKR_BACKEND is parsed ignoring case while --backend is case-sensitive."},
  {"file": "crates/ekr/src/cli/mod.rs", "line": 317, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "No existing case set a flag and its variable together until the added precedence case."},
  {"file": "crates/ekr/src/cli/mod.rs", "line": 377, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "A read verb given a mistyped store path creates an empty store directory there before reporting that the lineage has no seed."}
]
```
