---
format: aep.planning-md/2
id: review-result:p1-15-eventlog-adversary-r1
kind: review-result
status: active
title: Adversary pass 1, wave p1-15 eventlog unit
relations:
- reviews: story:eventlog-0-4-batched-reads
revision: 1
---
Adversary pass 1 on unit p1-15-eventlog (wave p1-15), `aep-drive:adversary`, against `ada2b98`.

Owners: 3 findings, 3 implementor (introduced).

Verdict: NEEDS-CHANGE (1 blocker, 1 warning), CONFIRMED (1 note). cases: executed 89→93, red 2; the unit bench red on a quiet run.

Cases in `crates/ekr-store/tests/adversary_p1_15_batched_reads.rs` (4; 2 red: an absent object, and the first object in order, are refused with a provider blob-integrity error instead of `required-object-missing` on SQLite; 2 green: a redacted object event is refused alike on both paths; a write by another handle after the stamp is trusted is read).

Bench on a quiet run (fastest of 3): file propose 42 → 277 ms and 47 → 279 ms (5.9–6.6×); steps 47 → 161 → 279 ms are linear.

Routing (coordinator): F1 the ratio bound is replaced (decision recorded on the story): growth must be linear, not bounded against revision 1; F2 and F3 back to the implementor.

```findings
[
  {"file": "crates/ekr-kernel/tests/command_bench.rs", "line": 262, "category": "acceptance", "severity": "blocker", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The unit bench fails its own <=3x bound on a quiet run (file propose 42->277 ms and 47->279 ms); the reported pass rests on a load-inflated 221 ms at revision 1, and linear per-open verification makes the ratio bound unreachable."},
  {"file": "crates/ekr-store/src/eventlog.rs", "line": 325, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "The batched load reads every blob before any stream, so a provider blob-integrity error replaces the documented in-order required-object-missing refusal (tests adversary_p1_15_batched_reads.rs:178 and :215)."},
  {"file": "Cargo.lock", "line": 1194, "category": "contract-drift", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "The tempfile getrandom 0.4.3->0.3.4 move is not needed by the eventlog pin (reverted lock passes cargo metadata --locked) and breaks the eventlog-packages-only scope."}
]
```
