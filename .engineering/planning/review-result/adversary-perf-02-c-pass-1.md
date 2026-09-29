---
format: aep.planning-md/3
id: review-result:adversary-perf-02-c-pass-1
kind: review-result
status: active
title: Adversary, wave perf-02 unit C, pass 1
relations:
- reviews: story:commit-hashes-the-graph-once
revision: 1
---
## Verdict

No reachable wrong root. The pass wrote a case that kills the double mutant (remembered root
across stores on one thread; the existing recovery tests also kill it) and a 400-tree oracle for the
streamed encoding. One contract note fixed in `18328aa0` (with three more of its class); one
pre-existing note filed as `task:commit-applies-once`.

```findings
- file: crates/ekr-core/src/hash.rs
  line: 83
  category: contract-drift
  severity: note
  verdict: INFEASIBLE
  origin: introduced
  message: "ContentHash::of promises the digest over exactly canonical_bytes for every Canonical, but a hashing encoder's public as_bytes returns only the unhashed window, so an encode that reads it gets a different address; no in-tree implementation does"
- file: crates/ekr-kernel/src/commands.rs
  line: 574
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: "the story asks the verify step to reuse the apply result instead of applying again; only the root is reused, so each commit still clones and applies the whole graph twice"
```
