---
format: aep.planning-md/1
id: review-result:p1-13-explain-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-13 explain unit
relations:
- reviews: story:seed-and-explain
revision: 1
---
Adversary pass 2 on unit p1-13-explain (wave p1-13), `aep-drive:adversary`, against the `bound()`
correction.

Owners: 3 findings, 3 implementor (introduced).

Verdict: INFEASIBLE. cases: executed 278→284, red 1.

Cases in `crates/ekr-kernel/tests/adversary_p1_13_explain_r2_bound.rs`, both providers: a
rewritten `GraphRoot` passes every check and is returned by snapshot (red); a root forged with its
coordinate, forged seed results, missing coordinates (green, each with its named code); a seed-only
lineage across a reopen and earlier revisions after a supersession are admitted (green). A
mutation probe of the 9 `bound()` checks: 7 killed by the unit's own suite, 2
(`root-record-disagrees`, `seed-record-disagrees`) killed only by this file.

```findings
- file: crates/ekr-kernel/src/explain.rs
  line: 297
  category: contract-drift
  severity: warning
  verdict: INFEASIBLE
  origin: introduced
  message: bound() leaves CanonicalGraph.root unbound, so a rewritten GraphRoot passes every check and snapshot returns it as the retained graph document.
- file: crates/ekr-kernel/tests/explain.rs
  line: 765
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the unit's suite and pass 1 stay green with bound()'s root-record-disagrees check (explain.rs:315) disabled.
- file: crates/ekr-kernel/tests/explain.rs
  line: 765
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the unit's suite and pass 1 stay green with bound()'s own seed-record-disagrees check (explain.rs:357) disabled.
```
