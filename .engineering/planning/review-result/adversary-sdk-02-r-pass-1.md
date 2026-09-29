---
format: aep.planning-md/3
id: review-result:adversary-sdk-02-r-pass-1
kind: review-result
status: active
title: Adversary, wave sdk-02 unit R, pass 1
relations:
- reviews: story:sdk-resolve-and-batch
revision: 1
---
## Verdict

NEEDS-CHANGE on `4ff4928f`, six findings; the three defects and the batch-wide refusal fixed in
`a181d099`, the two mutants covered by the adversary's cases in `d2113ebc`. Held under attack:
foreign-commit detection between the SDK's commit and its next head check and mid-flush, cache keys
(alias order, repeats, the same alias on two types, `Ambiguous` uncached), bisection termination and
partition, the `Stale` bound, the caps, and the 8 MiB batch under the session line limit.

```findings
- file: crates/ekr-sdk/src/batch.rs
  line: 347
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a commit whose reply is lost may have landed, and the BatchError named neither it nor the in-flight id, so a consumer retrying the unlisted groups committed them twice"
- file: crates/ekr-sdk/src/resolve.rs
  line: 304
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "after a lost commit reply the next flush reported the queued id as replaced by itself"
- file: crates/ekr-sdk/src/resolve.rs
  line: 282
  category: concurrency
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "a flush that failed after an alias-already-exists rejection left the refused nodes neither queued nor replaced"
- file: crates/ekr-sdk/src/resolve.rs
  line: 302
  category: mutant
  severity: warning
  verdict: NEEDS-CHANGE
  origin: introduced
  message: "dropping the requeue on an unrelated foreign commit lost every queued node and the unit's suite stayed green"
- file: crates/ekr-sdk/src/batch.rs
  line: 363
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "STALE_RETRIES 8 to 1 left the unit's suite green"
- file: crates/ekr-sdk/src/batch.rs
  line: 279
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: "a whole-transaction refusal such as a wrong proposer was bisected, costing 2n-1 proposes"
```
