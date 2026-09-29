---
format: aep.planning-md/3
id: review-result:adversary-perf-01-r-pass-1
kind: review-result
status: active
title: Adversary, wave perf-01 unit R, pass 1
relations:
- reviews: story:reads-share-verified-state
revision: 1
---
## Verdict

NEEDS-CHANGE on acceptance only; no correctness defect in the sharing.

```
unit: story:reads-share-verified-state (unit R), ekr-p1-r at 749bbf8c plus one test file
cases: executed 412→419, red 1
origin: introduced 1 / pre-existing 0 / undecided 3
```

## Cases added (`crates/ekr-kernel/tests/adversary_perf01_r_shared_reads.rs`)

A capture answers what it answered when taken, through later validate, reject, commit and stale
decisions; `Arc::make_mut` on a capture leaves the kernel untouched; past revisions read the same
before and after later commits (views rule 5); a head restored from a checkpoint indexes its own
graph; the alias index matches byte for byte (case, NFC/NFD, Kelvin sign, whitespace); a reader
racing a committing handle sees one state per capture; and the red case: a seed payload deleted after
a read is still answered by the live handle while a fresh handle refuses (pre-existing held-bytes
behaviour; ignored with `task:held-bytes-notice-deleted-blobs`).

## Coordinator decisions

- Acceptance 1 (≤ 5 ms resolve CPU) is measured on the merged R+B tree: 94% of R's remaining cost is
  the store's object copy, which unit B removes.
- The alias index kept for every former head (`replay.rs:121`) and the evidence route's graph copy
  (`view.rs:1225`) go back to the implementor.

```findings
- file: .engineering/planning/story/reads-share-verified-state.md
  line: 50
  category: acceptance
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: undecided
  message: "resolve inside ekr session measures 56.4 ms (sqlite) and 18.1 ms (file) CPU median at 1x by the implementor's own figures, against an acceptance of at most 5 ms"
- file: crates/ekr-store/src/eventlog.rs
  line: 883
  category: concurrency
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: "a handle that already read keeps answering after a seed payload blob is deleted while a fresh handle refuses, so the payloads_held guard at read.rs:266 is never reached through either shipped store"
- file: crates/ekr-kernel/src/replay.rs
  line: 121
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "the per-revision OnceLock AliasIndex is cloned into every later state, so a live session keeps the index of every former head it resolved against, with no limit"
- file: crates/ekr/src/cli/view.rs
  line: 1225
  category: acceptance
  severity: note
  verdict: CONFIRMED
  origin: undecided
  message: "the ekr view evidence route reads the head through Runtime::snapshot, which copies the whole verified head graph on every request, and the counting test does not cover it"
```
