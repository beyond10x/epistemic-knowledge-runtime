---
format: aep.planning-md/3
id: review-result:adversary-correct-07-s-pass-1
kind: review-result
status: active
title: Adversary, correct-07 unit S, pass 1
relations:
- reviews: task:sqlite-store-replaced-in-place
- reviews: task:held-bytes-notice-deleted-blobs
revision: 1
---
NEEDS-CHANGE

Adversary pass 1 on unit S (`impl/store-replaced` at `a704f2af`; cases committed as `3468f1b3`, four red and ignored). `cargo test -p ekr-store`: 155 passed, 5 ignored; targeted kernel and CLI lanes green.

A header read with `immutable=1` taken while a checkpoint writes the database file comes back malformed while the file's stat holds still, and is refused as `store-replaced`: a reader beside a self-checkpointing writer had up to 1,358 of 11,049 reads refused (4 of 6 runs), with an external checkpointer 6 of 6 runs, and one write refused; opens beside a writer were refused too. Two handles on one store is supported. The withdrawal guard matches only `.redact(` and `.delete_blob(` text.

Held: one stat per call and no header read while the file is unchanged (strace); rename-over refused; after `cp` over the file every session read verb and propose follow the new database; stream-before-bytes order tested by the perf01 case; the file provider's divergence path; read-only opens unchanged.

```findings
- file: crates/ekr-store/src/replaced.rs
  line: 146
  category: concurrency
  severity: blocker
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a header read during a checkpoint fails as malformed while the stat holds still, and is refused as store-replaced on reads, writes and opens of a store that was never replaced"
- file: crates/ekr-store/tests/eventlog_object_memo.rs
  line: 348
  category: judgement
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "the withdrawal guard matches only .redact( and .delete_blob( text, so a qualified call or forget_tenant in a product source passes it"
```
