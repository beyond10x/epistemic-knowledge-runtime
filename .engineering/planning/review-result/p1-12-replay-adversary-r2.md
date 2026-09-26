---
format: aep.planning-md/2
id: review-result:p1-12-replay-adversary-r2
kind: review-result
status: active
title: Adversary pass 2, wave p1-12 replay unit
relations:
- reviews: task:replay-refusals-have-no-forged-history-case
- reviews: task:validation-hash-uses-payload-domain
revision: 1
---
Adversary pass 2 on unit p1-12-replay (wave p1-12), `aep-drive:adversary`, against the correction
that moved the named revision and knowledge-root checks before the generic result check.

Owners: 2 findings, 2 pre-existing (neither introduced by this unit).

Verdict: CONFIRMED (notes only). cases: executed 289→297, red 0.

Eight cases in `crates/ekr-kernel/tests/adversary_p1_12_replay_r2_named_order.rs`, all green on both
providers: every other root field lied consistently; `result_hash` alone lies; receipt-only
revision and knowledge-root lies; several lies at once; claims 0 and `u64::MAX`; a second commit
reclaiming revision 1; selected reads on forged lineages.

A mutant copy built into the unit's shared build directory served mutant binaries from 01:44 to
about 01:47; the adversary rebuilt from an unmutated copy. Results in that window are invalid.

```findings
- file: crates/ekr-kernel/src/replay.rs
  line: 424
  category: mutant
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: dropping the result_hash clause from the generic commit check left every pre-existing test target green; only the pass-2 result_hash-alone case catches it
- file: crates/ekr-store/src/lib.rs
  line: 147
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: the RevisionOutOfOrder message says the lineage "is at" the expected revision, while the field and the code mean the revision it was ready for, one past the head
```
