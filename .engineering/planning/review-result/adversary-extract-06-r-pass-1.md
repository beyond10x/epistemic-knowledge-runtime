---
format: aep.planning-md/3
id: review-result:adversary-extract-06-r-pass-1
kind: review-result
status: active
title: Adversary, extract-06 unit R, pass 1
relations:
- reviews: task:resolver-queue-drops-shared-alias-answers
revision: 1
---
CONFIRMED

Adversary pass 1 on unit R (`impl/resolver-queue-invalidates`, `af444db0` and `0e48c52d`; tests committed as `de19fd27`). No finding. `cargo test -p ekr-sdk`: 155 passed, 2 pre-existing ignored.

Cases: a fixed-seed walk (3 seeds × 100 steps × both providers) comparing every answer with the flushed store's and every `ekr resolve` request with an ideal cache; a node queued under two aliases dropping a key that shares only the second; a node on the recheck list leaving no stale answer.

Attacked and held: `reconcile`'s ProposeNew arm, the recheck list, two queued nodes sharing an alias (flushed first), the empty alias, partly overlapping keys, unneeded drops (counted exactly).

```findings
[]
```
