---
format: aep.planning-md/3
id: review-result:adversary-sdk-03-p-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-03 unit P, pass 1
relations:
- reviews: task:commit-applies-once
- reviews: task:candidate-built-once-per-validation
revision: 1
---
## Verdict

CONFIRMED on `329b5e67`, no blocker: no stale or foreign graph reaches the publish check (the key
holds a weak reference to the prior graph's allocation plus the prior root, time, transaction hash,
validators and commit time); invariant 1 holds; refusal order and bytes are unchanged (the perf-01
differential digest). The adversary's cases in `93ecc046` cover every operation kind on profiles
v1–v3 and both providers, a commit overtaken by another writer, and a validation against an older
revision. Three notes are filed as `task:validate-builds-one-view-per-command`.

```findings
- file: crates/ekr-kernel/src/replay.rs
  line: 840
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: "a validate command runs the pipeline twice, deciding and in the admitting replay, so it builds two candidate views"
- file: crates/ekr-kernel/src/validate/candidate.rs
  line: 126
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the per-edge assertion index is rebuilt per validation by a full pass over every assertion"
- file: crates/ekr-kernel/src/apply.rs
  line: 77
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "no case observes the graph held in the thread-local, so removing its release keeps the suite green"
```
