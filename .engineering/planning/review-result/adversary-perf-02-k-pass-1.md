---
format: aep.planning-md/3
id: review-result:adversary-perf-02-k-pass-1
kind: review-result
status: active
title: Adversary, wave perf-02 unit K, pass 1
relations:
- reviews: task:checkpoint-cadence-by-size
revision: 1
---
## Verdict

NEEDS-CHANGE: a session at rest can move the store checkpoint back behind one another process wrote.
Reopen after 0-12 commits on both providers and v1/v2/v3 matches a full replay; a forged `/3`
`created` list changes no answer and frees no id. Three new cases catch mutants M1-M3 that the
existing suite let through.

```findings
- file: crates/ekr-kernel/src/commands.rs
  line: 522
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a session at rest writes the checkpoint of its own stale head, so the store newer checkpoint from another process is replaced and its blob deleted"
- file: crates/ekr-kernel/src/checkpoint.rs
  line: 403
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "no existing test observes the early-basis condition of checkpoint_behind"
- file: crates/ekr-kernel/src/checkpoint.rs
  line: 394
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "no existing test observes the no-retained-checkpoint branch of checkpoint_behind"
- file: crates/ekr-kernel/src/checkpoint.rs
  line: 383
  category: boundary
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "no existing test commits exactly REPLAY_CHECKPOINT_BYTES, so changing >= to > survives"
- file: crates/ekr-kernel/tests/adversary_checkpoint_cadence_p1.rs
  line: 561
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the rewritten byte-bound assertion cannot fail because the mixed histories stay far under 16 MiB"
- file: crates/ekr/src/conformance.rs
  line: 1775
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the conformance adapter never emits the created field that kernel.yaml declares for CommitReceiptV1"
```
