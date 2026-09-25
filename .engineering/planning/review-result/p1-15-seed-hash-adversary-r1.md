---
format: aep.planning-md/1
id: review-result:p1-15-seed-hash-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-15 seed-hash unit
relations:
- reviews: story:seed-evidence-content-hash
revision: 1
---
Adversary pass 1 on unit p1-15-seed-hash (wave p1-15), `aep-drive:adversary`, against `0e5ef9b`.

Owners: 1 finding, 1 implementor (introduced).

Verdict: CONFIRMED (1 warning). cases: executed 51→55, red 1.

Cases in `crates/ekr/tests/adversary_p1_15_seed_hash.rs` (4; 3 green: unreadable input exits 1; hash equals the kernel for non-UTF-8 and 2 MiB+1 payloads from file and stdin; the guide procedure seeds a non-UTF-8 and an empty payload on both providers; 1 red: an entry hash and payload key that disagree are refused as seed-evidence-payload-missing naming no hash, not as the guide says).

Blind agent trial 3 (`trial-3.md`) passed all 4 steps; its main friction was that no command produces the byte-list form `evidence_payloads` needs.

Routing (coordinator): the finding and the trial friction back to the implementor.

```findings
[
  {"file": "crates/ekr/src/cli/agent.rs", "line": 101, "category": "contract-drift", "severity": "warning", "verdict": "CONFIRMED", "origin": "introduced", "message": "The guide promises seed-evidence-payload-mismatch naming both hashes when a payload hash differs from its entry, but when the entry hash and the evidence_payloads key disagree the kernel refuses seed-evidence-payload-missing naming no hash (seed.rs:203-204), which is what an agent gets after correcting only one of the two places."}
]
```
