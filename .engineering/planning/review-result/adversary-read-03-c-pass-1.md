---
format: aep.planning-md/3
id: review-result:adversary-read-03-c-pass-1
kind: review-result
status: active
title: Adversary, wave read-03 unit C, pass 1
relations:
- reviews: story:changes-since-read
revision: 1
---
## Verdict

No red case. The unit's code held under every attack; two sentences of the specification were
wrong or incomplete. The adversary does not approve; the coordinator fixed both sentences in
`e6a3694a` and kept the six cases.

```
unit: story:changes-since-read, ekr-r3-c at fb763548 plus one untracked test file
verdict: CONFIRMED (spec wording only); no red case
cases: executed 67→73 (ekr-views, default features), red 0
origin: introduced 2 / pre-existing 0 / undecided 0
```

## Cases added

`crates/ekr-views/tests/changes_adversary.rs`, 6 cases, all green on both providers: provider byte
equality over 90 requests; since equal to, after, and past `at` and the head; cursors at the last
change, at the seed boundary and forged; a valid-time since across a supersession and a retraction;
limits 1, 2, 5, 6, 7 and 2000; no seed replay when the seed is not chosen.

## Attacked and not broken

Cursors; since against `at` and head; omitted `at` equals naming the head; valid-time straddling and
retractions; evidence ids on created nodes and edges including the seed revision; ordering ties;
limits 0, 1, 2000, 2001; file against SQLite; HTTP against MCP argument handling; commit timestamps
(the kernel refuses a commit time earlier than the head, `apply.rs:17`); views rule 5.

```findings
- file: systems/ekr/domains/views.yaml
  line: 74
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the summary says a since that is not exactly one of three inputs answers SinceMalformed, while the command block, the outcome and both hosts answer invalid-query / -32602
- file: systems/ekr/domains/views.yaml
  line: 66
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the claim that the answer depends only on the request once at is resolved is false for since_revision past the head, which is refused (naming the head) before a commit and answered empty after it
```
