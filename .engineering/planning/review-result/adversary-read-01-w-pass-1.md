---
format: aep.planning-md/3
id: review-result:adversary-read-01-w-pass-1
kind: review-result
status: active
title: 'Adversary pass 1 on unit W: the write path'
relations:
- reviews: task:write-verbs-cost-most-of-an-ingest
revision: 1
---
unit: W of wave read-01 (task:write-verbs-cost-most-of-an-ingest), commit 659a913f plus one untracked test file in ekr-read-w
verdict: NEEDS-CHANGE (acceptance not met, and one introduced cache-only defect); invariant 1 held under every attack
cases: executed 122→123, red 1
origin: introduced 2 / pre-existing 0 / undecided 0

## Case added

`crates/ekr-store/tests/adversary_write_path_pointer.rs`: handle A writes a verification pointer on a
store with no pointer (nothing written; A records "no pointer, length 0"), handle B writes a
checkpoint, A writes another verification pointer. Red on both providers: "the write after another
handle's pointer returned from this handle's record of an empty stream".

## What held (invariant 1)

A preparation from another handle or process is authorized at read-back; identical bytes cannot
carry another context (command key, predecessor event and hash, attempt, expected version, tenant
and stream coordinates are all checked); a stale preparation is refused by the provider's exact
version; crash and resume re-authorize in a new process; swapped bytes are refused
`preparation-address` before the memo; a successor attempt is refused `preparation-fingerprint` or
`preparation-native-metadata` with nothing appended; shared `Arc<ReplayState>` has no interior
mutability; `verify` reconstructs as `replay` does; a lost conditional pointer append re-reads.

```findings
- file: .engineering/planning/task/write-verbs-cost-most-of-an-ingest.md
  line: 25
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: acceptance requires each write verb under 100 ms on the 19 MB store; the commit's own measurements give validate 78-144 ms and commit 123-212 ms
- file: crates/ekr-store/src/eventlog.rs
  line: 1389
  category: concurrency
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the pointer memo is kept after a write that appended nothing, so a later verification pointer returns from a stale "empty stream" record and another handle's newer checkpoint is never pointed at, contrary to design 98.1(5)
```
