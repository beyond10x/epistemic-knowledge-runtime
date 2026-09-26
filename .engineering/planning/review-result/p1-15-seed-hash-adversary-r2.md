---
format: aep.planning-md/2
id: review-result:p1-15-seed-hash-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-15 seed-hash unit
relations:
- reviews: story:seed-evidence-content-hash
revision: 1
---
Adversary pass 2 on unit p1-15-seed-hash (wave p1-15), `aep-drive:adversary`, against `a425907`.

Owners: 1 finding, 1 implementor (introduced).

Verdict: CONFIRMED (1 note). cases: executed 194→197, red 0.

Pass-1 check: the pass-1 finding is resolved (the guide describes both refusals as the kernel raises them).

Cases in `crates/ekr/tests/adversary_p1_15_seed_hash_2.rs` (3, green): pasted payload_yaml seeds every byte value, an empty and a 1 MiB + 1 payload on both providers; payload-missing names exactly the keys no entry names in four cases; relative store paths seed as the guide says.

Routing (coordinator): no-op; the finding is held by the adversary case the unit keeps.

```findings
[
  {"file": "crates/ekr-kernel/tests/seed.rs", "line": 592, "category": "mutant", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "Dropping the unnamed-key filter at seed.rs:96 so the reason lists every evidence_payloads key survives all three unit cases for seed-evidence-payload-missing and is caught only by adversary_p1_15_seed_hash_2::payload_missing_names_exactly_the_keys_no_entry_names."}
]
```
