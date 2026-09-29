---
format: aep.planning-md/3
id: review-result:adversary-perf-02-m-pass-1
kind: review-result
status: active
title: Adversary, wave perf-02 unit M, pass 1
relations:
- reviews: story:session-keeps-only-the-head-graph
revision: 1
---
## Verdict

No correctness defect: validations against earlier revisions are decided on their own graphs (in a
session, after a checkpoint restore, after another handle commits, from a reader); past-revision
projections are byte-identical warm and cold on both providers; a long session holds at most 3
graphs. Two notes, both fixed in `96509131`.

```findings
- file: crates/ekr-kernel/src/read.rs
  line: 121
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the keeping hint outlives its read because the cached state still holds the requested revision graph until the handle next changes history, contrary to replay.rs:108-109 and :261"
- file: crates/ekr-views/src/lib.rs
  line: 188
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "load doc comment still says it reads the head first, while the code now reads schema_history first"
```
