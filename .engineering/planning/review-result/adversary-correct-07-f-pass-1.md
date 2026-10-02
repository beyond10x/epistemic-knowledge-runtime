---
format: aep.planning-md/3
id: review-result:adversary-correct-07-f-pass-1
kind: review-result
status: active
title: Adversary, correct-07 unit F, pass 1
relations:
- reviews: task:read-only-open-passes-under-load
- reviews: task:rendering-cost-test-passes-under-load
revision: 1
---
CONFIRMED

Adversary pass 1 on unit F (`impl/c7-tests-under-load` at `e33241b7`; cases committed as `4d0ae3bd`, two red and ignored). `cargo test -p ekr-store`: 166 passed, 4 ignored.

The unit's change holds: an absent or empty `-wal` read from the database alone misses no acknowledged commit (a 0-byte `-wal` holds no frame; any write changes its size and the read is taken again); the retry is bounded by a count; the hidden re-export flags nothing. Two pre-existing defects: a raced read-only open leaves a 0-byte `-wal` it created, contrary to the documented contract of writing nothing at the store's path; and a symlinked database path makes the open look for the `-wal` beside the link, so the read misses commits in the target's `-wal`. The counted rendering assertion sees provider reads only.

```findings
- file: crates/ekr-store/src/read_only.rs
  line: 372
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: "a read-only open raced by a closing writer leaves a 0-byte -wal it created, contrary to sqlite_read_only's documented contract of writing nothing at its path"
- file: crates/ekr-store/src/read_only.rs
  line: 74
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a symlinked database path makes the open look for the -wal beside the link, so the immutable read misses commits still in the target's -wal
- file: crates/ekr-views/tests/adversary_add_evidence_views.rs
  line: 148
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: the counted rendering assertion sees only provider reads, so a rendering made quadratic in CPU alone passes
```
